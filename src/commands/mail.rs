use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::{MailAttachmentCommand, MailCommand};
use crate::mail::{
    download_attachment, get_message, list_messages, DownloadAttachmentOptions, GetMessageOptions,
    ListMessagesOptions, MessageSummary,
};

const DEFAULT_LIST_LIMIT: u32 = 10;
const SUMMARY_TABLE_HEADER: &str = "DATE\tFROM\tSUBJECT\tMESSAGE ID";

pub fn run<S: AccountStore>(
    cmd: MailCommand,
    client: &AuthClient<'_, S>,
    quiet: bool,
) -> Result<()> {
    match cmd {
        MailCommand::List { limit, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_list_to(
                client,
                limit,
                json,
                &mut std::io::stdout(),
                None,
            ))
        }
        MailCommand::Search { query, limit, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_search_to(
                client,
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
            runtime.block_on(run_read_to(
                client,
                message_id,
                &mut std::io::stdout(),
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
                runtime.block_on(run_attachment_download_to(
                    client,
                    message_id,
                    attachment_id,
                    output.map(PathBuf::from),
                    quiet,
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
    let summaries = list_messages(client, &options)
        .await
        .context(error_context)?;
    write_summaries(&summaries, json, out)
}

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

    if !quiet {
        eprintln!(
            "Downloaded {} bytes to {}",
            downloaded.bytes,
            downloaded.path.display()
        );
    }

    Ok(())
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

fn write_summaries(
    summaries: &[MessageSummary],
    json: bool,
    out: &mut impl Write,
) -> Result<()> {
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
