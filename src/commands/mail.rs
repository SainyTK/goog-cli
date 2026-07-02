use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::{resolve_account, Config};
use crate::auth::state::{
    load_runtime_state, load_runtime_state_from_path, resource_key, save_runtime_state,
    save_runtime_state_to_path, RuntimeState,
};
use crate::cli::{MailAttachmentCommand, MailCommand};
use crate::mail::{
    download_attachment, get_message, list_messages, DownloadAttachmentOptions, GetMessageOptions,
    ListMessagesOptions, MailError, MessageSummary,
};

const DEFAULT_LIST_LIMIT: u32 = 10;
const SUMMARY_TABLE_HEADER: &str = "DATE\tFROM\tSUBJECT\tMESSAGE ID";

pub fn run<S: AccountStore>(
    cmd: MailCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    quiet: bool,
) -> Result<()> {
    match cmd {
        MailCommand::List { limit, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            runtime.block_on(run_list_to(
                &client,
                limit,
                json,
                &mut std::io::stdout(),
                None,
            ))
        }
        MailCommand::Search { query, limit, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            runtime.block_on(run_search_to(
                &client,
                query,
                limit,
                json,
                &mut std::io::stdout(),
                None,
            ))
        }
        MailCommand::Read { message_id } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_read_unified_to(
                config,
                store,
                account_override,
                message_id,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        MailCommand::Attachment { command } => match command {
            MailAttachmentCommand::Download {
                message_id,
                attachment_id,
                output,
            } => {
                let runtime =
                    tokio::runtime::Runtime::new().context("failed to start async runtime")?;
                runtime.block_on(run_attachment_download_unified_to(
                    config,
                    store,
                    account_override,
                    message_id,
                    attachment_id,
                    output.map(PathBuf::from),
                    quiet,
                    None,
                    None,
                ))
            }
        },
    }
}

pub(super) async fn run_list_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    limit: Option<u32>,
    json: bool,
    out: &mut impl Write,
    messages_url: Option<&str>,
) -> Result<()> {
    let options = list_options(limit, messages_url);
    run_summary_to(
        client,
        &options,
        json,
        out,
        "failed to list GoogleMail Messages",
    )
    .await
}

pub(super) async fn run_search_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    query: String,
    limit: Option<u32>,
    json: bool,
    out: &mut impl Write,
    messages_url: Option<&str>,
) -> Result<()> {
    let options = search_options(query, limit, messages_url);
    run_summary_to(
        client,
        &options,
        json,
        out,
        "failed to search GoogleMail Messages",
    )
    .await
}

async fn run_summary_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListMessagesOptions,
    json: bool,
    out: &mut impl Write,
    error_context: &'static str,
) -> Result<()> {
    let summaries = list_messages(client, options)
        .await
        .context(error_context)?;
    write_summaries(&summaries, json, out)
}

#[cfg(test)]
pub(super) async fn run_read_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    message_id: String,
    out: &mut impl Write,
    messages_url: Option<&str>,
) -> Result<()> {
    let options = get_message_options(message_id, messages_url);

    let message = get_message(client, &options)
        .await
        .context("failed to fetch GoogleMail Message")?;
    write_json_line(out, &message, "failed to serialize GoogleMail Message")
}

pub(super) async fn run_read_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    message_id: String,
    out: &mut impl Write,
    messages_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let options = get_message_options(message_id.clone(), messages_url);
    let message_resource_key = mail_message_resource_key(&message_id);

    let result = run_with_mail_unified_access(
        config,
        store,
        account_override,
        &message_resource_key,
        MailAccessAttempt::Read(&options),
        state_path,
    )
    .await
    .context("failed to fetch GoogleMail Message")?;
    let MailAccessResult::Message(message) = result else {
        unreachable!("read access returns a message")
    };

    write_json_line(out, &message, "failed to serialize GoogleMail Message")
}

#[cfg(test)]
pub(super) async fn run_attachment_download_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    message_id: String,
    attachment_id: String,
    output: Option<PathBuf>,
    quiet: bool,
    messages_url: Option<&str>,
) -> Result<()> {
    let options = attachment_download_options(message_id, attachment_id, output, messages_url);
    let downloaded = download_attachment(client, &options)
        .await
        .context("failed to download GoogleMail Attachment")?;

    write_download_notice(&downloaded, quiet);

    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_attachment_download_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    message_id: String,
    attachment_id: String,
    output: Option<PathBuf>,
    quiet: bool,
    messages_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let options =
        attachment_download_options(message_id.clone(), attachment_id, output, messages_url);
    let message_resource_key = mail_message_resource_key(&message_id);
    let result = run_with_mail_unified_access(
        config,
        store,
        account_override,
        &message_resource_key,
        MailAccessAttempt::DownloadAttachment(&options),
        state_path,
    )
    .await
    .context("failed to download GoogleMail Attachment")?;
    let MailAccessResult::Downloaded(downloaded) = result else {
        unreachable!("attachment download access returns a download result")
    };

    write_download_notice(&downloaded, quiet);

    Ok(())
}

fn mail_message_resource_key(message_id: &str) -> String {
    resource_key("mail", message_id)
}

fn write_download_notice(downloaded: &crate::mail::DownloadedAttachment, quiet: bool) {
    if quiet {
        return;
    }

    eprintln!(
        "Downloaded {} bytes to {}",
        downloaded.bytes,
        downloaded.path.display()
    );
}

enum MailAccessAttempt<'a> {
    Read(&'a GetMessageOptions),
    DownloadAttachment(&'a DownloadAttachmentOptions),
}

enum MailAccessResult {
    Message(serde_json::Value),
    Downloaded(crate::mail::DownloadedAttachment),
}

async fn run_with_mail_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    attempt: MailAccessAttempt<'_>,
    state_path: Option<&Path>,
) -> Result<MailAccessResult, MailError> {
    let mut state = load_mail_runtime_state(state_path)?;

    if account_override.is_some() {
        let account = resolve_account(config, account_override)
            .map_err(MailError::Auth)?
            .expect("explicit account resolution returns an account");
        return run_mail_access_as_account(
            config,
            store,
            &mut state,
            state_path,
            target_resource_key,
            &attempt,
            account,
        )
        .await;
    }

    let candidates = unified_access_candidates(config, &state, target_resource_key);
    let mut last_target_access_failure = None;

    for account in candidates {
        match run_mail_access_as_account(
            config,
            store,
            &mut state,
            state_path,
            target_resource_key,
            &attempt,
            account,
        )
        .await
        {
            Ok(result) => return Ok(result),
            Err(err) if is_target_access_failure(&err) => {
                last_target_access_failure = Some(err);
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_target_access_failure.unwrap_or(MailError::Auth(
        crate::auth::error::AuthError::ActiveAccountNotConfigured,
    )))
}

async fn run_mail_access_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    state: &mut RuntimeState,
    state_path: Option<&Path>,
    target_resource_key: &str,
    attempt: &MailAccessAttempt<'_>,
    account: String,
) -> Result<MailAccessResult, MailError> {
    let client =
        AuthClient::from_config(config.clone(), store, Some(&account)).map_err(MailError::Auth)?;
    let result = attempt_mail_access(&client, attempt).await?;
    state.set_resource_account(target_resource_key, account);
    save_mail_runtime_state(state, state_path)?;
    Ok(result)
}

async fn attempt_mail_access<S: AccountStore>(
    client: &AuthClient<'_, S>,
    attempt: &MailAccessAttempt<'_>,
) -> Result<MailAccessResult, MailError> {
    match attempt {
        MailAccessAttempt::Read(options) => get_message(client, options)
            .await
            .map(MailAccessResult::Message),
        MailAccessAttempt::DownloadAttachment(options) => download_attachment(client, options)
            .await
            .map(MailAccessResult::Downloaded),
    }
}

fn load_mail_runtime_state(state_path: Option<&Path>) -> Result<RuntimeState, MailError> {
    match state_path {
        Some(path) => load_runtime_state_from_path(path),
        None => load_runtime_state(),
    }
    .map_err(MailError::Auth)
}

fn save_mail_runtime_state(
    state: &RuntimeState,
    state_path: Option<&Path>,
) -> Result<(), MailError> {
    match state_path {
        Some(path) => save_runtime_state_to_path(state, path),
        None => save_runtime_state(state),
    }
    .map_err(MailError::Auth)
}

fn unified_access_candidates(
    config: &Config,
    state: &RuntimeState,
    target_resource_key: &str,
) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(mapped) = state.account_for_resource(target_resource_key) {
        push_if_configured(config, &mut candidates, mapped);
    }

    if let Some(active) = config.active_account() {
        push_if_configured(config, &mut candidates, active);
    }

    for account in &config.accounts {
        push_candidate(&mut candidates, account);
    }

    candidates
}

fn push_if_configured(config: &Config, candidates: &mut Vec<String>, account: &str) {
    if config
        .accounts
        .iter()
        .any(|configured| configured == account)
    {
        push_candidate(candidates, account);
    }
}

fn push_candidate(candidates: &mut Vec<String>, account: &str) {
    if !candidates.iter().any(|candidate| candidate == account) {
        candidates.push(account.to_string());
    }
}

fn is_target_access_failure(err: &MailError) -> bool {
    matches!(err, MailError::NotFound | MailError::PermissionDenied)
}

fn list_options(limit: Option<u32>, messages_url: Option<&str>) -> ListMessagesOptions {
    let mut options = ListMessagesOptions::inbox(limit.unwrap_or(DEFAULT_LIST_LIMIT));
    if let Some(messages_url) = messages_url {
        options = options.with_messages_url(messages_url);
    }
    options
}

fn search_options(
    query: String,
    limit: Option<u32>,
    messages_url: Option<&str>,
) -> ListMessagesOptions {
    let mut options = ListMessagesOptions::search(query, limit.unwrap_or(DEFAULT_LIST_LIMIT));
    if let Some(messages_url) = messages_url {
        options = options.with_messages_url(messages_url);
    }
    options
}

fn attachment_download_options(
    message_id: String,
    attachment_id: String,
    output: Option<PathBuf>,
    messages_url: Option<&str>,
) -> DownloadAttachmentOptions {
    let mut options = DownloadAttachmentOptions::new(message_id, attachment_id);
    if let Some(output) = output {
        options = options.with_output(output);
    }
    if let Some(messages_url) = messages_url {
        options = options.with_messages_url(messages_url);
    }
    options
}

fn get_message_options(message_id: String, messages_url: Option<&str>) -> GetMessageOptions {
    let mut options = GetMessageOptions::new(message_id);
    if let Some(messages_url) = messages_url {
        options = options.with_messages_url(messages_url);
    }
    options
}

fn write_summaries(summaries: &[MessageSummary], json: bool, out: &mut impl Write) -> Result<()> {
    if json {
        write_summary_ndjson(summaries, out)
    } else {
        write_summary_table(summaries, out)
    }
}

fn write_summary_ndjson(summaries: &[MessageSummary], out: &mut impl Write) -> Result<()> {
    for summary in summaries {
        serde_json::to_writer(&mut *out, summary)
            .context("failed to serialize GoogleMail Message Summary")?;
        writeln!(out).context("failed to write output")?;
    }
    Ok(())
}

fn write_summary_table(summaries: &[MessageSummary], out: &mut impl Write) -> Result<()> {
    writeln!(out, "{SUMMARY_TABLE_HEADER}").context("failed to write output")?;
    for summary in summaries {
        writeln!(
            out,
            "{}\t{}\t{}\t{}",
            summary.date, summary.from, summary.subject, summary.id
        )
        .context("failed to write output")?;
    }
    Ok(())
}

fn write_json_line(out: &mut impl Write, value: &serde_json::Value, context: &str) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context(context.to_string())?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}
