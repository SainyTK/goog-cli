use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::docs::suggestion::{
    select_suggestion_ids, suggestion_action_request_body, suggestion_threads, SuggestionAction,
    SuggestionSelection, SUGGESTION_THREAD_FIELDS,
};

use super::{
    batch_update_document_options, get_document_options, run_with_docs_unified_access,
    write_json_line, DocsAccessAttempt,
};

pub(crate) struct DocsUnifiedTarget<'a, S: AccountStore> {
    pub config: &'a Config,
    pub store: &'a S,
    pub account_override: Option<&'a str>,
    pub document_id: String,
    pub documents_url: Option<&'a str>,
    pub state_path: Option<&'a Path>,
}

pub(crate) async fn run_list_suggestions_unified_to<S: AccountStore>(
    target: DocsUnifiedTarget<'_, S>,
    open: bool,
    out: &mut impl Write,
) -> Result<()> {
    let document = get_suggestion_threads(&target).await?;
    write_json_line(
        out,
        &suggestion_threads(&document, open),
        "failed to serialize Docs suggestions",
    )
}

pub(crate) async fn run_suggestion_action_unified_to<S: AccountStore>(
    target: DocsUnifiedTarget<'_, S>,
    action: SuggestionAction,
    selection: SuggestionSelection,
    out: &mut impl Write,
) -> Result<()> {
    let document = get_suggestion_threads(&target).await?;
    let suggestion_ids = select_suggestion_ids(&document, &selection)?;
    if suggestion_ids.is_empty() {
        return write_json_line(
            out,
            &serde_json::json!({
                "documentId": target.document_id,
                "suggestionResponses": []
            }),
            "failed to serialize Docs suggestion response",
        );
    }

    let options = batch_update_document_options(
        target.document_id.clone(),
        suggestion_action_request_body(action, &suggestion_ids),
        target.documents_url,
    );
    let response = run_with_docs_unified_access(
        target.config,
        target.store,
        target.account_override,
        &resource_key("docs", &target.document_id),
        DocsAccessAttempt::BatchUpdate(&options),
        target.state_path,
    )
    .await
    .with_context(|| format!("failed to apply Google Docs {}", action.command_name()))?;
    write_json_line(
        out,
        &response,
        "failed to serialize Docs suggestion response",
    )
}

async fn get_suggestion_threads<S: AccountStore>(
    target: &DocsUnifiedTarget<'_, S>,
) -> Result<serde_json::Value> {
    let options = get_document_options(
        target.document_id.clone(),
        Some(SUGGESTION_THREAD_FIELDS.to_string()),
        true,
        target.documents_url,
    )
    .with_suggestion_threads();
    run_with_docs_unified_access(
        target.config,
        target.store,
        target.account_override,
        &resource_key("docs", &target.document_id),
        DocsAccessAttempt::Get(&options),
        target.state_path,
    )
    .await
    .context("failed to read Google Docs suggestions")
}
