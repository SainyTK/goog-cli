use std::io::{Read, Write};
use std::path::Path;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::cli::DocsCommand;
use crate::docs::{
    batch_update_document,
    change::{
        prepare_apply_list_change, prepare_apply_styles_change, prepare_create_footer_change,
        prepare_create_footnote_change, prepare_create_header_change,
        prepare_create_named_range_change, prepare_delete_named_range_change,
        prepare_edit_table_change, prepare_insert_image_change, prepare_insert_page_break_change,
        prepare_insert_section_break_change, prepare_insert_table_change,
        prepare_insert_text_change, prepare_replace_text_change, request_body_required_revision_id,
        request_body_with_revision, set_request_body_required_revision_id,
        split_docs_request_bodies, table_header_style_requests, write_docs_change_preview,
        ApplyListCommand, ApplyStylesCommand, CreateFooterCommand, CreateFootnoteCommand,
        CreateHeaderCommand, CreateNamedRangeCommand, DeleteNamedRangeCommand, EditTableCommand,
        InsertImageCommand, InsertPageBreakCommand, InsertSectionBreakCommand, InsertTableCommand,
        InsertTextCommand, PreparedDocsChange, ReplaceTextCommand,
    },
    create_document, extract_style_template, get_document,
    map::build_document_map,
    map::resolve_content_entry,
    map::search_document_text,
    map::ContentSelector,
    map::DocumentMap,
    map::DocumentMapEntry,
    map::DocumentMapEntryKind,
    map::DocumentRange,
    map::InsertTextSelector,
    map::RangeSelector,
    style_template::{load_style_template_in, save_style_template_in},
    BatchUpdateDocumentOptions, CreateDocumentOptions, DocsError, GetDocumentOptions,
    StyleTemplate,
};
use anyhow::{bail, Context, Result};

pub fn run<S: AccountStore>(
    mut cmd: DocsCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
) -> Result<()> {
    cmd.normalize_document_id();
    match cmd {
        DocsCommand::Create { title } => {
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_create_to(&client, title, &mut std::io::stdout(), None))
        }
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
        DocsCommand::InsertPageBreak {
            document_id,
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
            runtime.block_on(run_insert_page_break_unified_to(
                config,
                store,
                account_override,
                InsertPageBreakCommand {
                    document_id,
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
        DocsCommand::InsertSectionBreak {
            document_id,
            section_type,
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
            runtime.block_on(run_insert_section_break_unified_to(
                config,
                store,
                account_override,
                InsertSectionBreakCommand {
                    document_id,
                    section_type,
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
        DocsCommand::CreateHeader {
            document_id,
            dry_run,
            json,
            required_revision_id,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_create_header_unified_to(
                config,
                store,
                account_override,
                CreateHeaderCommand {
                    document_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::CreateFooter {
            document_id,
            dry_run,
            json,
            required_revision_id,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_create_footer_unified_to(
                config,
                store,
                account_override,
                CreateFooterCommand {
                    document_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::CreateFootnote {
            document_id,
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
            runtime.block_on(run_create_footnote_unified_to(
                config,
                store,
                account_override,
                CreateFootnoteCommand {
                    document_id,
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
            no_auto_style,
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
                    no_auto_style,
                },
                &mut std::io::stdout(),
                None,
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
            style_json,
            dry_run,
            json,
            required_revision_id,
            no_auto_style,
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
                    style_json,
                    dry_run,
                    json,
                    required_revision_id,
                    no_auto_style,
                },
                &mut std::io::stdout(),
                None,
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
            text,
            match_number,
            list_type,
            preset,
            dry_run,
            json,
            required_revision_id,
            no_auto_style,
        } => {
            let selector =
                range_selector(from_index, to_index, entry, page, line, text, match_number)?;
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
                    no_auto_style,
                },
                &mut std::io::stdout(),
                None,
                None,
                None,
            ))
        }
        DocsCommand::CreateNamedRange {
            document_id,
            name,
            from_index,
            to_index,
            entry,
            page,
            line,
            text,
            match_number,
            dry_run,
            json,
            required_revision_id,
        } => {
            let selector =
                range_selector(from_index, to_index, entry, page, line, text, match_number)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_create_named_range_unified_to(
                config,
                store,
                account_override,
                CreateNamedRangeCommand {
                    document_id,
                    name,
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
        DocsCommand::DeleteNamedRange {
            document_id,
            named_range_id,
            name,
            dry_run,
            json,
            required_revision_id,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_delete_named_range_unified_to(
                config,
                store,
                account_override,
                DeleteNamedRangeCommand {
                    document_id,
                    named_range_id,
                    name,
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
        DocsCommand::ShowStyleTemplate { document_id, json } => {
            run_show_style_template(&document_id, json, &mut std::io::stdout(), None)
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
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
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
    )
    .await
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
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
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
    )
    .await
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_insert_page_break_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: InsertPageBreakCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_insert_page_break_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
}

pub(super) async fn run_insert_page_break_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: InsertPageBreakCommand,
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
    let change = prepare_insert_page_break_change(&document_map, &command)?;
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_insert_section_break_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: InsertSectionBreakCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_insert_section_break_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
}

pub(super) async fn run_insert_section_break_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: InsertSectionBreakCommand,
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
    let change = prepare_insert_section_break_change(&document_map, &command)?;
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_create_header_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: CreateHeaderCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_create_header_change(&document_map, &command);
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
}

pub(super) async fn run_create_header_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CreateHeaderCommand,
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
    let change = prepare_create_header_change(&document_map, &command);
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_create_footer_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: CreateFooterCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_create_footer_change(&document_map, &command);
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
}

pub(super) async fn run_create_footer_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CreateFooterCommand,
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
    let change = prepare_create_footer_change(&document_map, &command);
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_create_footnote_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: CreateFootnoteCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_create_footnote_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
}

pub(super) async fn run_create_footnote_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CreateFootnoteCommand,
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
    let change = prepare_create_footnote_change(&document_map, &command)?;
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_insert_table_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: InsertTableCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    style_cache_dir: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_insert_table_change(&document_map, &command)?;
    let dry_run = command.dry_run;
    let no_auto_style = command.no_auto_style;
    let document_id = command.document_id.clone();
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await?;

    if !dry_run && !no_auto_style {
        apply_table_header_auto_style_to(client, &document_id, documents_url, style_cache_dir)
            .await;
    }
    Ok(())
}

pub(super) async fn run_insert_table_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: InsertTableCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
    style_cache_dir: Option<&Path>,
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
    let dry_run = command.dry_run;
    let no_auto_style = command.no_auto_style;
    let document_id = command.document_id.clone();
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
    )
    .await?;

    if !dry_run && !no_auto_style {
        apply_table_header_auto_style_unified_to(
            config,
            store,
            account_override,
            &document_id,
            documents_url,
            state_path,
            style_cache_dir,
        )
        .await;
    }
    Ok(())
}

#[cfg(test)]
async fn apply_table_header_auto_style_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: &str,
    documents_url: Option<&str>,
    style_cache_dir: Option<&Path>,
) {
    let outcome: Result<()> = async {
        let Some(table_style) = load_cached_style_template(document_id, style_cache_dir)
            .and_then(|template| template.table)
        else {
            return Ok(());
        };
        let document_map = get_document_map(client, document_id.to_string(), documents_url).await?;
        let Some(requests) = table_header_style_requests(&document_map, &table_style) else {
            return Ok(());
        };
        if requests.is_empty() {
            return Ok(());
        }
        let options = batch_update_document_options(
            document_id.to_string(),
            request_body_with_revision(requests, None),
            documents_url,
        );
        batch_update_document(client, &options).await?;
        Ok(())
    }
    .await;

    if let Err(err) = outcome {
        eprintln!("warning: failed to apply cached table header style to {document_id}: {err:#}");
    }
}

async fn apply_table_header_auto_style_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: &str,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
    style_cache_dir: Option<&Path>,
) {
    let outcome: Result<()> = async {
        let Some(table_style) = load_cached_style_template(document_id, style_cache_dir)
            .and_then(|template| template.table)
        else {
            return Ok(());
        };
        let document_map = get_document_map_unified(
            config,
            store,
            account_override,
            document_id.to_string(),
            documents_url,
            state_path,
        )
        .await?;
        let Some(requests) = table_header_style_requests(&document_map, &table_style) else {
            return Ok(());
        };
        if requests.is_empty() {
            return Ok(());
        }
        let options = batch_update_document_options(
            document_id.to_string(),
            request_body_with_revision(requests, None),
            documents_url,
        );
        let resource_key = resource_key("docs", document_id);
        run_with_docs_unified_access(
            config,
            store,
            account_override,
            &resource_key,
            DocsAccessAttempt::BatchUpdate(&options),
            state_path,
        )
        .await?;
        Ok(())
    }
    .await;

    if let Err(err) = outcome {
        eprintln!("warning: failed to apply cached table header style to {document_id}: {err:#}");
    }
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_apply_styles_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: ApplyStylesCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    style_cache_dir: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let style_template = if command.no_auto_style {
        None
    } else {
        load_cached_style_template(&command.document_id, style_cache_dir)
    };
    let change = prepare_apply_styles_change(&document_map, &command, style_template.as_ref())?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
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
    style_cache_dir: Option<&Path>,
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
    let style_template = if command.no_auto_style {
        None
    } else {
        load_cached_style_template(&command.document_id, style_cache_dir)
    };
    let change = prepare_apply_styles_change(&document_map, &command, style_template.as_ref())?;
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_apply_list_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: ApplyListCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    style_cache_dir: Option<&Path>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let style_template = if command.no_auto_style {
        None
    } else {
        load_cached_style_template(&command.document_id, style_cache_dir)
    };
    let change = prepare_apply_list_change(&document_map, &command, style_template.as_ref())?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
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
    style_cache_dir: Option<&Path>,
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
    let style_template = if command.no_auto_style {
        None
    } else {
        load_cached_style_template(&command.document_id, style_cache_dir)
    };
    let change = prepare_apply_list_change(&document_map, &command, style_template.as_ref())?;
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_create_named_range_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: CreateNamedRangeCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_create_named_range_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
}

pub(super) async fn run_create_named_range_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CreateNamedRangeCommand,
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
    let change = prepare_create_named_range_change(&document_map, &command)?;
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
    )
    .await
}

#[cfg(test)]
pub(super) async fn run_delete_named_range_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: DeleteNamedRangeCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_delete_named_range_change(&document_map, &command)?;
    apply_or_preview_docs_change(
        client,
        command.document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
    )
    .await
}

pub(super) async fn run_delete_named_range_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: DeleteNamedRangeCommand,
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
    let change = prepare_delete_named_range_change(&document_map, &command)?;
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
    )
    .await
}

pub(super) async fn run_create_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    title: String,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let mut options = CreateDocumentOptions::new(title);
    if let Some(documents_url) = documents_url {
        options = options.with_documents_url(documents_url);
    }

    let document = create_document(client, &options)
        .await
        .context("failed to create Google Docs Document")?;
    let document_id = document
        .get("documentId")
        .and_then(serde_json::Value::as_str)
        .context("Google Docs create response did not include a documentId")?;

    writeln!(
        out,
        "{}\thttps://docs.google.com/document/d/{}/edit",
        document_id, document_id
    )
    .context("failed to write output")?;
    Ok(())
}

#[cfg(test)]
pub(super) async fn run_get_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    fields: Option<String>,
    include_tabs_content: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    style_cache_dir: Option<&Path>,
) -> Result<()> {
    let options = get_document_options(
        document_id.clone(),
        fields,
        include_tabs_content,
        documents_url,
    );

    let document = get_document(client, &options)
        .await
        .context("failed to fetch Google Docs Document")?;
    refresh_style_template_cache(&document_id, &document, style_cache_dir);
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
    style_cache_dir: Option<&Path>,
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

    refresh_style_template_cache(&document_id, &document, style_cache_dir);
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
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, serde_json::Value, DocsError> {
            Box::pin(run_docs_access_as_account(config, store, &attempt, account))
        },
        is_target_access_failure,
    )
    .await
}

async fn run_docs_access_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    attempt: &DocsAccessAttempt<'_>,
    account: String,
) -> Result<serde_json::Value, DocsError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))?;
    let result = attempt_docs_access(&client, attempt).await?;
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
async fn apply_or_preview_docs_change<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    change: PreparedDocsChange,
    dry_run: bool,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    if dry_run {
        write_docs_change_preview(out, &change, json)
    } else {
        let command_name = change.command_name().to_string();
        let response =
            apply_docs_change_requests(client, document_id, change, documents_url).await?;
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
    change: PreparedDocsChange,
    dry_run: bool,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    if dry_run {
        write_docs_change_preview(out, &change, json)
    } else {
        let command_name = change.command_name().to_string();
        let response = apply_docs_change_requests_unified(
            config,
            store,
            account_override,
            document_id,
            change,
            documents_url,
            state_path,
        )
        .await?;
        write_json_line(
            out,
            &response,
            &format!("failed to serialize Docs {command_name} response"),
        )
    }
}

#[cfg(test)]
async fn apply_docs_change_requests<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    change: PreparedDocsChange,
    documents_url: Option<&str>,
) -> Result<serde_json::Value> {
    let command_name = change.command_name().to_string();
    let mut required_revision_id = request_body_required_revision_id(change.request_body());
    let request_bodies = split_docs_request_bodies(change.request_body(), &command_name);
    let mut final_response = serde_json::Value::Null;

    for mut request_body in request_bodies {
        set_request_body_required_revision_id(&mut request_body, required_revision_id.as_deref());
        let options =
            batch_update_document_options(document_id.clone(), request_body, documents_url);
        let response = batch_update_document(client, &options)
            .await
            .with_context(|| format!("failed to apply Google Docs {command_name}"))?;
        required_revision_id = request_body_required_revision_id(&response);
        final_response = response;
    }

    Ok(final_response)
}

async fn apply_docs_change_requests_unified<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    change: PreparedDocsChange,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<serde_json::Value> {
    let resource_key = resource_key("docs", &document_id);
    let command_name = change.command_name().to_string();
    let mut required_revision_id = request_body_required_revision_id(change.request_body());
    let request_bodies = split_docs_request_bodies(change.request_body(), &command_name);
    let mut final_response = serde_json::Value::Null;

    for mut request_body in request_bodies {
        set_request_body_required_revision_id(&mut request_body, required_revision_id.as_deref());
        let options =
            batch_update_document_options(document_id.clone(), request_body, documents_url);
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
        required_revision_id = request_body_required_revision_id(&response);
        final_response = response;
    }

    Ok(final_response)
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
        insertion_locations: Vec::new(),
    }
}

/// Refreshes the on-disk style template cache for a document after a
/// successful `docs get`. This is a side effect only: failures are logged to
/// stderr and never cause the `get` command itself to fail.
fn refresh_style_template_cache(
    document_id: &str,
    document: &serde_json::Value,
    style_cache_dir: Option<&Path>,
) {
    let Some(template) = extract_style_template(document_id, document) else {
        return;
    };
    if let Err(err) = save_style_template_in(style_cache_dir, &template) {
        eprintln!("warning: failed to update cached style template for {document_id}: {err}");
    }
}

fn load_cached_style_template(
    document_id: &str,
    style_cache_dir: Option<&Path>,
) -> Option<StyleTemplate> {
    load_style_template_in(style_cache_dir, document_id)
        .ok()
        .flatten()
}

pub(super) fn run_show_style_template(
    document_id: &str,
    json: bool,
    out: &mut impl Write,
    style_cache_dir: Option<&Path>,
) -> Result<()> {
    match load_style_template_in(style_cache_dir, document_id)
        .context("failed to read cached style template")?
    {
        Some(template) => {
            if json {
                write_json_line(out, &template, "failed to serialize cached style template")
            } else {
                write_style_template_summary(out, &template)
            }
        }
        None => {
            writeln!(
                out,
                "no cached style template for this document; run `docs get {document_id}` first"
            )
            .context("failed to write missing style template message")?;
            Ok(())
        }
    }
}

fn write_style_template_summary(out: &mut impl Write, template: &StyleTemplate) -> Result<()> {
    writeln!(out, "Style template for {}", template.document_id)
        .context("failed to write style template header")?;
    if let Some(revision_id) = &template.source_revision_id {
        writeln!(out, "Source revision: {revision_id}")
            .context("failed to write style template revision")?;
    }
    let mut named_style_types: Vec<&String> = template.named_styles.keys().collect();
    named_style_types.sort();
    for style_type in named_style_types {
        let named_style = &template.named_styles[style_type];
        writeln!(
            out,
            "{style_type}: bold={:?} italic={:?} fontSize={:?} color={:?}",
            named_style.text_style.bold,
            named_style.text_style.italic,
            named_style.text_style.font_size_pt,
            named_style.text_style.foreground_color
        )
        .context("failed to write named style summary")?;
    }
    if let Some(table) = &template.table {
        writeln!(
            out,
            "table header: background={:?} bold={:?} italic={:?}",
            table.header_row.background_color,
            table.header_row.text_style.bold,
            table.header_row.text_style.italic
        )
        .context("failed to write table style summary")?;
    }
    if let Some(list) = &template.list {
        writeln!(
            out,
            "list: type={:?} preset={}",
            list.list_type, list.preset
        )
        .context("failed to write list style summary")?;
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
        "{:<5} {:<7} {:<5} {:<4} {:<20} {:<10} {:<10} {:<10} {:<18} {:<15} Preview",
        "Entry", "Index", "Page", "Line", "Kind", "Handle", "Object", "Size", "Style", "Confidence"
    )
    .context("failed to write Docs Document Map header")?;

    for entry in entries {
        let style = entry.style.as_deref().unwrap_or("-");
        let handle = entry
            .image_handle
            .as_deref()
            .or(entry.table_handle.as_deref())
            .unwrap_or("-");
        let object = entry.object_id.as_deref().unwrap_or("-");
        let size = match (entry.rows, entry.columns) {
            (Some(rows), Some(columns)) => format!("{rows}x{columns}"),
            _ => "-".into(),
        };
        writeln!(
            out,
            "{:<5} {:<7} {:<5} {:<4} {:<20} {:<10} {:<10} {:<10} {:<18} {:<15} {}",
            entry.entry,
            display_optional(entry.location.index),
            display_optional(entry.location.page),
            entry.location.content_line,
            format!("{:?}", entry.kind),
            handle,
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
