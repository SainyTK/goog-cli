use std::io::{Read, Write};

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::DocsCommand;
use crate::docs::{
    batch_update_document, get_document, BatchUpdateDocumentOptions, GetDocumentOptions,
};

pub fn run<S: AccountStore>(cmd: DocsCommand, client: &AuthClient<'_, S>) -> Result<()> {
    match cmd {
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

pub(super) async fn run_get_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    fields: Option<String>,
    include_tabs_content: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let mut options = GetDocumentOptions::new(document_id)
        .with_include_tabs_content(include_tabs_content);
    if let Some(fields) = fields {
        options = options.with_fields(fields);
    }
    if let Some(documents_url) = documents_url {
        options = options.with_documents_url(documents_url);
    }

    let document = get_document(client, &options)
        .await
        .context("failed to fetch Google Docs Document")?;
    serde_json::to_writer(&mut *out, &document).context("failed to serialize Docs Document")?;
    writeln!(out).context("failed to write output")?;
    Ok(())
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
    let mut options = BatchUpdateDocumentOptions::new(document_id, request_body);
    if let Some(documents_url) = documents_url {
        options = options.with_documents_url(documents_url);
    }

    let response = batch_update_document(client, &options)
        .await
        .context("failed to apply Google Docs Batch Update")?;
    serde_json::to_writer(&mut *out, &response)
        .context("failed to serialize Docs Batch Update response")?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}

fn read_request_body(path_or_stdin: &str, input: &mut impl Read) -> Result<serde_json::Value> {
    let body = if path_or_stdin == "-" {
        let mut body = String::new();
        input
            .read_to_string(&mut body)
            .context("failed to read Google Docs Batch Update request body from stdin")?;
        body
    } else {
        std::fs::read_to_string(path_or_stdin).with_context(|| {
            format!("failed to read Google Docs Batch Update request body: {path_or_stdin}")
        })?
    };

    serde_json::from_str(&body).context("failed to parse Google Docs Batch Update request body")
}
