use std::io::Write;

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::DocsCommand;
use crate::docs::{get_document, GetDocumentOptions};

pub fn run<S: AccountStore>(cmd: DocsCommand, client: &AuthClient<'_, S>) -> Result<()> {
    match cmd {
        DocsCommand::Get { document_id } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_get_to(
                client,
                document_id,
                &mut std::io::stdout(),
                None,
            ))
        }
    }
}

pub(super) async fn run_get_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let mut options = GetDocumentOptions::new(document_id);
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
