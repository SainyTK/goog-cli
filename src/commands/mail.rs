use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::cli::{MailAttachmentCommand, MailCommand, MailDraftCommand};
use crate::mail::{
    create_draft, decode_base64url, download_attachment, get_message, list_messages,
    parse_message_reference, resolve_message_reference, update_draft, CreateDraftOptions,
    DownloadAttachmentOptions, DraftAttachment, DraftMessage, GetMessageOptions,
    ListMessagesOptions, MailError, MessageReference, MessageSummary, UpdateDraftOptions,
};

const DEFAULT_LIST_LIMIT: u32 = 10;
const SUMMARY_TABLE_HEADER: &str = "DATE\tFROM\tSUBJECT\tMESSAGE ID";
const SEARCH_EMPTY_TABLE_MESSAGE: &str = "No matching messages found.";
const DRAFT_TABLE_HEADER: &str = "DRAFT ID\tMESSAGE ID\tTHREAD ID";

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
        MailCommand::Read { message_id, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_read_unified_to(
                config,
                store,
                account_override,
                message_id,
                json,
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
        MailCommand::Draft { command } => match command {
            MailDraftCommand::Create {
                to,
                cc,
                bcc,
                subject,
                body,
                body_file,
                attachment,
                json,
            } => {
                let runtime =
                    tokio::runtime::Runtime::new().context("failed to start async runtime")?;
                let client = AuthClient::from_config(config.clone(), store, account_override)?;
                let body = resolve_draft_body(body, body_file)?;
                let attachments = resolve_draft_attachments(attachment)?;
                runtime.block_on(run_draft_create_to(
                    &client,
                    CreateDraftInput {
                        to,
                        cc,
                        bcc,
                        subject,
                        body,
                        attachments,
                    },
                    json,
                    &mut std::io::stdout(),
                    None,
                ))
            }
            MailDraftCommand::Edit {
                draft_id,
                to,
                cc,
                bcc,
                subject,
                body,
                body_file,
                attachment,
                json,
            } => {
                let runtime =
                    tokio::runtime::Runtime::new().context("failed to start async runtime")?;
                let client = AuthClient::from_config(config.clone(), store, account_override)?;
                let body = resolve_draft_body(body, body_file)?;
                let attachments = resolve_draft_attachments(attachment)?;
                runtime.block_on(run_draft_edit_to(
                    &client,
                    draft_id,
                    CreateDraftInput {
                        to,
                        cc,
                        bcc,
                        subject,
                        body,
                        attachments,
                    },
                    json,
                    &mut std::io::stdout(),
                    None,
                ))
            }
        },
    }
}

#[derive(Debug, Clone)]
pub(super) struct CreateDraftInput {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub attachments: Vec<DraftAttachmentInput>,
}

#[derive(Debug, Clone)]
pub(super) struct DraftAttachmentInput {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
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
        None,
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
        Some(SEARCH_EMPTY_TABLE_MESSAGE),
    )
    .await
}

pub(super) async fn run_draft_create_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    input: CreateDraftInput,
    json: bool,
    out: &mut impl Write,
    drafts_url: Option<&str>,
) -> Result<()> {
    let options = create_draft_options(input, drafts_url);
    let draft = create_draft(client, &options)
        .await
        .context("failed to create GoogleMail Draft")?;
    write_draft(&draft, json, out)
}

pub(super) async fn run_draft_edit_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    draft_id: String,
    input: CreateDraftInput,
    json: bool,
    out: &mut impl Write,
    drafts_url: Option<&str>,
) -> Result<()> {
    let options = update_draft_options(draft_id, input, drafts_url);
    let draft = update_draft(client, &options)
        .await
        .context("failed to edit GoogleMail Draft")?;
    write_draft(&draft, json, out)
}

async fn run_summary_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListMessagesOptions,
    json: bool,
    out: &mut impl Write,
    error_context: &'static str,
    empty_result_message: Option<&'static str>,
) -> Result<()> {
    let summaries = list_messages(client, options)
        .await
        .context(error_context)?;
    write_summaries(&summaries, json, out, empty_result_message)
}

#[cfg(test)]
pub(super) async fn run_read_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    message_id: String,
    json: bool,
    out: &mut impl Write,
    messages_url: Option<&str>,
) -> Result<()> {
    let reference = parse_message_reference(&message_id);
    let message_id = resolve_message_reference(client, &reference, messages_url)
        .await
        .context("failed to resolve GoogleMail Message reference")?;
    let options = get_message_options(message_id, messages_url);

    let message = get_message(client, &options)
        .await
        .context("failed to fetch GoogleMail Message")?;
    write_message(&message, json, out)
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_read_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    message_id: String,
    json: bool,
    out: &mut impl Write,
    messages_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let reference = parse_message_reference(&message_id);
    let message_resource_key = mail_message_resource_key(&message_reference_key(&reference));

    let result = run_with_mail_unified_access(
        config,
        store,
        account_override,
        &message_resource_key,
        MailAccessAttempt::Read {
            reference,
            messages_url,
        },
        state_path,
    )
    .await
    .context("failed to fetch GoogleMail Message")?;
    let MailAccessResult::Message(message) = result else {
        unreachable!("read access returns a message")
    };

    write_message(&message, json, out)
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
    let reference = parse_message_reference(&message_id);
    let message_id = resolve_message_reference(client, &reference, messages_url)
        .await
        .context("failed to resolve GoogleMail Message reference")?;
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
    let reference = parse_message_reference(&message_id);
    let message_resource_key = mail_message_resource_key(&message_reference_key(&reference));
    let result = run_with_mail_unified_access(
        config,
        store,
        account_override,
        &message_resource_key,
        MailAccessAttempt::DownloadAttachment {
            reference,
            attachment_id,
            output,
            messages_url,
        },
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

fn message_reference_key(reference: &MessageReference) -> String {
    match reference {
        MessageReference::MessageId(message_id) => message_id.clone(),
        MessageReference::Thread {
            thread_id,
            preferred_label,
        } => format!(
            "thread:{thread_id}:{}",
            preferred_label.as_deref().unwrap_or_default()
        ),
    }
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
    Read {
        reference: MessageReference,
        messages_url: Option<&'a str>,
    },
    DownloadAttachment {
        reference: MessageReference,
        attachment_id: String,
        output: Option<PathBuf>,
        messages_url: Option<&'a str>,
    },
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
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, MailAccessResult, MailError> {
            Box::pin(run_mail_access_as_account(config, store, &attempt, account))
        },
        is_target_access_failure,
    )
    .await
}

async fn run_mail_access_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    attempt: &MailAccessAttempt<'_>,
    account: String,
) -> Result<MailAccessResult, MailError> {
    let client =
        AuthClient::from_config(config.clone(), store, Some(&account)).map_err(MailError::Auth)?;
    let result = attempt_mail_access(&client, attempt).await?;
    Ok(result)
}

async fn attempt_mail_access<S: AccountStore>(
    client: &AuthClient<'_, S>,
    attempt: &MailAccessAttempt<'_>,
) -> Result<MailAccessResult, MailError> {
    match attempt {
        MailAccessAttempt::Read {
            reference,
            messages_url,
        } => {
            let message_id = resolve_message_reference(client, reference, *messages_url).await?;
            let options = get_message_options(message_id, *messages_url);
            get_message(client, &options)
                .await
                .map(MailAccessResult::Message)
        }
        MailAccessAttempt::DownloadAttachment {
            reference,
            attachment_id,
            output,
            messages_url,
        } => {
            let message_id = resolve_message_reference(client, reference, *messages_url).await?;
            let options = attachment_download_options(
                message_id,
                attachment_id.clone(),
                output.clone(),
                *messages_url,
            );
            download_attachment(client, &options)
                .await
                .map(MailAccessResult::Downloaded)
        }
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

fn create_draft_options(input: CreateDraftInput, drafts_url: Option<&str>) -> CreateDraftOptions {
    let mut options =
        CreateDraftOptions::new(input.to, input.cc, input.bcc, input.subject, input.body)
            .with_attachments(
                input
                    .attachments
                    .into_iter()
                    .map(|attachment| DraftAttachment {
                        filename: attachment.filename,
                        content_type: attachment.content_type,
                        data: attachment.data,
                    })
                    .collect(),
            );
    if let Some(drafts_url) = drafts_url {
        options = options.with_drafts_url(drafts_url);
    }
    options
}

fn update_draft_options(
    draft_id: String,
    input: CreateDraftInput,
    drafts_url: Option<&str>,
) -> UpdateDraftOptions {
    let mut options = UpdateDraftOptions::new(
        draft_id,
        DraftMessage {
            to: input.to,
            cc: input.cc,
            bcc: input.bcc,
            subject: input.subject,
            body: input.body,
            attachments: input
                .attachments
                .into_iter()
                .map(|attachment| DraftAttachment {
                    filename: attachment.filename,
                    content_type: attachment.content_type,
                    data: attachment.data,
                })
                .collect(),
        },
    );
    if let Some(drafts_url) = drafts_url {
        options = options.with_drafts_url(drafts_url);
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

fn resolve_draft_body(body: Option<String>, body_file: Option<String>) -> Result<String> {
    match (body, body_file) {
        (Some(body), None) => Ok(body),
        (None, Some(path)) => std::fs::read_to_string(&path)
            .with_context(|| format!("failed to read GoogleMail Draft body file: {path}")),
        (None, None) => Ok(String::new()),
        (Some(_), Some(_)) => unreachable!("clap prevents --body and --body-file together"),
    }
}

pub(super) fn resolve_draft_attachments(paths: Vec<String>) -> Result<Vec<DraftAttachmentInput>> {
    paths
        .into_iter()
        .map(|path| {
            let data = std::fs::read(&path)
                .with_context(|| format!("failed to read GoogleMail Draft Attachment: {path}"))?;
            let path = PathBuf::from(path);
            let filename = path
                .file_name()
                .and_then(std::ffi::OsStr::to_str)
                .filter(|filename| !filename.is_empty())
                .context("GoogleMail Draft Attachment path has no filename")?
                .to_string();
            Ok(DraftAttachmentInput {
                content_type: content_type_for_path(&path),
                filename,
                data,
            })
        })
        .collect()
}

fn content_type_for_path(path: &Path) -> String {
    match path
        .extension()
        .and_then(std::ffi::OsStr::to_str)
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("csv") => "text/csv",
        Some("gif") => "image/gif",
        Some("htm") | Some("html") => "text/html",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("json") => "application/json",
        Some("md") => "text/markdown",
        Some("pdf") => "application/pdf",
        Some("png") => "image/png",
        Some("txt") => "text/plain",
        Some("webp") => "image/webp",
        Some("xml") => "application/xml",
        Some("zip") => "application/zip",
        _ => "application/octet-stream",
    }
    .to_string()
}

fn write_summaries(
    summaries: &[MessageSummary],
    json: bool,
    out: &mut impl Write,
    empty_result_message: Option<&str>,
) -> Result<()> {
    if json {
        return match (summaries.is_empty(), empty_result_message) {
            (true, Some(_)) => writeln!(out, "[]").context("failed to write output"),
            _ => write_summary_ndjson(summaries, out),
        };
    }

    match (summaries.is_empty(), empty_result_message) {
        (true, Some(message)) => writeln!(out, "{message}").context("failed to write output"),
        _ => write_summary_table(summaries, out),
    }
}

fn write_draft(draft: &serde_json::Value, json: bool, out: &mut impl Write) -> Result<()> {
    if json {
        return write_json_line(out, draft, "failed to serialize GoogleMail Draft");
    }

    writeln!(out, "{DRAFT_TABLE_HEADER}").context("failed to write output")?;
    writeln!(
        out,
        "{}\t{}\t{}",
        draft
            .get("id")
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
        draft
            .get("message")
            .and_then(|message| message.get("id"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or(""),
        draft
            .get("message")
            .and_then(|message| message.get("threadId"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("")
    )
    .context("failed to write output")
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

fn write_message(message: &serde_json::Value, json: bool, out: &mut impl Write) -> Result<()> {
    if json {
        write_json_line(out, message, "failed to serialize GoogleMail Message")
    } else {
        write_message_markdown(message, out)
    }
}

fn write_message_markdown(message: &serde_json::Value, out: &mut impl Write) -> Result<()> {
    let payload = message.get("payload");
    let headers: Vec<(&str, &str)> = payload
        .and_then(|payload| payload.get("headers"))
        .and_then(serde_json::Value::as_array)
        .map(|headers| {
            headers
                .iter()
                .filter_map(|header| {
                    let name = header.get("name")?.as_str()?;
                    let value = header.get("value")?.as_str()?;
                    Some((name, value))
                })
                .collect()
        })
        .unwrap_or_default();
    let header_value = |name: &str| -> Option<&str> {
        headers
            .iter()
            .find(|(header_name, _)| header_name.eq_ignore_ascii_case(name))
            .map(|(_, value)| *value)
    };

    writeln!(
        out,
        "# {}",
        header_value("Subject").unwrap_or("(no subject)")
    )
    .context("failed to write output")?;
    writeln!(out).context("failed to write output")?;
    for (label, name) in [
        ("From", "From"),
        ("To", "To"),
        ("Cc", "Cc"),
        ("Date", "Date"),
    ] {
        if let Some(value) = header_value(name) {
            writeln!(out, "**{label}:** {value}").context("failed to write output")?;
        }
    }
    writeln!(out).context("failed to write output")?;
    writeln!(out, "---").context("failed to write output")?;
    writeln!(out).context("failed to write output")?;

    let body = payload.map(extract_body_text).unwrap_or_default();
    writeln!(out, "{}", body.trim()).context("failed to write output")?;

    let mut attachments = Vec::new();
    if let Some(payload) = payload {
        collect_attachments(payload, &mut attachments);
    }
    let (inline, real): (Vec<_>, Vec<_>) = attachments.into_iter().partition(|a| a.inline);

    if !real.is_empty() {
        writeln!(out).context("failed to write output")?;
        writeln!(out, "---").context("failed to write output")?;
        writeln!(out).context("failed to write output")?;
        writeln!(out, "**Attachments:**").context("failed to write output")?;
        for attachment in &real {
            write_attachment_line(out, attachment)?;
        }
    }

    if !inline.is_empty() {
        writeln!(out).context("failed to write output")?;
        if real.is_empty() {
            writeln!(out, "---").context("failed to write output")?;
            writeln!(out).context("failed to write output")?;
        }
        writeln!(
            out,
            "**Inline images** (embedded in the message body, e.g. signature icons/logos — not separate files):"
        )
        .context("failed to write output")?;
        for attachment in &inline {
            write_attachment_line(out, attachment)?;
        }
    }

    Ok(())
}

fn write_attachment_line(out: &mut impl Write, attachment: &Attachment) -> Result<()> {
    writeln!(
        out,
        "- {} ({}, {} bytes) — attachment ID: `{}`",
        attachment.filename.as_deref().unwrap_or("(untitled)"),
        attachment.mime_type,
        attachment.size,
        attachment.attachment_id,
    )
    .context("failed to write output")
}

struct Attachment {
    filename: Option<String>,
    mime_type: String,
    size: u64,
    attachment_id: String,
    inline: bool,
}

fn collect_attachments(payload: &serde_json::Value, attachments: &mut Vec<Attachment>) {
    if let Some(attachment_id) = payload
        .get("body")
        .and_then(|body| body.get("attachmentId"))
        .and_then(serde_json::Value::as_str)
    {
        let filename = payload
            .get("filename")
            .and_then(serde_json::Value::as_str)
            .filter(|filename| !filename.is_empty())
            .map(str::to_string);
        let mime_type = payload
            .get("mimeType")
            .and_then(serde_json::Value::as_str)
            .unwrap_or("application/octet-stream")
            .to_string();
        let size = payload
            .get("body")
            .and_then(|body| body.get("size"))
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let inline = part_content_disposition(payload)
            .map(|disposition| disposition.eq_ignore_ascii_case("inline"))
            .unwrap_or(false);
        attachments.push(Attachment {
            filename,
            mime_type,
            size,
            attachment_id: attachment_id.to_string(),
            inline,
        });
    }

    if let Some(parts) = payload.get("parts").and_then(serde_json::Value::as_array) {
        for part in parts {
            collect_attachments(part, attachments);
        }
    }
}

fn part_content_disposition(payload: &serde_json::Value) -> Option<&str> {
    let headers = payload.get("headers")?.as_array()?;
    let value = headers.iter().find_map(|header| {
        let name = header.get("name")?.as_str()?;
        if name.eq_ignore_ascii_case("Content-Disposition") {
            header.get("value")?.as_str()
        } else {
            None
        }
    })?;
    Some(value.split(';').next().unwrap_or(value).trim())
}

fn extract_body_text(payload: &serde_json::Value) -> String {
    if let Some(html) = find_body_part(payload, "text/html") {
        return html_to_markdown(&html);
    }
    find_body_part(payload, "text/plain").unwrap_or_default()
}

fn find_body_part(payload: &serde_json::Value, mime_type: &str) -> Option<String> {
    let this_mime_type = payload
        .get("mimeType")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if this_mime_type == mime_type {
        if let Some(data) = payload
            .get("body")
            .and_then(|body| body.get("data"))
            .and_then(serde_json::Value::as_str)
        {
            if let Ok(bytes) = decode_base64url(data) {
                return Some(String::from_utf8_lossy(&bytes).into_owned());
            }
        }
    }

    payload
        .get("parts")
        .and_then(serde_json::Value::as_array)?
        .iter()
        .find_map(|part| find_body_part(part, mime_type))
}

const BUFFERED_TAGS: &[&str] = &["a", "b", "strong", "i", "em", "td", "th"];

#[derive(Default)]
struct TableCtx {
    rows: Vec<Vec<String>>,
    current_row: Vec<String>,
}

struct Frame {
    tag: String,
    href: Option<String>,
    buf: String,
}

#[derive(Default)]
struct HtmlToMarkdown {
    output: String,
    frames: Vec<Frame>,
    tables: Vec<TableCtx>,
    skip_depth: u32,
}

impl HtmlToMarkdown {
    fn write_str(&mut self, s: &str) {
        match self.frames.last_mut() {
            Some(frame) => frame.buf.push_str(s),
            None => self.output.push_str(s),
        }
    }

    fn handle_text(&mut self, raw: &str) {
        if self.skip_depth > 0 {
            return;
        }
        let collapsed = collapse_whitespace(&decode_html_entities(raw));
        if !collapsed.is_empty() {
            self.write_str(&collapsed);
        }
    }

    fn handle_tag(&mut self, raw: &str) {
        let raw = raw.trim();
        let closing = raw.starts_with('/');
        let self_closing = raw.ends_with('/');
        let body = raw.trim_start_matches('/').trim_end_matches('/').trim();
        let mut parts = body.splitn(2, |c: char| c.is_whitespace());
        let name = parts.next().unwrap_or("").to_ascii_lowercase();
        let attrs = parts.next().unwrap_or("");

        if name.is_empty() || name.starts_with('!') || name.starts_with('?') {
            return;
        }

        if self.skip_depth > 0 {
            if matches!(name.as_str(), "style" | "script" | "head") {
                if closing {
                    self.skip_depth = self.skip_depth.saturating_sub(1);
                } else if !self_closing {
                    self.skip_depth += 1;
                }
            }
            return;
        }

        if !closing && !self_closing && matches!(name.as_str(), "style" | "script" | "head") {
            self.skip_depth += 1;
            return;
        }

        if closing {
            self.handle_end(&name);
        } else {
            self.handle_start(&name, attrs);
            if self_closing {
                if name == "br" {
                    // already emitted by handle_start
                } else if BUFFERED_TAGS.contains(&name.as_str()) {
                    self.handle_end(&name);
                }
            }
        }
    }

    fn handle_start(&mut self, name: &str, attrs: &str) {
        match name {
            "br" => self.write_str("\n"),
            "hr" => self.write_str("\n\n---\n\n"),
            "li" => self.write_str("- "),
            "table" => self.tables.push(TableCtx::default()),
            "tr" => {
                if let Some(table) = self.tables.last_mut() {
                    table.current_row.clear();
                }
            }
            _ if BUFFERED_TAGS.contains(&name) => {
                let href = if name == "a" {
                    extract_attr(attrs, "href").map(|href| decode_html_entities(&href))
                } else {
                    None
                };
                self.frames.push(Frame {
                    tag: name.to_string(),
                    href,
                    buf: String::new(),
                });
            }
            _ => {}
        }
    }

    fn handle_end(&mut self, name: &str) {
        match name {
            "p" | "div" | "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.write_str("\n\n");
            }
            "li" | "ul" | "ol" => {
                self.write_str("\n");
            }
            "tr" => {
                if let Some(table) = self.tables.last_mut() {
                    let row = std::mem::take(&mut table.current_row);
                    table.rows.push(row);
                }
            }
            "table" => {
                if let Some(table) = self.tables.pop() {
                    let rendered = render_table(&table.rows);
                    self.write_str(&rendered);
                }
            }
            _ if BUFFERED_TAGS.contains(&name) => {
                let Some(frame) = self.frames.pop() else {
                    return;
                };
                if frame.tag != name {
                    // Mismatched close; put the frame back and give up gracefully.
                    let text = frame.buf.trim().to_string();
                    self.write_str(&text);
                    return;
                }
                let text = frame.buf.trim().to_string();
                if name == "td" || name == "th" {
                    if let Some(table) = self.tables.last_mut() {
                        table
                            .current_row
                            .push(text.replace('|', "\\|").replace('\n', " "));
                    }
                    return;
                }
                let formatted = match name {
                    "b" | "strong" if !text.is_empty() => format!("**{text}**"),
                    "i" | "em" if !text.is_empty() => format!("*{text}*"),
                    "a" => match &frame.href {
                        Some(href) if !text.is_empty() => format!("[{text}]({href})"),
                        Some(href) => href.clone(),
                        None => text,
                    },
                    _ => text,
                };
                self.write_str(&formatted);
            }
            _ => {}
        }
    }

    fn finish(mut self) -> String {
        while let Some(frame) = self.frames.pop() {
            self.output.push_str(frame.buf.trim());
        }
        while let Some(table) = self.tables.pop() {
            let rendered = render_table(&table.rows);
            self.output.push_str(&rendered);
        }
        collapse_blank_lines(&self.output).trim().to_string()
    }
}

fn html_to_markdown(html: &str) -> String {
    let mut converter = HtmlToMarkdown::default();
    let mut text_buf = String::new();
    let mut raw_tag = String::new();
    let mut in_tag = false;

    for ch in html.chars() {
        if ch == '<' {
            if !text_buf.is_empty() {
                converter.handle_text(&text_buf);
                text_buf.clear();
            }
            in_tag = true;
            raw_tag.clear();
            continue;
        }
        if ch == '>' && in_tag {
            in_tag = false;
            converter.handle_tag(&raw_tag);
            continue;
        }
        if in_tag {
            raw_tag.push(ch);
        } else {
            text_buf.push(ch);
        }
    }
    if !text_buf.is_empty() {
        converter.handle_text(&text_buf);
    }

    converter.finish()
}

fn render_table(rows: &[Vec<String>]) -> String {
    let col_count = rows.iter().map(Vec::len).max().unwrap_or(0);
    if col_count == 0 {
        return String::new();
    }

    let mut out = String::from("\n\n");
    for (row_index, row) in rows.iter().enumerate() {
        out.push('|');
        for column in 0..col_count {
            let cell = row.get(column).map(String::as_str).unwrap_or("");
            out.push(' ');
            out.push_str(cell);
            out.push_str(" |");
        }
        out.push('\n');
        if row_index == 0 {
            out.push('|');
            for _ in 0..col_count {
                out.push_str(" --- |");
            }
            out.push('\n');
        }
    }
    out.push('\n');
    out
}

fn extract_attr(attrs: &str, attr_name: &str) -> Option<String> {
    let lower = attrs.to_ascii_lowercase();
    let needle = format!("{attr_name}=");
    let pos = lower.find(&needle)?;
    let after = &attrs[pos + needle.len()..];
    let mut chars = after.chars();
    let quote = chars.next()?;
    if quote == '"' || quote == '\'' {
        let rest = &after[quote.len_utf8()..];
        let end = rest.find(quote)?;
        Some(rest[..end].to_string())
    } else {
        let end = after
            .find(|c: char| c.is_whitespace())
            .unwrap_or(after.len());
        Some(after[..end].to_string())
    }
}

fn collapse_whitespace(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_was_space = false;
    for ch in s.chars() {
        if ch.is_whitespace() {
            if !last_was_space {
                out.push(' ');
            }
            last_was_space = true;
        } else {
            out.push(ch);
            last_was_space = false;
        }
    }
    out
}

fn collapse_blank_lines(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut newline_run = 0;
    for ch in s.chars() {
        if ch == '\n' {
            newline_run += 1;
            if newline_run <= 2 {
                out.push('\n');
            }
        } else {
            newline_run = 0;
            out.push(ch);
        }
    }
    out
}

fn decode_html_entities(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp_pos) = rest.find('&') {
        out.push_str(&rest[..amp_pos]);
        let after_amp = &rest[amp_pos + 1..];
        let semicolon = after_amp
            .as_bytes()
            .iter()
            .take(12)
            .position(|byte| *byte == b';');
        if let Some(semicolon) = semicolon {
            let entity = &after_amp[..semicolon];
            if let Some(resolved) = resolve_entity(entity) {
                out.push_str(&resolved);
                rest = &after_amp[semicolon + 1..];
                continue;
            }
        }
        out.push('&');
        rest = after_amp;
    }
    out.push_str(rest);
    out
}

fn resolve_entity(entity: &str) -> Option<String> {
    if let Some(hex) = entity
        .strip_prefix("#x")
        .or_else(|| entity.strip_prefix("#X"))
    {
        return u32::from_str_radix(hex, 16)
            .ok()
            .and_then(char::from_u32)
            .map(String::from);
    }
    if let Some(dec) = entity.strip_prefix('#') {
        return dec
            .parse::<u32>()
            .ok()
            .and_then(char::from_u32)
            .map(String::from);
    }
    let resolved = match entity {
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "quot" => "\"",
        "apos" => "'",
        "nbsp" => "\u{a0}",
        "mdash" => "\u{2014}",
        "ndash" => "\u{2013}",
        "hellip" => "\u{2026}",
        "lsquo" => "\u{2018}",
        "rsquo" => "\u{2019}",
        "ldquo" => "\u{201c}",
        "rdquo" => "\u{201d}",
        "copy" => "\u{a9}",
        "reg" => "\u{ae}",
        "trade" => "\u{2122}",
        _ => return None,
    };
    Some(resolved.to_string())
}
