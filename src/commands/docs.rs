use std::io::{Read, Write};

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::DocsCommand;
use crate::docs::{
    batch_update_document, get_document, map::build_document_map, map::DocumentMap,
    BatchUpdateDocumentOptions, GetDocumentOptions,
};

pub fn run<S: AccountStore>(cmd: DocsCommand, client: &AuthClient<'_, S>) -> Result<()> {
    match cmd {
        DocsCommand::Map { document_id, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_map_to(
                client,
                document_id,
                json,
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::Get {
            document_id,
            fields,
            include_tabs_content,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_get_to(
                client,
                document_id,
                fields,
                include_tabs_content,
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::BatchUpdate {
            document_id,
            requests,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            let mut stdin = std::io::stdin();
            runtime.block_on(run_batch_update_to(
                client,
                document_id,
                requests,
                &mut stdin,
                &mut std::io::stdout(),
                None,
            ))
        }
    }
}

pub(super) async fn run_map_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let options = get_document_options(document_id, None, true, documents_url);

    let document = get_document(client, &options)
        .await
        .context("failed to fetch Google Docs Document")?;
    let document_map = build_document_map(&document);
    if json {
        write_json_line(out, &document_map, "failed to serialize Docs Document Map")
    } else {
        write_document_map_table(out, &document_map)
    }
}

pub(super) async fn run_get_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    fields: Option<String>,
    include_tabs_content: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let options = get_document_options(document_id, fields, include_tabs_content, documents_url);

    let document = get_document(client, &options)
        .await
        .context("failed to fetch Google Docs Document")?;
    write_json_line(out, &document, "failed to serialize Docs Document")
}

pub(super) async fn run_batch_update_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    requests: String,
    input: &mut impl Read,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let request_body = read_request_body(&requests, input)?;
    let options = batch_update_document_options(document_id, request_body, documents_url);

    let response = batch_update_document(client, &options)
        .await
        .context("failed to apply Google Docs Batch Update")?;
    write_json_line(
        out,
        &response,
        "failed to serialize Docs Batch Update response",
    )
}

fn read_request_body(path_or_stdin: &str, input: &mut impl Read) -> Result<serde_json::Value> {
    let (body, request_source) = if path_or_stdin == "-" {
        let mut body = String::new();
        input
            .read_to_string(&mut body)
            .context("failed to read Google Docs Batch Update request body from stdin")?;
        (body, "stdin".to_string())
    } else {
        let body = std::fs::read_to_string(path_or_stdin).with_context(|| {
            format!("failed to read Google Docs Batch Update request body: {path_or_stdin}")
        })?;
        (body, path_or_stdin.to_string())
    };

    serde_json::from_str(&body).with_context(|| {
        format!("failed to parse Google Docs Batch Update request body from {request_source}")
    })
}

fn get_document_options(
    document_id: String,
    fields: Option<String>,
    include_tabs_content: bool,
    documents_url: Option<&str>,
) -> GetDocumentOptions {
    let mut options =
        GetDocumentOptions::new(document_id).with_include_tabs_content(include_tabs_content);
    if let Some(fields) = fields {
        options = options.with_fields(fields);
    }
    if let Some(documents_url) = documents_url {
        options = options.with_documents_url(documents_url);
    }
    options
}

fn batch_update_document_options(
    document_id: String,
    request_body: serde_json::Value,
    documents_url: Option<&str>,
) -> BatchUpdateDocumentOptions {
    let mut options = BatchUpdateDocumentOptions::new(document_id, request_body);
    if let Some(documents_url) = documents_url {
        options = options.with_documents_url(documents_url);
    }
    options
}

fn write_document_map_table(out: &mut impl Write, document_map: &DocumentMap) -> Result<()> {
    writeln!(
        out,
        "{:<5} {:<7} {:<5} {:<4} {:<20} {:<18} {:<15} Preview",
        "Entry", "Index", "Page", "Line", "Kind", "Style", "Confidence"
    )
    .context("failed to write Docs Document Map header")?;

    for entry in &document_map.entries {
        let index = entry
            .location
            .index
            .map(|index| index.to_string())
            .unwrap_or_else(|| "-".into());
        let page = entry
            .location
            .page
            .map(|page| page.to_string())
            .unwrap_or_else(|| "-".into());
        let style = entry.style.as_deref().unwrap_or("-");
        writeln!(
            out,
            "{:<5} {:<7} {:<5} {:<4} {:<20} {:<18} {:<15} {}",
            entry.entry,
            index,
            page,
            entry.location.content_line,
            format!("{:?}", entry.kind),
            style,
            format!("{:?}", entry.location.confidence),
            entry.preview
        )
        .context("failed to write Docs Document Map row")?;
    }

    Ok(())
}

fn write_json_line<T: serde::Serialize>(
    out: &mut impl Write,
    value: &T,
    context: &str,
) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context(context.to_string())?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}
