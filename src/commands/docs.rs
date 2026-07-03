use std::io::{Read, Write};
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::{resolve_account, Config};
use crate::auth::state::resource_key;
use crate::auth::unified_access::UnifiedAccess;
use crate::cli::{DocsCommand, DocsListType};
use crate::docs::{
    batch_update_document, get_document, map::build_document_map, map::search_document_text,
    map::DocumentLocation, map::DocumentMap, map::DocumentMapEntry, map::DocumentMapEntryKind,
    map::DocumentRange, map::DocumentTextBlock, BatchUpdateDocumentOptions, DocsError,
    GetDocumentOptions,
};

pub fn run<S: AccountStore>(
    cmd: DocsCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
) -> Result<()> {
    match cmd {
        DocsCommand::Map { document_id, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_map_unified_to(
                config,
                store,
                account_override,
                document_id,
                json,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::SearchText {
            document_id,
            text,
            json,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_search_text_unified_to(
                config,
                store,
                account_override,
                document_id,
                text,
                json,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::GetContent {
            document_id,
            index,
            entry,
            page,
            line,
            heading,
            json,
        } => {
            let selector = content_selector(index, entry, page, line, heading)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_get_content_unified_to(
                config,
                store,
                account_override,
                document_id,
                selector,
                json,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::InsertText {
            document_id,
            text,
            index,
            entry,
            page,
            line,
            after_heading,
            before_heading,
            after_text,
            before_text,
            dry_run,
            json,
            required_revision_id,
        } => {
            let selector = insert_text_selector(
                index,
                entry,
                page,
                line,
                after_heading,
                before_heading,
                after_text,
                before_text,
            )?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_insert_text_unified_to(
                config,
                store,
                account_override,
                InsertTextCommand {
                    document_id,
                    text,
                    selector,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::ReplaceText {
            document_id,
            old_text,
            new_text,
            match_number,
            all,
            dry_run,
            json,
            required_revision_id,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_replace_text_unified_to(
                config,
                store,
                account_override,
                ReplaceTextCommand {
                    document_id,
                    old_text,
                    new_text,
                    match_number,
                    all,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::ListImages { document_id, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_list_images_unified_to(
                config,
                store,
                account_override,
                document_id,
                json,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::ListTables { document_id, json } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_list_tables_unified_to(
                config,
                store,
                account_override,
                document_id,
                json,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::InsertImage {
            document_id,
            image_uri,
            index,
            entry,
            page,
            line,
            after_heading,
            before_heading,
            after_text,
            before_text,
            dry_run,
            json,
            required_revision_id,
        } => {
            let selector = insert_text_selector(
                index,
                entry,
                page,
                line,
                after_heading,
                before_heading,
                after_text,
                before_text,
            )?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_insert_image_unified_to(
                config,
                store,
                account_override,
                InsertImageCommand {
                    document_id,
                    image_uri,
                    selector,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::InsertTable {
            document_id,
            data,
            rows,
            columns,
            index,
            entry,
            page,
            line,
            after_heading,
            before_heading,
            after_text,
            before_text,
            dry_run,
            json,
            required_revision_id,
        } => {
            let selector = insert_text_selector(
                index,
                entry,
                page,
                line,
                after_heading,
                before_heading,
                after_text,
                before_text,
            )?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_insert_table_unified_to(
                config,
                store,
                account_override,
                InsertTableCommand {
                    document_id,
                    data,
                    rows,
                    columns,
                    selector,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::EditTable {
            document_id,
            table_id,
            data,
            resize,
            dry_run,
            json,
            required_revision_id,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_edit_table_unified_to(
                config,
                store,
                account_override,
                EditTableCommand {
                    document_id,
                    table_id,
                    data,
                    resize,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::ApplyStyles {
            document_id,
            from_index,
            to_index,
            entry,
            page,
            line,
            text,
            match_number,
            bold,
            italic,
            font_size,
            foreground_color,
            heading,
            dry_run,
            json,
            required_revision_id,
        } => {
            let selector =
                range_selector(from_index, to_index, entry, page, line, text, match_number)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_apply_styles_unified_to(
                config,
                store,
                account_override,
                ApplyStylesCommand {
                    document_id,
                    selector,
                    bold,
                    italic,
                    font_size,
                    foreground_color,
                    heading,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::ApplyList {
            document_id,
            from_index,
            to_index,
            entry,
            page,
            line,
            list_type,
            preset,
            dry_run,
            json,
            required_revision_id,
        } => {
            let selector = range_selector(from_index, to_index, entry, page, line, None, None)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_apply_list_unified_to(
                config,
                store,
                account_override,
                ApplyListCommand {
                    document_id,
                    selector,
                    list_type,
                    preset,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
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
            runtime.block_on(run_get_unified_to(
                config,
                store,
                account_override,
                document_id,
                fields,
                include_tabs_content,
                &mut std::io::stdout(),
                None,
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
            runtime.block_on(run_batch_update_unified_to(
                config,
                store,
                account_override,
                document_id,
                requests,
                &mut stdin,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
    }
}

#[cfg(test)]
pub(super) async fn run_map_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    write_document_map(out, &document_map, json)
}

pub(super) async fn run_map_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        document_id,
        documents_url,
        state_path,
    )
    .await?;
    write_document_map(out, &document_map, json)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum ContentSelector {
    Index(i64),
    Entry(usize),
    PageLine { page: usize, line: usize },
    Heading(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum InsertTextSelector {
    Index(i64),
    Entry(usize),
    PageLine { page: usize, line: usize },
    AfterHeading(String),
    BeforeHeading(String),
    AfterText(String),
    BeforeText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InsertTextCommand {
    pub document_id: String,
    pub text: String,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReplaceTextCommand {
    pub document_id: String,
    pub old_text: String,
    pub new_text: String,
    pub match_number: Option<usize>,
    pub all: bool,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InsertImageCommand {
    pub document_id: String,
    pub image_uri: String,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct InsertTableCommand {
    pub document_id: String,
    pub data: Option<String>,
    pub rows: Option<usize>,
    pub columns: Option<usize>,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EditTableCommand {
    pub document_id: String,
    pub table_id: String,
    pub data: String,
    pub resize: bool,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ApplyStylesCommand {
    pub document_id: String,
    pub selector: RangeSelector,
    pub bold: bool,
    pub italic: bool,
    pub font_size: Option<f64>,
    pub foreground_color: Option<String>,
    pub heading: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ApplyListCommand {
    pub document_id: String,
    pub selector: RangeSelector,
    pub list_type: Option<DocsListType>,
    pub preset: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum RangeSelector {
    IndexRange {
        start_index: i64,
        end_index: i64,
    },
    Entry(usize),
    PageLine {
        page: usize,
        line: usize,
    },
    Text {
        text: String,
        match_number: Option<usize>,
    },
}

#[cfg(test)]
pub(super) async fn run_search_text_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    text: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    let ranges = search_document_text(&document_map, &text);
    write_search_text_results(out, &ranges, json)
}

pub(super) async fn run_search_text_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    text: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        document_id,
        documents_url,
        state_path,
    )
    .await?;
    let ranges = search_document_text(&document_map, &text);
    write_search_text_results(out, &ranges, json)
}

#[cfg(test)]
pub(super) async fn run_get_content_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    selector: ContentSelector,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    let entry = resolve_content_entry(&document_map, &selector)?;
    write_content_entry(out, &document_map, entry, json)
}

pub(super) async fn run_get_content_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    selector: ContentSelector,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        document_id,
        documents_url,
        state_path,
    )
    .await?;
    let entry = resolve_content_entry(&document_map, &selector)?;
    write_content_entry(out, &document_map, entry, json)
}

#[cfg(test)]
pub(super) async fn run_insert_text_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: InsertTextCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_insert_text_change(&document_map, &command)?;

    if command.dry_run {
        write_insert_text_dry_run(out, &change, command.json)
    } else {
        let request_body = change.request_body;
        let options =
            batch_update_document_options(command.document_id, request_body, documents_url);
        let response = batch_update_document(client, &options)
            .await
            .context("failed to apply Google Docs insert-text")?;
        write_json_line(
            out,
            &response,
            "failed to serialize Docs insert-text response",
        )
    }
}

pub(super) async fn run_insert_text_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: InsertTextCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_insert_text_change(&document_map, &command)?;

    if command.dry_run {
        write_insert_text_dry_run(out, &change, command.json)
    } else {
        let request_body = change.request_body;
        let options =
            batch_update_document_options(command.document_id.clone(), request_body, documents_url);
        let resource_key = resource_key("docs", &command.document_id);
        let response = run_with_docs_unified_access(
            config,
            store,
            account_override,
            &resource_key,
            DocsAccessAttempt::BatchUpdate(&options),
            state_path,
        )
        .await
        .context("failed to apply Google Docs insert-text")?;
        write_json_line(
            out,
            &response,
            "failed to serialize Docs insert-text response",
        )
    }
}

#[cfg(test)]
pub(super) async fn run_replace_text_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: ReplaceTextCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_replace_text_change(&document_map, &command)?;

    if command.dry_run {
        write_replace_text_dry_run(out, &change, command.json)
    } else {
        let request_body = change.request_body;
        let options =
            batch_update_document_options(command.document_id, request_body, documents_url);
        let response = batch_update_document(client, &options)
            .await
            .context("failed to apply Google Docs replace-text")?;
        write_json_line(
            out,
            &response,
            "failed to serialize Docs replace-text response",
        )
    }
}

pub(super) async fn run_replace_text_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: ReplaceTextCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_replace_text_change(&document_map, &command)?;

    if command.dry_run {
        write_replace_text_dry_run(out, &change, command.json)
    } else {
        let request_body = change.request_body;
        let options =
            batch_update_document_options(command.document_id.clone(), request_body, documents_url);
        let resource_key = resource_key("docs", &command.document_id);
        let response = run_with_docs_unified_access(
            config,
            store,
            account_override,
            &resource_key,
            DocsAccessAttempt::BatchUpdate(&options),
            state_path,
        )
        .await
        .context("failed to apply Google Docs replace-text")?;
        write_json_line(
            out,
            &response,
            "failed to serialize Docs replace-text response",
        )
    }
}

#[cfg(test)]
pub(super) async fn run_list_images_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    write_filtered_entries(
        out,
        &document_map,
        &[
            DocumentMapEntryKind::InlineImage,
            DocumentMapEntryKind::PositionedImage,
        ],
        json,
    )
}

pub(super) async fn run_list_images_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        document_id,
        documents_url,
        state_path,
    )
    .await?;
    write_filtered_entries(
        out,
        &document_map,
        &[
            DocumentMapEntryKind::InlineImage,
            DocumentMapEntryKind::PositionedImage,
        ],
        json,
    )
}

#[cfg(test)]
pub(super) async fn run_list_tables_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    write_filtered_entries(out, &document_map, &[DocumentMapEntryKind::Table], json)
}

pub(super) async fn run_list_tables_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        document_id,
        documents_url,
        state_path,
    )
    .await?;
    write_filtered_entries(out, &document_map, &[DocumentMapEntryKind::Table], json)
}

#[cfg(test)]
pub(super) async fn run_insert_image_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: InsertImageCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_insert_image_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        "insert-image",
    )
    .await
}

pub(super) async fn run_insert_image_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: InsertImageCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_insert_image_change(&document_map, &command)?;
    apply_or_preview_docs_change_unified(
        config,
        store,
        account_override,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        state_path,
        "insert-image",
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_insert_table_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: InsertTableCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_insert_table_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        "insert-table",
    )
    .await
}

pub(super) async fn run_insert_table_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: InsertTableCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_insert_table_change(&document_map, &command)?;
    apply_or_preview_docs_change_unified(
        config,
        store,
        account_override,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        state_path,
        "insert-table",
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_edit_table_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: EditTableCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_edit_table_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        "edit-table",
    )
    .await
}

pub(super) async fn run_edit_table_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: EditTableCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_edit_table_change(&document_map, &command)?;
    apply_or_preview_docs_change_unified(
        config,
        store,
        account_override,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        state_path,
        "edit-table",
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_apply_styles_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: ApplyStylesCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_apply_styles_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        "apply-styles",
    )
    .await
}

pub(super) async fn run_apply_styles_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: ApplyStylesCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_apply_styles_change(&document_map, &command)?;
    apply_or_preview_docs_change_unified(
        config,
        store,
        account_override,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        state_path,
        "apply-styles",
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_apply_list_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: ApplyListCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_apply_list_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        "apply-list",
    )
    .await
}

pub(super) async fn run_apply_list_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: ApplyListCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_apply_list_change(&document_map, &command)?;
    apply_or_preview_docs_change_unified(
        config,
        store,
        account_override,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        state_path,
        "apply-list",
    )
    .await
}

#[cfg(test)]
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

pub(super) async fn run_get_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    fields: Option<String>,
    include_tabs_content: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let options = get_document_options(
        document_id.clone(),
        fields,
        include_tabs_content,
        documents_url,
    );
    let resource_key = resource_key("docs", &document_id);
    let document = run_with_docs_unified_access(
        config,
        store,
        account_override,
        &resource_key,
        DocsAccessAttempt::Get(&options),
        state_path,
    )
    .await
    .context("failed to fetch Google Docs Document")?;

    write_json_line(out, &document, "failed to serialize Docs Document")
}

#[cfg(test)]
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

pub(super) async fn run_batch_update_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    requests: String,
    input: &mut impl Read,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let request_body = read_request_body(&requests, input)?;
    let options = batch_update_document_options(document_id.clone(), request_body, documents_url);
    let resource_key = resource_key("docs", &document_id);
    let response = run_with_docs_unified_access(
        config,
        store,
        account_override,
        &resource_key,
        DocsAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .context("failed to apply Google Docs Batch Update")?;

    write_json_line(
        out,
        &response,
        "failed to serialize Docs Batch Update response",
    )
}

enum DocsAccessAttempt<'a> {
    Get(&'a GetDocumentOptions),
    BatchUpdate(&'a BatchUpdateDocumentOptions),
}

async fn run_with_docs_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    attempt: DocsAccessAttempt<'_>,
    state_path: Option<&Path>,
) -> Result<serde_json::Value, DocsError> {
    let mut access = UnifiedAccess::load(target_resource_key, state_path)?;

    if account_override.is_some() {
        let account = resolve_account(config, account_override)?
            .expect("explicit account resolution returns an account");
        return run_docs_access_as_account(config, store, &mut access, &attempt, account).await;
    }

    let candidates = access.candidates(config);
    let mut last_target_access_failure = None;

    for account in candidates {
        match run_docs_access_as_account(config, store, &mut access, &attempt, account).await {
            Ok(result) => return Ok(result),
            Err(err) if is_target_access_failure(&err) => {
                last_target_access_failure = Some(err);
            }
            Err(err) => return Err(err),
        }
    }

    Err(last_target_access_failure.unwrap_or(DocsError::Auth(
        crate::auth::error::AuthError::ActiveAccountNotConfigured,
    )))
}

async fn run_docs_access_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    access: &mut UnifiedAccess,
    attempt: &DocsAccessAttempt<'_>,
    account: String,
) -> Result<serde_json::Value, DocsError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))?;
    let result = attempt_docs_access(&client, attempt).await?;
    access.record_success(account)?;
    Ok(result)
}

async fn attempt_docs_access<S: AccountStore>(
    client: &AuthClient<'_, S>,
    attempt: &DocsAccessAttempt<'_>,
) -> Result<serde_json::Value, DocsError> {
    match attempt {
        DocsAccessAttempt::Get(options) => get_document(client, options).await,
        DocsAccessAttempt::BatchUpdate(options) => batch_update_document(client, options).await,
    }
}

fn is_target_access_failure(err: &DocsError) -> bool {
    matches!(err, DocsError::NotFound | DocsError::PermissionDenied)
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

#[cfg(test)]
async fn get_document_map<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    documents_url: Option<&str>,
) -> Result<DocumentMap> {
    let options = get_document_options(document_id, None, true, documents_url);
    let document = get_document(client, &options)
        .await
        .context("failed to fetch Google Docs Document")?;
    Ok(build_document_map(&document))
}

async fn get_document_map_unified<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<DocumentMap> {
    let options = get_document_options(document_id.clone(), None, true, documents_url);
    let resource_key = resource_key("docs", &document_id);
    let document = run_with_docs_unified_access(
        config,
        store,
        account_override,
        &resource_key,
        DocsAccessAttempt::Get(&options),
        state_path,
    )
    .await
    .context("failed to fetch Google Docs Document")?;
    Ok(build_document_map(&document))
}

pub(super) fn content_selector(
    index: Option<i64>,
    entry: Option<usize>,
    page: Option<usize>,
    line: Option<usize>,
    heading: Option<String>,
) -> Result<ContentSelector> {
    let selector_count = usize::from(index.is_some())
        + usize::from(entry.is_some())
        + usize::from(page.is_some() || line.is_some())
        + usize::from(heading.is_some());
    if selector_count != 1 {
        bail!(
            "provide exactly one content selector: --index, --entry, --page with --line, or --heading"
        );
    }

    if let Some(index) = index {
        return Ok(ContentSelector::Index(index));
    }
    if let Some(entry) = entry {
        return Ok(ContentSelector::Entry(entry));
    }
    if page.is_some() || line.is_some() {
        let Some(page) = page else {
            bail!("--page and --line must be provided together");
        };
        let Some(line) = line else {
            bail!("--page and --line must be provided together");
        };
        return Ok(ContentSelector::PageLine { page, line });
    }
    if let Some(heading) = heading {
        return Ok(ContentSelector::Heading(heading));
    }

    unreachable!("selector count checked above")
}

fn insert_text_selector(
    index: Option<i64>,
    entry: Option<usize>,
    page: Option<usize>,
    line: Option<usize>,
    after_heading: Option<String>,
    before_heading: Option<String>,
    after_text: Option<String>,
    before_text: Option<String>,
) -> Result<InsertTextSelector> {
    let selector_count = usize::from(index.is_some())
        + usize::from(entry.is_some())
        + usize::from(page.is_some() || line.is_some())
        + usize::from(after_heading.is_some())
        + usize::from(before_heading.is_some())
        + usize::from(after_text.is_some())
        + usize::from(before_text.is_some());
    if selector_count != 1 {
        bail!(
            "provide exactly one insert-text selector: --index, --entry, --page with --line, --after-heading, --before-heading, --after-text, or --before-text"
        );
    }

    if let Some(index) = index {
        return Ok(InsertTextSelector::Index(index));
    }
    if let Some(entry) = entry {
        return Ok(InsertTextSelector::Entry(entry));
    }
    if page.is_some() || line.is_some() {
        let Some(page) = page else {
            bail!("--page and --line must be provided together");
        };
        let Some(line) = line else {
            bail!("--page and --line must be provided together");
        };
        return Ok(InsertTextSelector::PageLine { page, line });
    }
    if let Some(heading) = after_heading {
        return Ok(InsertTextSelector::AfterHeading(heading));
    }
    if let Some(heading) = before_heading {
        return Ok(InsertTextSelector::BeforeHeading(heading));
    }
    if let Some(text) = after_text {
        return Ok(InsertTextSelector::AfterText(text));
    }
    if let Some(text) = before_text {
        return Ok(InsertTextSelector::BeforeText(text));
    }

    unreachable!("selector count checked above")
}

fn range_selector(
    from_index: Option<i64>,
    to_index: Option<i64>,
    entry: Option<usize>,
    page: Option<usize>,
    line: Option<usize>,
    text: Option<String>,
    match_number: Option<usize>,
) -> Result<RangeSelector> {
    let selector_count = usize::from(from_index.is_some() || to_index.is_some())
        + usize::from(entry.is_some())
        + usize::from(page.is_some() || line.is_some())
        + usize::from(text.is_some() || match_number.is_some());
    if selector_count != 1 {
        bail!(
            "provide exactly one range selector: --from-index with --to-index, --entry, --page with --line, or --text with optional --match"
        );
    }
    if from_index.is_some() || to_index.is_some() {
        let Some(start_index) = from_index else {
            bail!("--from-index and --to-index must be provided together");
        };
        let Some(end_index) = to_index else {
            bail!("--from-index and --to-index must be provided together");
        };
        if end_index <= start_index {
            bail!("--to-index must be greater than --from-index");
        }
        return Ok(RangeSelector::IndexRange {
            start_index,
            end_index,
        });
    }
    if let Some(entry) = entry {
        return Ok(RangeSelector::Entry(entry));
    }
    if page.is_some() || line.is_some() {
        let Some(page) = page else {
            bail!("--page and --line must be provided together");
        };
        let Some(line) = line else {
            bail!("--page and --line must be provided together");
        };
        return Ok(RangeSelector::PageLine { page, line });
    }
    let Some(text) = text else {
        bail!("--text is required when using --match");
    };
    Ok(RangeSelector::Text { text, match_number })
}

fn document_map_with_entry(document_map: &DocumentMap, entry: &DocumentMapEntry) -> DocumentMap {
    DocumentMap {
        document_id: document_map.document_id.clone(),
        title: document_map.title.clone(),
        revision_id: document_map.revision_id.clone(),
        entries: vec![entry.clone()],
        document_locations: vec![entry.location.clone()],
        text_blocks: Vec::new(),
    }
}

fn resolve_content_entry<'a>(
    document_map: &'a DocumentMap,
    selector: &ContentSelector,
) -> Result<&'a DocumentMapEntry> {
    match selector {
        ContentSelector::Index(index) => document_map
            .entries
            .iter()
            .filter(|entry| {
                entry
                    .location
                    .index
                    .is_some_and(|entry_index| entry_index <= *index)
            })
            .max_by_key(|entry| entry.location.index)
            .with_context(|| format!("no content found at Google Docs index {index}")),
        ContentSelector::Entry(entry_number) => document_map
            .entries
            .iter()
            .find(|entry| entry.entry == *entry_number)
            .with_context(|| format!("Document Map entry {entry_number} was not found")),
        ContentSelector::PageLine { page, line } => document_map
            .entries
            .iter()
            .find(|entry| {
                entry.location.page == Some(*page) && entry.location.content_line == *line
            })
            .with_context(|| format!("no content found at page {page}, line {line}")),
        ContentSelector::Heading(heading) => resolve_heading(document_map, heading),
    }
}

fn resolve_range_selector(
    document_map: &DocumentMap,
    selector: &RangeSelector,
) -> Result<DocumentRange> {
    match selector {
        RangeSelector::IndexRange {
            start_index,
            end_index,
        } => Ok(DocumentRange {
            start_index: *start_index,
            end_index: *end_index,
            location: DocumentLocation {
                index: Some(*start_index),
                page: None,
                content_line: 0,
                confidence: crate::docs::map::LocationConfidence::Unknown,
            },
            preview: format!("range {start_index}..{end_index}"),
        }),
        RangeSelector::Entry(entry_number) => {
            let entry =
                resolve_content_entry(document_map, &ContentSelector::Entry(*entry_number))?;
            range_for_entry(document_map, entry)
        }
        RangeSelector::PageLine { page, line } => {
            let entry = resolve_content_entry(
                document_map,
                &ContentSelector::PageLine {
                    page: *page,
                    line: *line,
                },
            )?;
            range_for_entry(document_map, entry)
        }
        RangeSelector::Text { text, match_number } => {
            let command = ReplaceTextCommand {
                document_id: String::new(),
                old_text: text.clone(),
                new_text: String::new(),
                match_number: *match_number,
                all: false,
                dry_run: true,
                json: true,
                required_revision_id: None,
            };
            let ranges = resolve_replace_text_ranges(document_map, &command)?;
            ranges
                .into_iter()
                .next()
                .context("text range selector did not resolve a match")
        }
    }
}

fn range_for_entry(document_map: &DocumentMap, entry: &DocumentMapEntry) -> Result<DocumentRange> {
    let start_index = entry
        .location
        .index
        .context("selected Document Map entry has no Google Docs index")?;
    let end_index = document_map
        .entries
        .iter()
        .filter_map(|candidate| candidate.location.index)
        .find(|candidate_index| *candidate_index > start_index)
        .unwrap_or(start_index + 1);
    Ok(DocumentRange {
        start_index,
        end_index,
        location: entry.location.clone(),
        preview: entry.preview.clone(),
    })
}

fn resolve_table_handle<'a>(
    document_map: &'a DocumentMap,
    table_id: &str,
) -> Result<&'a DocumentMapEntry> {
    let entry = document_map
        .entries
        .iter()
        .find(|entry| {
            entry.kind == DocumentMapEntryKind::Table
                && entry.table_handle.as_deref() == Some(table_id)
        })
        .with_context(|| format!("table handle {table_id} was not found"))?;
    Ok(entry)
}

fn read_table_data(path: &str) -> Result<Vec<Vec<String>>> {
    let body = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read table data file: {path}"))?;
    let delimiter = if path.ends_with(".tsv") { '\t' } else { ',' };
    let rows = body
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            line.split(delimiter)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        bail!("table data file is empty");
    }
    let columns = rows[0].len();
    if columns == 0 || rows.iter().any(|row| row.len() != columns) {
        bail!("table data must be rectangular");
    }
    Ok(rows)
}

fn resolve_heading<'a>(
    document_map: &'a DocumentMap,
    heading: &str,
) -> Result<&'a DocumentMapEntry> {
    let matches = document_map
        .entries
        .iter()
        .filter(|entry| entry.kind == DocumentMapEntryKind::Heading && entry.preview == heading)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [entry] => Ok(entry),
        [] => bail!("heading selector {heading:?} did not match any Document Map entries"),
        candidates => {
            let candidate_list = candidates
                .iter()
                .map(|entry| {
                    format!(
                        "entry {} index {} page {} line {} preview {}",
                        entry.entry,
                        display_optional(entry.location.index),
                        display_optional(entry.location.page),
                        entry.location.content_line,
                        entry.preview
                    )
                })
                .collect::<Vec<_>>()
                .join("; ");
            bail!("ambiguous heading selector {heading:?}; candidates: {candidate_list}")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsertTextDryRun {
    revision_id: Option<String>,
    location: crate::docs::map::DocumentLocation,
    request_body: serde_json::Value,
    preview: InsertTextPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InsertTextPreview {
    before: String,
    after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceTextDryRun {
    revision_id: Option<String>,
    ranges: Vec<DocumentRange>,
    request_body: serde_json::Value,
    preview: ReplaceTextPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceTextPreview {
    changes: Vec<ReplaceTextPreviewChange>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ReplaceTextPreviewChange {
    range: DocumentRange,
    before: String,
    after: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocsHighLevelChange {
    revision_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    location: Option<DocumentLocation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    range: Option<DocumentRange>,
    request_body: serde_json::Value,
    preview: DocsChangePreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocsChangePreview {
    command: String,
    summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedInsertTextLocation {
    location: DocumentLocation,
    preview_before: String,
    preview_offset: usize,
}

fn prepare_insert_text_change(
    document_map: &DocumentMap,
    command: &InsertTextCommand,
) -> Result<InsertTextDryRun> {
    let resolved = resolve_insert_text_location(document_map, &command.selector)?;
    let request_body = insert_text_request_body(
        resolved.location.index,
        &command.text,
        command.required_revision_id.as_deref(),
    );
    let preview = InsertTextPreview {
        before: resolved.preview_before.clone(),
        after: insert_preview_text(
            &resolved.preview_before,
            resolved.preview_offset,
            &command.text,
        ),
    };

    Ok(InsertTextDryRun {
        revision_id: document_map.revision_id.clone(),
        location: resolved.location,
        request_body,
        preview,
    })
}

fn prepare_replace_text_change(
    document_map: &DocumentMap,
    command: &ReplaceTextCommand,
) -> Result<ReplaceTextDryRun> {
    let ranges = resolve_replace_text_ranges(document_map, command)?;
    let request_body = replace_text_request_body(
        &ranges,
        &command.new_text,
        command.required_revision_id.as_deref(),
    );
    let preview = replace_text_preview(document_map, &ranges, &command.old_text, &command.new_text);

    Ok(ReplaceTextDryRun {
        revision_id: document_map.revision_id.clone(),
        ranges,
        request_body,
        preview,
    })
}

fn prepare_insert_image_change(
    document_map: &DocumentMap,
    command: &InsertImageCommand,
) -> Result<DocsHighLevelChange> {
    let resolved = resolve_insert_text_location(document_map, &command.selector)?;
    let Some(index) = resolved.location.index else {
        bail!("insert-image selector resolved without a Google Docs index");
    };
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "insertInlineImage": {
                "location": { "index": index },
                "uri": command.image_uri
            }
        })],
        command.required_revision_id.as_deref(),
    );
    Ok(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(resolved.location),
        range: None,
        request_body,
        preview: DocsChangePreview {
            command: "insert-image".into(),
            summary: format!(
                "Insert inline image at index {index} from {}",
                command.image_uri
            ),
        },
    })
}

fn prepare_insert_table_change(
    document_map: &DocumentMap,
    command: &InsertTableCommand,
) -> Result<DocsHighLevelChange> {
    let data = match &command.data {
        Some(path) => Some(read_table_data(path)?),
        None => None,
    };
    let (rows, columns) = insert_table_dimensions(command, data.as_deref())?;
    let resolved = resolve_insert_text_location(document_map, &command.selector)?;
    let Some(index) = resolved.location.index else {
        bail!("insert-table selector resolved without a Google Docs index");
    };
    let mut requests = vec![serde_json::json!({
        "insertTable": {
            "location": { "index": index },
            "rows": rows,
            "columns": columns
        }
    })];
    if let Some(data) = &data {
        requests.extend(insert_table_data_requests(index, data));
    }
    let request_body =
        request_body_with_revision(requests, command.required_revision_id.as_deref());
    let summary = if let Some(data) = &data {
        format!(
            "Insert {}x{} table at index {index}: {}",
            rows,
            columns,
            compact_table_data_preview(data)
        )
    } else {
        format!("Insert {rows}x{columns} table at index {index}")
    };
    Ok(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(resolved.location),
        range: None,
        request_body,
        preview: DocsChangePreview {
            command: "insert-table".into(),
            summary,
        },
    })
}

fn insert_table_dimensions(
    command: &InsertTableCommand,
    data: Option<&[Vec<String>]>,
) -> Result<(usize, usize)> {
    if data.is_some() && (command.rows.is_some() || command.columns.is_some()) {
        bail!("insert-table accepts either --data or --rows with --columns, not both");
    }
    if let Some(data) = data {
        return Ok((data.len(), data[0].len()));
    }
    let Some(rows) = command.rows else {
        bail!("insert-table requires --data or --rows with --columns");
    };
    let Some(columns) = command.columns else {
        bail!("insert-table requires --data or --rows with --columns");
    };
    if rows == 0 || columns == 0 {
        bail!("insert-table requires --rows and --columns to be greater than zero");
    }
    Ok((rows, columns))
}

fn insert_table_data_requests(table_index: i64, data: &[Vec<String>]) -> Vec<serde_json::Value> {
    let mut requests = Vec::new();
    for (row_index, row) in data.iter().enumerate().rev() {
        for (column_index, text) in row.iter().enumerate().rev() {
            if text.is_empty() {
                continue;
            }
            requests.push(serde_json::json!({
                "insertText": {
                    "location": {
                        "index": inserted_table_cell_text_index(
                            table_index,
                            row_index,
                            column_index
                        )
                    },
                    "text": text
                }
            }));
        }
    }
    requests
}

fn inserted_table_cell_text_index(
    table_index: i64,
    row_index: usize,
    column_index: usize,
) -> i64 {
    table_index + 4 + (row_index as i64 * 5) + (column_index as i64 * 2)
}

fn prepare_edit_table_change(
    document_map: &DocumentMap,
    command: &EditTableCommand,
) -> Result<DocsHighLevelChange> {
    let data = read_table_data(&command.data)?;
    let table = resolve_table_handle(document_map, &command.table_id)?;
    let rows = table.rows.unwrap_or(0);
    let columns = table.columns.unwrap_or(0);
    if !command.resize && (data.len() != rows || data.iter().any(|row| row.len() != columns)) {
        bail!(
            "edit-table data dimensions are {}x{} but {} is {}x{}; pass --resize when structural resizing is supported",
            data.len(),
            data.first().map_or(0, Vec::len),
            command.table_id,
            rows,
            columns
        );
    }
    if command.resize {
        bail!("edit-table --resize is not supported yet");
    }
    if table.table_cells.len() != rows || table.table_cells.iter().any(|row| row.len() != columns) {
        bail!("selected table does not expose editable cell text ranges");
    }

    let request_body = request_body_with_revision(
        edit_table_requests(&table.table_cells, &data),
        command.required_revision_id.as_deref(),
    );
    Ok(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(table.location.clone()),
        range: None,
        request_body,
        preview: DocsChangePreview {
            command: "edit-table".into(),
            summary: format!(
                "Replace {} with {}x{} table data",
                command.table_id, rows, columns
            ),
        },
    })
}

fn edit_table_requests(
    table_cells: &[Vec<DocumentRange>],
    data: &[Vec<String>],
) -> Vec<serde_json::Value> {
    let mut requests = Vec::new();
    for (row_index, row) in table_cells.iter().enumerate().rev() {
        for (column_index, range) in row.iter().enumerate().rev() {
            if range.end_index > range.start_index {
                requests.push(serde_json::json!({
                    "deleteContentRange": {
                        "range": docs_range(range)
                    }
                }));
            }
            requests.push(serde_json::json!({
                "insertText": {
                    "location": { "index": range.start_index },
                    "text": data[row_index][column_index]
                }
            }));
        }
    }
    requests
}

fn prepare_apply_styles_change(
    document_map: &DocumentMap,
    command: &ApplyStylesCommand,
) -> Result<DocsHighLevelChange> {
    let range = resolve_range_selector(document_map, &command.selector)?;
    let (text_style, fields) = text_style_payload(command)?;
    let mut requests = Vec::new();
    if !fields.is_empty() {
        requests.push(serde_json::json!({
            "updateTextStyle": {
                "range": docs_range(&range),
                "textStyle": text_style,
                "fields": fields.join(",")
            }
        }));
    }
    if let Some(heading) = &command.heading {
        requests.push(serde_json::json!({
            "updateParagraphStyle": {
                "range": docs_range(&range),
                "paragraphStyle": { "namedStyleType": heading },
                "fields": "namedStyleType"
            }
        }));
    }
    if requests.is_empty() {
        bail!("apply-styles requires at least one style flag");
    }
    let request_body =
        request_body_with_revision(requests, command.required_revision_id.as_deref());
    Ok(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: Some(range.clone()),
        request_body,
        preview: DocsChangePreview {
            command: "apply-styles".into(),
            summary: format!(
                "Apply styles to range {}..{}",
                range.start_index, range.end_index
            ),
        },
    })
}

fn prepare_apply_list_change(
    document_map: &DocumentMap,
    command: &ApplyListCommand,
) -> Result<DocsHighLevelChange> {
    if command.list_type.is_some() && command.preset.is_some() {
        bail!("apply-list accepts either --type or --preset, not both");
    }
    let preset = command
        .preset
        .clone()
        .or_else(|| command.list_type.map(list_type_preset).map(str::to_string))
        .context("apply-list requires --type or --preset")?;
    let range = resolve_range_selector(document_map, &command.selector)?;
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "createParagraphBullets": {
                "range": docs_range(&range),
                "bulletPreset": preset
            }
        })],
        command.required_revision_id.as_deref(),
    );
    Ok(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: Some(range.clone()),
        request_body,
        preview: DocsChangePreview {
            command: "apply-list".into(),
            summary: format!(
                "Apply list preset to range {}..{}",
                range.start_index, range.end_index
            ),
        },
    })
}

fn resolve_insert_text_location(
    document_map: &DocumentMap,
    selector: &InsertTextSelector,
) -> Result<ResolvedInsertTextLocation> {
    match selector {
        InsertTextSelector::Index(index) => resolved_for_index(document_map, *index),
        InsertTextSelector::Entry(entry_number) => {
            let entry =
                resolve_content_entry(document_map, &ContentSelector::Entry(*entry_number))?;
            resolved_for_entry_start(entry)
        }
        InsertTextSelector::PageLine { page, line } => {
            let entry = resolve_content_entry(
                document_map,
                &ContentSelector::PageLine {
                    page: *page,
                    line: *line,
                },
            )?;
            resolved_for_entry_start(entry)
        }
        InsertTextSelector::BeforeHeading(heading) => {
            let entry = resolve_heading(document_map, heading)?;
            resolved_for_entry_start(entry)
        }
        InsertTextSelector::AfterHeading(heading) => {
            let entry = resolve_heading(document_map, heading)?;
            resolved_for_entry_end(document_map, entry)
        }
        InsertTextSelector::BeforeText(text) => {
            let range = resolve_text_anchor(document_map, text)?;
            let preview_offset =
                text_anchor_preview_offset(document_map, &range, range.start_index);
            Ok(ResolvedInsertTextLocation {
                location: DocumentLocation {
                    index: Some(range.start_index),
                    ..range.location.clone()
                },
                preview_before: range.preview.clone(),
                preview_offset,
            })
        }
        InsertTextSelector::AfterText(text) => {
            let range = resolve_text_anchor(document_map, text)?;
            let preview_offset = text_anchor_preview_offset(document_map, &range, range.end_index);
            Ok(ResolvedInsertTextLocation {
                location: DocumentLocation {
                    index: Some(range.end_index),
                    ..range.location.clone()
                },
                preview_before: range.preview.clone(),
                preview_offset,
            })
        }
    }
}

fn resolved_for_index(
    document_map: &DocumentMap,
    index: i64,
) -> Result<ResolvedInsertTextLocation> {
    let entry = resolve_content_entry(document_map, &ContentSelector::Index(index))?;
    let preview_offset = entry
        .location
        .index
        .map(|start| preview_offset_for_index(&entry.preview, start, index))
        .unwrap_or(0);
    Ok(ResolvedInsertTextLocation {
        location: DocumentLocation {
            index: Some(index),
            ..entry.location.clone()
        },
        preview_before: entry.preview.clone(),
        preview_offset,
    })
}

fn resolved_for_entry_start(entry: &DocumentMapEntry) -> Result<ResolvedInsertTextLocation> {
    let Some(index) = entry.location.index else {
        bail!(
            "Document Map entry {} does not have a Google Docs index",
            entry.entry
        );
    };
    Ok(ResolvedInsertTextLocation {
        location: DocumentLocation {
            index: Some(index),
            ..entry.location.clone()
        },
        preview_before: entry.preview.clone(),
        preview_offset: 0,
    })
}

fn resolved_for_entry_end(
    document_map: &DocumentMap,
    entry: &DocumentMapEntry,
) -> Result<ResolvedInsertTextLocation> {
    let Some(start_index) = entry.location.index else {
        bail!(
            "Document Map entry {} does not have a Google Docs index",
            entry.entry
        );
    };
    let end_index = document_map
        .text_blocks
        .iter()
        .find(|block| block.start_index == start_index)
        .map(text_block_end_index)
        .unwrap_or(start_index);
    Ok(ResolvedInsertTextLocation {
        location: DocumentLocation {
            index: Some(end_index),
            ..entry.location.clone()
        },
        preview_before: entry.preview.clone(),
        preview_offset: entry.preview.chars().count(),
    })
}

fn text_anchor_preview_offset(
    document_map: &DocumentMap,
    range: &DocumentRange,
    insertion_index: i64,
) -> usize {
    let block_start_index = document_map
        .text_blocks
        .iter()
        .find(|block| text_block_contains_range(block, range))
        .map(|block| block.start_index)
        .unwrap_or(range.start_index);

    preview_offset_for_index(&range.preview, block_start_index, insertion_index)
}

fn text_block_contains_range(block: &DocumentTextBlock, range: &DocumentRange) -> bool {
    block.start_index <= range.start_index && range.end_index <= text_block_end_index(block)
}

fn text_block_end_index(block: &DocumentTextBlock) -> i64 {
    block.start_index + block.text.encode_utf16().count() as i64
}

fn resolve_text_anchor(document_map: &DocumentMap, text: &str) -> Result<DocumentRange> {
    let matches = search_document_text(document_map, text);
    match matches.as_slice() {
        [range] => Ok(range.clone()),
        [] => bail!("text selector {text:?} did not match any Document Map entries"),
        candidates => {
            let candidate_list = format_range_candidates(candidates);
            bail!("ambiguous text selector {text:?}; candidates: {candidate_list}")
        }
    }
}

fn resolve_replace_text_ranges(
    document_map: &DocumentMap,
    command: &ReplaceTextCommand,
) -> Result<Vec<DocumentRange>> {
    if command.old_text.is_empty() {
        bail!("replace-text old text must not be empty");
    }
    if command.all && command.match_number.is_some() {
        bail!("provide only one replace-text disambiguator: --match or --all");
    }
    if command.match_number == Some(0) {
        bail!("--match must be 1 or greater");
    }

    let matches = search_document_text(document_map, &command.old_text);
    if matches.is_empty() {
        bail!(
            "replace-text did not match {old_text:?}",
            old_text = command.old_text.as_str()
        );
    }
    if command.all {
        return Ok(matches);
    }
    if let Some(match_number) = command.match_number {
        return matches
            .get(match_number - 1)
            .cloned()
            .map(|range| vec![range])
            .with_context(|| {
                format!(
                    "replace-text match {match_number} was not found; {} matches available",
                    matches.len()
                )
            });
    }

    match matches.as_slice() {
        [range] => Ok(vec![range.clone()]),
        candidates => {
            let candidate_list = format_range_candidates(candidates);
            bail!(
                "ambiguous replace-text match {old_text:?}; candidates: {candidate_list}",
                old_text = command.old_text.as_str()
            )
        }
    }
}

fn format_range_candidates(candidates: &[DocumentRange]) -> String {
    candidates
        .iter()
        .enumerate()
        .map(|(index, range)| {
            format!(
                "match {} index {} page {} line {} preview {}",
                index + 1,
                range.start_index,
                display_optional(range.location.page),
                range.location.content_line,
                range.preview
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

fn insert_text_request_body(
    index: Option<i64>,
    text: &str,
    required_revision_id: Option<&str>,
) -> serde_json::Value {
    request_body_with_revision(
        vec![serde_json::json!({
            "insertText": {
                "location": { "index": index },
                "text": text
            }
        })],
        required_revision_id,
    )
}

fn request_body_with_revision(
    requests: Vec<serde_json::Value>,
    required_revision_id: Option<&str>,
) -> serde_json::Value {
    let mut body = serde_json::json!({ "requests": requests });
    if let Some(required_revision_id) = required_revision_id {
        body["writeControl"] = serde_json::json!({
            "requiredRevisionId": required_revision_id
        });
    }
    body
}

fn docs_range(range: &DocumentRange) -> serde_json::Value {
    serde_json::json!({
        "startIndex": range.start_index,
        "endIndex": range.end_index
    })
}

fn text_style_payload(
    command: &ApplyStylesCommand,
) -> Result<(serde_json::Value, Vec<&'static str>)> {
    let mut style = serde_json::Map::new();
    let mut fields = Vec::new();
    if command.bold {
        style.insert("bold".into(), serde_json::Value::Bool(true));
        fields.push("bold");
    }
    if command.italic {
        style.insert("italic".into(), serde_json::Value::Bool(true));
        fields.push("italic");
    }
    if let Some(font_size) = command.font_size {
        style.insert(
            "fontSize".into(),
            serde_json::json!({ "magnitude": font_size, "unit": "PT" }),
        );
        fields.push("fontSize");
    }
    if let Some(color) = &command.foreground_color {
        style.insert("foregroundColor".into(), foreground_color_payload(color)?);
        fields.push("foregroundColor");
    }
    Ok((serde_json::Value::Object(style), fields))
}

fn foreground_color_payload(color: &str) -> Result<serde_json::Value> {
    let hex = color.strip_prefix('#').unwrap_or(color);
    if hex.len() != 6 || !hex.chars().all(|character| character.is_ascii_hexdigit()) {
        bail!("--foreground-color must be a #RRGGBB hex color");
    }
    let red = u8::from_str_radix(&hex[0..2], 16).context("invalid red color component")?;
    let green = u8::from_str_radix(&hex[2..4], 16).context("invalid green color component")?;
    let blue = u8::from_str_radix(&hex[4..6], 16).context("invalid blue color component")?;
    Ok(serde_json::json!({
        "color": {
            "rgbColor": {
                "red": red as f64 / 255.0,
                "green": green as f64 / 255.0,
                "blue": blue as f64 / 255.0
            }
        }
    }))
}

fn list_type_preset(list_type: DocsListType) -> &'static str {
    match list_type {
        DocsListType::Bullet => "BULLET_DISC_CIRCLE_SQUARE",
        DocsListType::Numbered => "NUMBERED_DECIMAL_ALPHA_ROMAN",
        DocsListType::Dash => "BULLET_DIAMONDX_ARROW3D_SQUARE",
        DocsListType::Checkbox => "BULLET_CHECKBOX",
    }
}

fn replace_text_request_body(
    ranges: &[DocumentRange],
    new_text: &str,
    required_revision_id: Option<&str>,
) -> serde_json::Value {
    let mut requests = Vec::new();
    let mut ranges_descending = ranges.to_vec();
    ranges_descending.sort_by_key(|range| std::cmp::Reverse(range.start_index));

    for range in ranges_descending {
        requests.push(serde_json::json!({
            "deleteContentRange": {
                "range": docs_range(&range)
            }
        }));
        requests.push(serde_json::json!({
            "insertText": {
                "location": { "index": range.start_index },
                "text": new_text
            }
        }));
    }

    request_body_with_revision(requests, required_revision_id)
}

fn insert_preview_text(before: &str, char_offset: usize, inserted_text: &str) -> String {
    let byte_offset = before
        .char_indices()
        .nth(char_offset)
        .map(|(index, _)| index)
        .unwrap_or(before.len());
    let mut after = before.to_string();
    after.insert_str(byte_offset, inserted_text);
    after
}

fn replace_text_preview(
    document_map: &DocumentMap,
    ranges: &[DocumentRange],
    old_text: &str,
    new_text: &str,
) -> ReplaceTextPreview {
    ReplaceTextPreview {
        changes: ranges
            .iter()
            .map(|range| ReplaceTextPreviewChange {
                range: range.clone(),
                before: range.preview.clone(),
                after: replace_text_preview_after(document_map, range, old_text, new_text),
            })
            .collect(),
    }
}

fn replace_text_preview_after(
    document_map: &DocumentMap,
    range: &DocumentRange,
    old_text: &str,
    new_text: &str,
) -> String {
    let block = document_map
        .text_blocks
        .iter()
        .find(|block| text_block_contains_range(block, range));
    let Some(block) = block else {
        return range.preview.replacen(old_text, new_text, 1);
    };

    let start_offset = utf16_byte_offset(&block.text, range.start_index - block.start_index);
    let end_offset = utf16_byte_offset(&block.text, range.end_index - block.start_index);
    let mut after = block.text.clone();
    after.replace_range(start_offset..end_offset, new_text);
    compact_preview(&after)
}

fn utf16_byte_offset(text: &str, utf16_offset: i64) -> usize {
    if utf16_offset <= 0 {
        return 0;
    }

    let mut units = 0;
    for (byte_index, character) in text.char_indices() {
        if units >= utf16_offset {
            return byte_index;
        }
        units += character.len_utf16() as i64;
    }
    text.len()
}

fn compact_preview(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_PREVIEW_CHARS: usize = 80;
    if compact.chars().count() <= MAX_PREVIEW_CHARS {
        compact
    } else {
        let mut truncated = compact
            .chars()
            .take(MAX_PREVIEW_CHARS - 3)
            .collect::<String>();
        truncated.push_str("...");
        truncated
    }
}

fn compact_table_data_preview(data: &[Vec<String>]) -> String {
    let preview = data
        .iter()
        .take(2)
        .map(|row| {
            row.iter()
                .take(3)
                .map(|cell| compact_preview(cell))
                .collect::<Vec<_>>()
                .join(" | ")
        })
        .collect::<Vec<_>>()
        .join(" / ");
    compact_preview(&preview)
}

fn preview_offset_for_index(preview: &str, block_start_index: i64, insertion_index: i64) -> usize {
    let requested_offset = insertion_index.saturating_sub(block_start_index) as usize;
    requested_offset.min(preview.chars().count())
}

fn write_insert_text_dry_run(
    out: &mut impl Write,
    dry_run: &InsertTextDryRun,
    json: bool,
) -> Result<()> {
    if json {
        write_json_line(out, dry_run, "failed to serialize Docs insert-text dry run")
    } else {
        write_insert_text_preview(out, dry_run)
    }
}

fn write_replace_text_dry_run(
    out: &mut impl Write,
    dry_run: &ReplaceTextDryRun,
    json: bool,
) -> Result<()> {
    if json {
        write_json_line(
            out,
            dry_run,
            "failed to serialize Docs replace-text dry run",
        )
    } else {
        write_replace_text_preview(out, dry_run)
    }
}

#[cfg(test)]
async fn apply_or_preview_docs_change<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    change: DocsHighLevelChange,
    dry_run: bool,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    command_name: &str,
) -> Result<()> {
    if dry_run {
        write_docs_change_preview(out, &change, json)
    } else {
        let options =
            batch_update_document_options(document_id, change.request_body, documents_url);
        let response = batch_update_document(client, &options)
            .await
            .with_context(|| format!("failed to apply Google Docs {command_name}"))?;
        write_json_line(
            out,
            &response,
            &format!("failed to serialize Docs {command_name} response"),
        )
    }
}

async fn apply_or_preview_docs_change_unified<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    change: DocsHighLevelChange,
    dry_run: bool,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
    command_name: &str,
) -> Result<()> {
    if dry_run {
        write_docs_change_preview(out, &change, json)
    } else {
        let options =
            batch_update_document_options(document_id.clone(), change.request_body, documents_url);
        let resource_key = resource_key("docs", &document_id);
        let response = run_with_docs_unified_access(
            config,
            store,
            account_override,
            &resource_key,
            DocsAccessAttempt::BatchUpdate(&options),
            state_path,
        )
        .await
        .with_context(|| format!("failed to apply Google Docs {command_name}"))?;
        write_json_line(
            out,
            &response,
            &format!("failed to serialize Docs {command_name} response"),
        )
    }
}

fn write_docs_change_preview(
    out: &mut impl Write,
    change: &DocsHighLevelChange,
    json: bool,
) -> Result<()> {
    if json {
        write_json_line(out, change, "failed to serialize Docs dry run")
    } else {
        writeln!(
            out,
            "{}: {}",
            change.preview.command, change.preview.summary
        )
        .context("failed to write Docs dry-run preview")?;
        Ok(())
    }
}

fn write_insert_text_preview(out: &mut impl Write, dry_run: &InsertTextDryRun) -> Result<()> {
    writeln!(
        out,
        "Insert text at index {}",
        display_optional(dry_run.location.index)
    )
    .context("failed to write Docs insert-text preview header")?;
    writeln!(out, "Before: {}", dry_run.preview.before)
        .context("failed to write Docs insert-text before preview")?;
    writeln!(out, "After: {}", dry_run.preview.after)
        .context("failed to write Docs insert-text after preview")?;
    Ok(())
}

fn write_replace_text_preview(out: &mut impl Write, dry_run: &ReplaceTextDryRun) -> Result<()> {
    writeln!(out, "Replace text in {} match(es)", dry_run.ranges.len())
        .context("failed to write Docs replace-text preview header")?;
    for (index, change) in dry_run.preview.changes.iter().enumerate() {
        writeln!(
            out,
            "Match {} at index {}",
            index + 1,
            change.range.start_index
        )
        .context("failed to write Docs replace-text match preview")?;
        writeln!(out, "Before: {}", change.before)
            .context("failed to write Docs replace-text before preview")?;
        writeln!(out, "After: {}", change.after)
            .context("failed to write Docs replace-text after preview")?;
    }
    Ok(())
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

fn write_document_map(out: &mut impl Write, document_map: &DocumentMap, json: bool) -> Result<()> {
    if json {
        write_json_line(out, document_map, "failed to serialize Docs Document Map")
    } else {
        write_document_map_table(out, document_map)
    }
}

fn write_filtered_entries(
    out: &mut impl Write,
    document_map: &DocumentMap,
    kinds: &[DocumentMapEntryKind],
    json: bool,
) -> Result<()> {
    let entries = document_map
        .entries
        .iter()
        .filter(|entry| kinds.contains(&entry.kind))
        .cloned()
        .collect::<Vec<_>>();
    if json {
        write_json_line(out, &entries, "failed to serialize Docs filtered entries")
    } else {
        write_document_entries_table(out, &entries)
    }
}

fn write_search_text_results(
    out: &mut impl Write,
    ranges: &[DocumentRange],
    json: bool,
) -> Result<()> {
    if json {
        write_json_line(out, ranges, "failed to serialize Docs text matches")
    } else {
        write_search_text_table(out, ranges)
    }
}

fn write_content_entry(
    out: &mut impl Write,
    document_map: &DocumentMap,
    entry: &DocumentMapEntry,
    json: bool,
) -> Result<()> {
    if json {
        write_json_line(out, entry, "failed to serialize Docs content entry")
    } else {
        write_document_map_table(out, &document_map_with_entry(document_map, entry))
    }
}

fn write_document_map_table(out: &mut impl Write, document_map: &DocumentMap) -> Result<()> {
    write_document_entries_table(out, &document_map.entries)
}

fn write_document_entries_table(out: &mut impl Write, entries: &[DocumentMapEntry]) -> Result<()> {
    writeln!(
        out,
        "{:<5} {:<7} {:<5} {:<4} {:<20} {:<10} {:<10} {:<18} {:<15} Preview",
        "Entry", "Index", "Page", "Line", "Kind", "Object", "Size", "Style", "Confidence"
    )
    .context("failed to write Docs Document Map header")?;

    for entry in entries {
        let style = entry.style.as_deref().unwrap_or("-");
        let object = entry
            .object_id
            .as_deref()
            .or(entry.table_handle.as_deref())
            .unwrap_or("-");
        let size = match (entry.rows, entry.columns) {
            (Some(rows), Some(columns)) => format!("{rows}x{columns}"),
            _ => "-".into(),
        };
        writeln!(
            out,
            "{:<5} {:<7} {:<5} {:<4} {:<20} {:<10} {:<10} {:<18} {:<15} {}",
            entry.entry,
            display_optional(entry.location.index),
            display_optional(entry.location.page),
            entry.location.content_line,
            format!("{:?}", entry.kind),
            object,
            size,
            style,
            format!("{:?}", entry.location.confidence),
            entry.preview
        )
        .context("failed to write Docs Document Map row")?;
    }

    Ok(())
}

fn write_search_text_table(out: &mut impl Write, ranges: &[DocumentRange]) -> Result<()> {
    writeln!(
        out,
        "{:<5} {:<5} {:<4} {:<5} {:<15} Preview",
        "Match", "Page", "Line", "Index", "Confidence"
    )
    .context("failed to write Docs text search header")?;

    for (match_number, range) in ranges.iter().enumerate() {
        writeln!(
            out,
            "{:<5} {:<5} {:<4} {:<5} {:<15} {}",
            match_number + 1,
            display_optional(range.location.page),
            range.location.content_line,
            range.start_index,
            format!("{:?}", range.location.confidence),
            range.preview
        )
        .context("failed to write Docs text search row")?;
    }

    Ok(())
}

fn display_optional<T: ToString>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".into())
}

fn write_json_line<T: serde::Serialize + ?Sized>(
    out: &mut impl Write,
    value: &T,
    context: &str,
) -> Result<()> {
    serde_json::to_writer(&mut *out, value).context(context.to_string())?;
    writeln!(out).context("failed to write output")?;
    Ok(())
}
