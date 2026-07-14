use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::cli::{
    DocsBreakCommand, DocsCommand, DocsCompareScope, DocsFooterCommand, DocsFootnoteCommand,
    DocsHeaderCommand, DocsImageCommand, DocsMapType, DocsNamedRangeCommand, DocsStyleCommand,
    DocsTableCommand, DocsTextCommand,
};
use crate::docs::{
    batch_update_document,
    change::{
        prepare_apply_list_change, prepare_apply_styles_change, prepare_configure_page_change,
        prepare_copy_named_styles_change, prepare_copy_page_style_change,
        prepare_create_footer_change, prepare_create_footnote_change, prepare_create_header_change,
        prepare_create_named_range_change, prepare_delete_named_range_change,
        prepare_edit_table_change, prepare_insert_image_change, prepare_insert_page_break_change,
        prepare_insert_section_break_change, prepare_insert_table_change,
        prepare_insert_text_change, prepare_pin_table_header_rows_change,
        prepare_replace_text_change, prepare_set_table_column_widths_change,
        prepare_style_table_row_change, prepare_update_named_style_change,
        request_body_required_revision_id, request_body_with_revision,
        set_request_body_required_revision_id, split_docs_request_bodies,
        table_header_style_requests, write_docs_change_preview, ApplyListCommand,
        ApplyStylesCommand, ConfigurePageCommand, CopyNamedStylesCommand, CopyPageStyleCommand,
        CreateFooterCommand, CreateFootnoteCommand, CreateHeaderCommand, CreateNamedRangeCommand,
        DeleteNamedRangeCommand, EditTableCommand, InsertImageCommand, InsertPageBreakCommand,
        InsertSectionBreakCommand, InsertTableCommand, InsertTextCommand,
        PinTableHeaderRowsCommand, PreparedDocsChange, ReplaceTextCommand,
        SetTableColumnWidthsCommand, StyleTableRowCommand, UpdateNamedStyleCommand,
    },
    copy_document, create_document, extract_style_template, get_document,
    map::build_document_map,
    map::resolve_content_entry,
    map::search_document_text,
    map::ContentSelector,
    map::DocumentBreak,
    map::DocumentList,
    map::DocumentMap,
    map::DocumentMapEntry,
    map::DocumentMapEntryKind,
    map::DocumentRange,
    map::DocumentSegment,
    map::InsertTextSelector,
    map::RangeSelector,
    style_template::{load_style_template_in, save_style_template_in},
    BatchUpdateDocumentOptions, CopyDocumentOptions, CreateDocumentOptions, DocsError,
    GetDocumentOptions, StyleTemplate,
};
use crate::drive::{
    export_google_file, DownloadedFile, DriveError, ExportGoogleFileOptions, GoogleFileExportFormat,
};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub fn run<S: AccountStore>(
    mut cmd: DocsCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    output_json_by_default: bool,
    quiet: bool,
) -> Result<()> {
    cmd.normalize_document_id();
    match cmd {
        DocsCommand::List {
            limit,
            all,
            folder,
            json,
        } => {
            let json = super::drive::should_emit_json(json, output_json_by_default);
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(super::drive::run_docs_list_command_to(
                config,
                store,
                account_override,
                limit,
                all,
                folder,
                json,
                quiet,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
                None,
            ))
        }
        DocsCommand::Create { title } => {
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_create_to(&client, title, &mut std::io::stdout(), None))
        }
        DocsCommand::Copy {
            source_document_id,
            title,
        } => {
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_copy_to(
                &client,
                source_document_id,
                title,
                &mut std::io::stdout(),
                None,
            ))
        }
        DocsCommand::Compare {
            source_document_id,
            target_document_id,
            json,
            scope,
            fail_on_difference,
            max_differences,
            summary_only,
            difference_pattern,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_compare_unified_to(
                config,
                store,
                account_override,
                CompareDocumentsCommand {
                    source_document_id,
                    target_document_id,
                    json,
                    scope,
                    fail_on_difference,
                    max_differences,
                    summary_only,
                    difference_pattern,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::ExportPdf {
            document_id,
            output,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_export_pdf_unified_to(
                config,
                store,
                account_override,
                document_id,
                PathBuf::from(output),
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Map {
            document_id,
            type_,
            index,
            entry,
            page,
            line,
            heading,
            json,
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            if let Some(selector) = optional_content_selector(index, entry, page, line, heading)? {
                if type_ != DocsMapType::All {
                    bail!("--type cannot be combined with content selectors");
                }
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
            } else {
                runtime.block_on(run_map_unified_to(
                    config,
                    store,
                    account_override,
                    document_id,
                    type_,
                    json,
                    &mut std::io::stdout(),
                    None,
                    None,
                ))
            }
        }
        DocsCommand::Text {
            command:
                DocsTextCommand::Search {
                    document_id,
                    text,
                    json,
                },
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
        DocsCommand::Text {
            command:
                DocsTextCommand::Insert {
                    document_id,
                    text,
                    at,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let selector = insert_text_selector(at)?;
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
        DocsCommand::Text {
            command:
                DocsTextCommand::Replace {
                    document_id,
                    old_text,
                    new_text,
                    match_number,
                    all,
                    dry_run,
                    json,
                    required_revision_id,
                },
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
        DocsCommand::Image {
            command:
                DocsImageCommand::Insert {
                    document_id,
                    image_uri,
                    at,
                    segment_id,
                    width,
                    height,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let selector = at.map(insert_text_selector).transpose()?;
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
                    segment_id,
                    width,
                    height,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Break {
            command:
                DocsBreakCommand::Page {
                    document_id,
                    at,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let selector = insert_text_selector(at)?;
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
        DocsCommand::Break {
            command:
                DocsBreakCommand::Section {
                    document_id,
                    section_type,
                    at,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let selector = insert_text_selector(at)?;
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
        DocsCommand::Header {
            command:
                DocsHeaderCommand::Create {
                    document_id,
                    text,
                    section_break_index,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_create_header_unified_to(
                config,
                store,
                account_override,
                CreateHeaderCommand {
                    document_id,
                    text,
                    section_break_index,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Footer {
            command:
                DocsFooterCommand::Create {
                    document_id,
                    text,
                    section_break_index,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_create_footer_unified_to(
                config,
                store,
                account_override,
                CreateFooterCommand {
                    document_id,
                    text,
                    section_break_index,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Footnote {
            command:
                DocsFootnoteCommand::Insert {
                    document_id,
                    at,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let selector = insert_text_selector(at)?;
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
        DocsCommand::Table {
            command:
                DocsTableCommand::Insert {
                    document_id,
                    data,
                    rows,
                    columns,
                    at,
                    dry_run,
                    json,
                    required_revision_id,
                    no_auto_style,
                },
        } => {
            let selector = insert_text_selector(at)?;
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
        DocsCommand::Table {
            command:
                DocsTableCommand::Columns {
                    document_id,
                    table_id,
                    widths,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_set_table_column_widths_unified_to(
                config,
                store,
                account_override,
                SetTableColumnWidthsCommand {
                    document_id,
                    table_id,
                    widths,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Table {
            command:
                DocsTableCommand::Style {
                    document_id,
                    table_id,
                    row,
                    column,
                    background_color,
                    content_alignment,
                    border_color,
                    border_width,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_style_table_row_unified_to(
                config,
                store,
                account_override,
                StyleTableRowCommand {
                    document_id,
                    table_id,
                    row,
                    column,
                    background_color,
                    content_alignment,
                    border_color,
                    border_width,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Table {
            command:
                DocsTableCommand::HeaderRows {
                    document_id,
                    table_id,
                    rows,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_pin_table_header_rows_unified_to(
                config,
                store,
                account_override,
                PinTableHeaderRowsCommand {
                    document_id,
                    table_id,
                    rows,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Table {
            command:
                DocsTableCommand::Edit {
                    document_id,
                    table_id,
                    data,
                    resize,
                    dry_run,
                    json,
                    required_revision_id,
                },
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
        DocsCommand::Style {
            command:
                DocsStyleCommand::Apply {
                    document_id,
                    from_index,
                    to_index,
                    segment_id,
                    entry,
                    page,
                    line,
                    text,
                    match_number,
                    bold,
                    italic,
                    underline,
                    font_size,
                    font_family,
                    foreground_color,
                    link_heading_id,
                    alignment,
                    direction,
                    space_above,
                    space_below,
                    line_spacing,
                    spacing_mode,
                    indent_start,
                    indent_end,
                    indent_first_line,
                    keep_with_next,
                    keep_lines_together,
                    avoid_widow_and_orphan,
                    page_break_before,
                    heading,
                    style_json,
                    dry_run,
                    json,
                    required_revision_id,
                    no_cached_style,
                },
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
                    segment_id: segment_id.map(|segment_id| *segment_id),
                    bold,
                    italic,
                    underline,
                    font_size,
                    font_family,
                    foreground_color,
                    link_heading_id: link_heading_id.map(|heading_id| *heading_id),
                    alignment,
                    direction,
                    space_above,
                    space_below,
                    line_spacing,
                    spacing_mode,
                    indent_start,
                    indent_end,
                    indent_first_line,
                    keep_with_next,
                    keep_lines_together,
                    avoid_widow_and_orphan,
                    page_break_before,
                    heading,
                    style_json: style_json.map(|style_json| *style_json),
                    dry_run,
                    json,
                    required_revision_id: required_revision_id
                        .map(|required_revision_id| *required_revision_id),
                    no_auto_style: no_cached_style,
                },
                &mut std::io::stdout(),
                None,
                None,
                None,
            ))
        }
        DocsCommand::Style {
            command:
                DocsStyleCommand::Named {
                    document_id,
                    named_style,
                    style_json,
                    tab_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_update_named_style_unified_to(
                config,
                store,
                account_override,
                UpdateNamedStyleCommand {
                    document_id,
                    named_style,
                    style_json: *style_json,
                    tab_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Style {
            command:
                DocsStyleCommand::CopyNamed {
                    source_document_id,
                    target_document_id,
                    source_tab_id,
                    target_tab_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_copy_named_styles_unified_to(
                config,
                store,
                account_override,
                CopyNamedStylesCommand {
                    source_document_id,
                    target_document_id,
                    source_tab_id,
                    target_tab_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Style {
            command:
                DocsStyleCommand::Page {
                    document_id,
                    page_width,
                    page_height,
                    margin_top,
                    margin_bottom,
                    margin_left,
                    margin_right,
                    margin_header,
                    margin_footer,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_configure_page_unified_to(
                config,
                store,
                account_override,
                ConfigurePageCommand {
                    document_id,
                    page_width,
                    page_height,
                    margin_top,
                    margin_bottom,
                    margin_left,
                    margin_right,
                    margin_header,
                    margin_footer,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::Style {
            command:
                DocsStyleCommand::CopyPage {
                    source_document_id,
                    target_document_id,
                    source_tab_id,
                    target_tab_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
        } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_copy_page_style_unified_to(
                config,
                store,
                account_override,
                CopyPageStyleCommand {
                    source_document_id,
                    target_document_id,
                    source_tab_id,
                    target_tab_id,
                    dry_run,
                    json,
                    required_revision_id,
                },
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
        DocsCommand::ListFormat {
            command:
                crate::cli::DocsListCommand::Apply {
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
                    no_cached_style,
                },
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
                    no_auto_style: no_cached_style,
                },
                &mut std::io::stdout(),
                None,
                None,
                None,
            ))
        }
        DocsCommand::NamedRange { command } => run_named_range_command(
            command,
            config,
            store,
            account_override,
            &mut std::io::stdout(),
        ),
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
        DocsCommand::Style {
            command: DocsStyleCommand::Template { document_id, json },
        } => run_show_style_template(&document_id, json, &mut std::io::stdout(), None),
    }
}

fn run_named_range_command<S: AccountStore>(
    command: DocsNamedRangeCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    out: &mut impl Write,
) -> Result<()> {
    match command {
        DocsNamedRangeCommand::Create {
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
                out,
                None,
                None,
            ))
        }
        DocsNamedRangeCommand::Delete {
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
                out,
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
    map_type: DocsMapType,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, document_id, documents_url).await?;
    write_document_map(out, &document_map, map_type, json)
}

pub(super) async fn run_map_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    map_type: DocsMapType,
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
    write_document_map(out, &document_map, map_type, json)
}

#[cfg(test)]
pub(super) async fn run_compare_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: CompareDocumentsCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let source_map = get_document_map(client, command.source_document_id, documents_url).await?;
    let target_map = get_document_map(client, command.target_document_id, documents_url).await?;
    write_document_comparison(
        out,
        &source_map,
        &target_map,
        command.json,
        command.scope,
        command.fail_on_difference,
        DocumentComparisonPreview {
            max_differences: command.max_differences as usize,
            summary_only: command.summary_only,
            difference_pattern: command.difference_pattern.as_deref(),
        },
    )
}

pub(super) async fn run_compare_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CompareDocumentsCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let source_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.source_document_id,
        documents_url,
        state_path,
    )
    .await?;
    let target_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.target_document_id,
        documents_url,
        state_path,
    )
    .await?;
    write_document_comparison(
        out,
        &source_map,
        &target_map,
        command.json,
        command.scope,
        command.fail_on_difference,
        DocumentComparisonPreview {
            max_differences: command.max_differences as usize,
            summary_only: command.summary_only,
            difference_pattern: command.difference_pattern.as_deref(),
        },
    )
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
    apply_or_preview_created_segment(
        client,
        command.document_id,
        change,
        command.text.as_deref(),
        "createHeader",
        "headerId",
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
    apply_or_preview_created_segment_unified(
        config,
        store,
        account_override,
        command.document_id,
        change,
        command.text.as_deref(),
        "createHeader",
        "headerId",
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
    apply_or_preview_created_segment(
        client,
        command.document_id,
        change,
        command.text.as_deref(),
        "createFooter",
        "footerId",
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
    apply_or_preview_created_segment_unified(
        config,
        store,
        account_override,
        command.document_id,
        change,
        command.text.as_deref(),
        "createFooter",
        "footerId",
        command.dry_run,
        command.json,
        out,
        documents_url,
        state_path,
    )
    .await
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
async fn apply_or_preview_created_segment<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    change: PreparedDocsChange,
    text: Option<&str>,
    reply_key: &str,
    id_key: &str,
    dry_run: bool,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    if dry_run || text.is_none() {
        return apply_or_preview_docs_change(
            client,
            document_id,
            change,
            dry_run,
            json,
            out,
            documents_url,
        )
        .await;
    }

    let create_response =
        apply_docs_change_requests(client, document_id.clone(), change, documents_url).await?;
    let segment_id = created_segment_id(&create_response, reply_key, id_key)?;
    let request_body = segment_text_request_body(
        segment_id,
        text.expect("text presence checked above"),
        request_body_required_revision_id(&create_response).as_deref(),
    );
    let options = batch_update_document_options(document_id, request_body, documents_url);
    let text_response = batch_update_document(client, &options)
        .await
        .with_context(|| format!("failed to populate created {reply_key}"))?;
    let response = merge_batch_update_responses(create_response, text_response);
    write_json_line(
        out,
        &response,
        "failed to serialize Docs segment create response",
    )
}

#[allow(clippy::too_many_arguments)]
async fn apply_or_preview_created_segment_unified<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    change: PreparedDocsChange,
    text: Option<&str>,
    reply_key: &str,
    id_key: &str,
    dry_run: bool,
    json: bool,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    if dry_run || text.is_none() {
        return apply_or_preview_docs_change_unified(
            config,
            store,
            account_override,
            document_id,
            change,
            dry_run,
            json,
            out,
            documents_url,
            state_path,
        )
        .await;
    }

    let create_response = apply_docs_change_requests_unified(
        config,
        store,
        account_override,
        document_id.clone(),
        change,
        documents_url,
        state_path,
    )
    .await?;
    let segment_id = created_segment_id(&create_response, reply_key, id_key)?;
    let request_body = segment_text_request_body(
        segment_id,
        text.expect("text presence checked above"),
        request_body_required_revision_id(&create_response).as_deref(),
    );
    let options = batch_update_document_options(document_id.clone(), request_body, documents_url);
    let resource_key = resource_key("docs", &document_id);
    let text_response = run_with_docs_unified_access(
        config,
        store,
        account_override,
        &resource_key,
        DocsAccessAttempt::BatchUpdate(&options),
        state_path,
    )
    .await
    .with_context(|| format!("failed to populate created {reply_key}"))?;
    let response = merge_batch_update_responses(create_response, text_response);
    write_json_line(
        out,
        &response,
        "failed to serialize Docs segment create response",
    )
}

fn created_segment_id<'a>(
    response: &'a serde_json::Value,
    reply_key: &str,
    id_key: &str,
) -> Result<&'a str> {
    response["replies"][0][reply_key][id_key]
        .as_str()
        .with_context(|| format!("Google Docs did not return {id_key} after {reply_key}"))
}

fn segment_text_request_body(
    segment_id: &str,
    text: &str,
    required_revision_id: Option<&str>,
) -> serde_json::Value {
    let mut request_body = serde_json::json!({
        "requests": [{
            "insertText": {
                "endOfSegmentLocation": { "segmentId": segment_id },
                "text": text
            }
        }]
    });
    set_request_body_required_revision_id(&mut request_body, required_revision_id);
    request_body
}

fn merge_batch_update_responses(
    mut create_response: serde_json::Value,
    text_response: serde_json::Value,
) -> serde_json::Value {
    if let (Some(create_replies), Some(text_replies)) = (
        create_response["replies"].as_array_mut(),
        text_response["replies"].as_array(),
    ) {
        create_replies.extend(text_replies.iter().cloned());
    }
    if let Some(write_control) = text_response.get("writeControl") {
        create_response["writeControl"] = write_control.clone();
    }
    create_response
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
    let insert_index = change
        .location_index()
        .context("table insert did not resolve to a Google Docs index")?;
    let dry_run = command.dry_run;
    let no_auto_style = command.no_auto_style;
    let document_id = command.document_id.clone();
    let data = command.data.clone();
    let mut insert_output = Vec::new();
    if dry_run {
        apply_or_preview_docs_change(
            client,
            command.document_id,
            change,
            true,
            command.json,
            out,
            documents_url,
        )
        .await?;
    } else {
        apply_or_preview_docs_change(
            client,
            command.document_id,
            change,
            false,
            command.json,
            &mut insert_output,
            documents_url,
        )
        .await?;
    }

    if !dry_run {
        if let Some(data) = data {
            let document_map = get_document_map(client, document_id.clone(), documents_url).await?;
            let table_id = inserted_table_handle(&document_map, insert_index)?;
            let edit = prepare_edit_table_change(
                &document_map,
                &EditTableCommand {
                    document_id: document_id.clone(),
                    table_id,
                    data,
                    resize: false,
                    dry_run: false,
                    json: false,
                    required_revision_id: document_map.revision_id.clone(),
                },
            )?;
            apply_or_preview_docs_change(
                client,
                document_id.clone(),
                edit,
                false,
                false,
                &mut std::io::sink(),
                documents_url,
            )
            .await?;
        }
        out.write_all(&insert_output)
            .context("failed to write Google Docs table insert output")?;
    }

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
    let insert_index = change
        .location_index()
        .context("table insert did not resolve to a Google Docs index")?;
    let dry_run = command.dry_run;
    let no_auto_style = command.no_auto_style;
    let document_id = command.document_id.clone();
    let data = command.data.clone();
    let mut insert_output = Vec::new();
    if dry_run {
        apply_or_preview_docs_change_unified(
            config,
            store,
            account_override,
            command.document_id,
            change,
            true,
            command.json,
            out,
            documents_url,
            state_path,
        )
        .await?;
    } else {
        apply_or_preview_docs_change_unified(
            config,
            store,
            account_override,
            command.document_id,
            change,
            false,
            command.json,
            &mut insert_output,
            documents_url,
            state_path,
        )
        .await?;
    }

    if !dry_run {
        if let Some(data) = data {
            let document_map = get_document_map_unified(
                config,
                store,
                account_override,
                document_id.clone(),
                documents_url,
                state_path,
            )
            .await?;
            let table_id = inserted_table_handle(&document_map, insert_index)?;
            let edit = prepare_edit_table_change(
                &document_map,
                &EditTableCommand {
                    document_id: document_id.clone(),
                    table_id,
                    data,
                    resize: false,
                    dry_run: false,
                    json: false,
                    required_revision_id: document_map.revision_id.clone(),
                },
            )?;
            apply_or_preview_docs_change_unified(
                config,
                store,
                account_override,
                document_id.clone(),
                edit,
                false,
                false,
                &mut std::io::sink(),
                documents_url,
                state_path,
            )
            .await?;
        }
        out.write_all(&insert_output)
            .context("failed to write Google Docs table insert output")?;
    }

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

pub(super) fn inserted_table_handle(
    document_map: &DocumentMap,
    insert_index: i64,
) -> Result<String> {
    let table_index = insert_index + 1;
    document_map
        .entries
        .iter()
        .find(|entry| {
            entry.kind == DocumentMapEntryKind::Table && entry.location.index == Some(table_index)
        })
        .and_then(|entry| entry.table_handle.clone())
        .with_context(|| format!("inserted table was not found at Google Docs index {table_index}"))
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
pub(super) async fn run_style_table_row_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: StyleTableRowCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_style_table_row_change(&document_map, &command)?;
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

pub(super) async fn run_style_table_row_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: StyleTableRowCommand,
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
    let change = prepare_style_table_row_change(&document_map, &command)?;
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
pub(super) async fn run_set_table_column_widths_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: SetTableColumnWidthsCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_set_table_column_widths_change(&document_map, &command)?;
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

pub(super) async fn run_set_table_column_widths_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: SetTableColumnWidthsCommand,
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
    let change = prepare_set_table_column_widths_change(&document_map, &command)?;
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
pub(super) async fn run_pin_table_header_rows_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    command: PinTableHeaderRowsCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
) -> Result<()> {
    let document_map = get_document_map(client, command.document_id.clone(), documents_url).await?;
    let change = prepare_pin_table_header_rows_change(&document_map, &command)?;
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

pub(super) async fn run_pin_table_header_rows_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: PinTableHeaderRowsCommand,
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
    let change = prepare_pin_table_header_rows_change(&document_map, &command)?;
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

pub(super) async fn run_configure_page_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: ConfigurePageCommand,
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
    let change = prepare_configure_page_change(&document_map, &command)?;
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

pub(super) async fn run_update_named_style_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: UpdateNamedStyleCommand,
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
    let change = prepare_update_named_style_change(&document_map, &command)?;
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

pub(super) async fn run_copy_named_styles_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CopyNamedStylesCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let source_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.source_document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let target_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.target_document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_copy_named_styles_change(&source_map, &target_map, &command)?;
    apply_or_preview_docs_change_unified(
        config,
        store,
        account_override,
        command.target_document_id,
        change,
        command.dry_run,
        command.json,
        out,
        documents_url,
        state_path,
    )
    .await
}

pub(super) async fn run_copy_page_style_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CopyPageStyleCommand,
    out: &mut impl Write,
    documents_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let source_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.source_document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let target_map = get_document_map_unified(
        config,
        store,
        account_override,
        command.target_document_id.clone(),
        documents_url,
        state_path,
    )
    .await?;
    let change = prepare_copy_page_style_change(&source_map, &target_map, &command)?;
    apply_or_preview_docs_change_unified(
        config,
        store,
        account_override,
        command.target_document_id,
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

pub(super) async fn run_copy_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    source_document_id: String,
    title: String,
    out: &mut impl Write,
    drive_files_url: Option<&str>,
) -> Result<()> {
    let mut options = CopyDocumentOptions::new(source_document_id, title);
    if let Some(drive_files_url) = drive_files_url {
        options = options.with_drive_files_url(drive_files_url);
    }

    let document = copy_document(client, &options)
        .await
        .context("failed to copy Google Docs Document")?;
    let document_id = document
        .get("id")
        .and_then(serde_json::Value::as_str)
        .context("Google Drive copy response did not include an id")?;

    writeln!(
        out,
        "{}\thttps://docs.google.com/document/d/{}/edit",
        document_id, document_id
    )
    .context("failed to write output")?;
    Ok(())
}

#[cfg(test)]
pub(super) async fn run_export_pdf_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    document_id: String,
    output: PathBuf,
    out: &mut impl Write,
    drive_files_url: Option<&str>,
) -> Result<()> {
    let mut options =
        ExportGoogleFileOptions::new(document_id, GoogleFileExportFormat::Pdf, &output);
    if let Some(drive_files_url) = drive_files_url {
        options = options.with_files_url(drive_files_url);
    }

    let exported = export_google_file(client, &options, |_| {})
        .await
        .map_err(with_pdf_export_context)?;
    writeln!(out, "{}\t{}", exported.path.display(), exported.bytes)
        .context("failed to write output")?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_export_pdf_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    document_id: String,
    output: PathBuf,
    out: &mut impl Write,
    drive_files_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let mut options =
        ExportGoogleFileOptions::new(&document_id, GoogleFileExportFormat::Pdf, &output);
    if let Some(drive_files_url) = drive_files_url {
        options = options.with_files_url(drive_files_url);
    }
    let target_resource_key = resource_key("docs", &document_id);
    let exported = UnifiedAccess::run(
        config,
        account_override,
        &target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, DownloadedFile, DriveError> {
            Box::pin(export_pdf_as_account(config, store, &options, account))
        },
        is_drive_target_access_failure,
    )
    .await
    .map_err(with_pdf_export_context)?;

    writeln!(out, "{}\t{}", exported.path.display(), exported.bytes)
        .context("failed to write output")?;
    Ok(())
}

async fn export_pdf_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    options: &ExportGoogleFileOptions,
    account: String,
) -> Result<DownloadedFile, DriveError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))?;
    export_google_file(&client, options, |_| {}).await
}

pub(super) fn with_pdf_export_context(error: DriveError) -> anyhow::Error {
    match error {
        DriveError::NotFound | DriveError::PermissionDenied => anyhow::Error::new(error).context(
            "failed to export Google Docs Document as PDF; confirm the selected account can access the document and that file and Workspace policies allow downloading, printing, and copying; use --account EMAIL when multiple accounts are authorized",
        ),
        error => anyhow::Error::new(error).context("failed to export Google Docs Document as PDF"),
    }
}

fn is_drive_target_access_failure(error: &DriveError) -> bool {
    matches!(error, DriveError::NotFound | DriveError::PermissionDenied)
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
        .context("failed to read Google Docs Document")?;
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
    .context("failed to read Google Docs Document")?;

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
        .context("failed to read Google Docs Document")?;
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
    .context("failed to read Google Docs Document")?;
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

fn optional_content_selector(
    index: Option<i64>,
    entry: Option<usize>,
    page: Option<usize>,
    line: Option<usize>,
    heading: Option<String>,
) -> Result<Option<ContentSelector>> {
    if index.is_none() && entry.is_none() && page.is_none() && line.is_none() && heading.is_none() {
        return Ok(None);
    }

    content_selector(index, entry, page, line, heading).map(Some)
}

pub(super) fn insert_text_selector(at: String) -> Result<InsertTextSelector> {
    parse_insert_at_selector(&at)
}

fn parse_insert_at_selector(selector: &str) -> Result<InsertTextSelector> {
    if let Some((page, line)) = selector.split_once(",line:") {
        let Some(page) = page.strip_prefix("page:") else {
            bail!("invalid --at selector {selector:?}; use page:P,line:L");
        };
        return Ok(InsertTextSelector::PageLine {
            page: parse_insert_at_number(page, "page")?,
            line: parse_insert_at_number(line, "line")?,
        });
    }

    let Some((kind, value)) = selector.split_once(':') else {
        bail!("invalid --at selector {selector:?}; expected kind:value");
    };
    let value = trim_insert_at_value(value);
    match kind {
        "index" => Ok(InsertTextSelector::Index(parse_insert_at_number(
            value, "index",
        )?)),
        "entry" => Ok(InsertTextSelector::Entry(parse_insert_at_number(
            value, "entry",
        )?)),
        "heading" | "after-heading" => Ok(InsertTextSelector::AfterHeading(value.into())),
        "before-heading" => Ok(InsertTextSelector::BeforeHeading(value.into())),
        "after-text" => Ok(InsertTextSelector::AfterText(value.into())),
        "before-text" => Ok(InsertTextSelector::BeforeText(value.into())),
        _ => bail!(
            "invalid --at selector kind {kind:?}; expected index, entry, page, heading, after-heading, before-heading, after-text, or before-text"
        ),
    }
}

fn parse_insert_at_number<T>(value: &str, label: &str) -> Result<T>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    value
        .parse()
        .map_err(|error| anyhow::anyhow!("invalid {label} value in --at selector: {error}"))
}

fn trim_insert_at_value(value: &str) -> &str {
    value
        .strip_prefix('"')
        .and_then(|value| value.strip_suffix('"'))
        .or_else(|| {
            value
                .strip_prefix('\'')
                .and_then(|value| value.strip_suffix('\''))
        })
        .unwrap_or(value)
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
        document_styles: document_map.document_styles.clone(),
        named_styles: document_map.named_styles.clone(),
        breaks: Vec::new(),
        blank_paragraphs: Vec::new(),
        segments: Vec::new(),
        lists: Vec::new(),
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

fn write_document_map(
    out: &mut impl Write,
    document_map: &DocumentMap,
    map_type: DocsMapType,
    json: bool,
) -> Result<()> {
    if map_type == DocsMapType::Segments {
        return write_document_segments(out, &document_map.segments, json);
    }
    if map_type == DocsMapType::Breaks {
        return write_document_breaks(out, &document_map.breaks, json);
    }
    if map_type == DocsMapType::Lists {
        return write_document_lists(out, &document_map.lists, json);
    }
    if let Some(kinds) = map_type_entry_kinds(map_type) {
        write_filtered_entries(out, document_map, kinds, json)
    } else if json {
        write_json_line(out, document_map, "failed to serialize Docs Document Map")
    } else {
        write_document_map_table(out, document_map)?;
        if !document_map.breaks.is_empty() {
            writeln!(out).context("failed to separate Docs break map")?;
            write_document_breaks(out, &document_map.breaks, false)?;
        }
        if !document_map.lists.is_empty() {
            writeln!(out).context("failed to separate Docs list map")?;
            write_document_lists_table(out, &document_map.lists)?;
        }
        if !document_map.segments.is_empty() {
            writeln!(out).context("failed to separate Docs segment map")?;
            write_document_segments_table(out, &document_map.segments)?;
        }
        Ok(())
    }
}

fn map_type_entry_kinds(map_type: DocsMapType) -> Option<&'static [DocumentMapEntryKind]> {
    match map_type {
        DocsMapType::All => None,
        DocsMapType::Images => Some(&[
            DocumentMapEntryKind::InlineImage,
            DocumentMapEntryKind::PositionedImage,
        ]),
        DocsMapType::Tables => Some(&[DocumentMapEntryKind::Table]),
        DocsMapType::Breaks | DocsMapType::Lists | DocsMapType::Segments => {
            unreachable!("list and segment maps are handled separately")
        }
    }
}

fn write_document_breaks(out: &mut impl Write, breaks: &[DocumentBreak], json: bool) -> Result<()> {
    if json {
        write_json_line(out, breaks, "failed to serialize Docs breaks")
    } else {
        writeln!(
            out,
            "{:<15} {:<7} {:<5} {:<16} {:<30} Preview",
            "Break", "Index", "Page", "Section type", "Header/footer"
        )
        .context("failed to write Docs break map header")?;
        for document_break in breaks {
            writeln!(
                out,
                "{:<15} {:<7} {:<5} {:<16} {:<30} {}",
                format!("{:?}", document_break.kind),
                display_optional(document_break.location.index),
                display_optional(document_break.location.page),
                document_break.section_type.as_deref().unwrap_or("-"),
                section_header_footer_summary(document_break.section_style.as_ref()),
                document_break.preview
            )
            .context("failed to write Docs break map row")?;
        }
        Ok(())
    }
}

fn section_header_footer_summary(section_style: Option<&serde_json::Value>) -> String {
    let Some(section_style) = section_style else {
        return "-".into();
    };
    let references = [
        ("defaultHeaderId", "header"),
        ("defaultFooterId", "footer"),
        ("firstPageHeaderId", "first-header"),
        ("firstPageFooterId", "first-footer"),
        ("evenPageHeaderId", "even-header"),
        ("evenPageFooterId", "even-footer"),
    ]
    .into_iter()
    .filter_map(|(field, label)| {
        section_style
            .get(field)
            .and_then(serde_json::Value::as_str)
            .map(|id| format!("{label}:{id}"))
    })
    .collect::<Vec<_>>();

    if references.is_empty() {
        "-".into()
    } else {
        references.join(",")
    }
}

fn write_document_lists(out: &mut impl Write, lists: &[DocumentList], json: bool) -> Result<()> {
    if json {
        write_json_line(out, lists, "failed to serialize Docs lists")
    } else {
        write_document_lists_table(out, lists)
    }
}

fn write_document_lists_table(out: &mut impl Write, lists: &[DocumentList]) -> Result<()> {
    writeln!(
        out,
        "{:<24} {:<7} {:<7} {:<6} {:<8} {:<18} Preview",
        "List", "Start", "End", "Items", "Levels", "Glyphs"
    )
    .context("failed to write Docs list map header")?;

    for list in lists {
        let levels = list
            .nesting_levels
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let glyphs = list
            .glyphs
            .iter()
            .map(|glyph| {
                glyph
                    .glyph_symbol
                    .as_deref()
                    .or(glyph.glyph_format.as_deref())
                    .or(glyph.glyph_type.as_deref())
                    .unwrap_or("-")
            })
            .collect::<Vec<_>>()
            .join(",");
        writeln!(
            out,
            "{:<24} {:<7} {:<7} {:<6} {:<8} {:<18} {}",
            list.list_id,
            display_optional(list.start_index),
            display_optional(list.end_index),
            list.item_count,
            levels,
            glyphs,
            list.preview
        )
        .context("failed to write Docs list map row")?;
    }

    Ok(())
}

fn write_document_segments(
    out: &mut impl Write,
    segments: &[DocumentSegment],
    json: bool,
) -> Result<()> {
    if json {
        write_json_line(out, segments, "failed to serialize Docs segments")
    } else {
        write_document_segments_table(out, segments)
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct CompareDocumentsCommand {
    pub(super) source_document_id: String,
    pub(super) target_document_id: String,
    pub(super) json: bool,
    pub(super) scope: DocsCompareScope,
    pub(super) fail_on_difference: bool,
    pub(super) max_differences: u32,
    pub(super) summary_only: bool,
    pub(super) difference_pattern: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentComparisonReport {
    source_document_id: Option<String>,
    target_document_id: Option<String>,
    summary_only: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    difference_preview_pattern: Option<String>,
    matches: bool,
    total_difference_count: usize,
    total_displayed_difference_count: usize,
    total_difference_count_hidden_by_limit: usize,
    total_difference_count_hidden_by_summary: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_preview_difference_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    total_difference_count_outside_preview: Option<usize>,
    scopes: Vec<DocumentComparisonScope>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentComparisonScope {
    scope: &'static str,
    matches: bool,
    source_fingerprint: String,
    target_fingerprint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    source_inventory: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    target_inventory: Option<serde_json::Value>,
    difference_count: usize,
    displayed_difference_count: usize,
    difference_count_hidden_by_limit: usize,
    difference_count_hidden_by_summary: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    preview_difference_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    difference_count_outside_preview: Option<usize>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    difference_patterns: Vec<DocumentComparisonDifferencePattern>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    differences: Vec<DocumentComparisonDifference>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct DocumentComparisonDifferencePattern {
    path: String,
    count: usize,
    example: DocumentComparisonDifference,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct DocumentComparisonDifference {
    pub(super) path: String,
    pub(super) source: String,
    pub(super) target: String,
}

#[derive(Clone, Copy)]
pub(super) struct DocumentComparisonPreview<'a> {
    pub(super) max_differences: usize,
    pub(super) summary_only: bool,
    pub(super) difference_pattern: Option<&'a str>,
}

pub(super) fn write_document_comparison(
    out: &mut impl Write,
    source_map: &DocumentMap,
    target_map: &DocumentMap,
    json: bool,
    scope: DocsCompareScope,
    fail_on_difference: bool,
    preview: DocumentComparisonPreview<'_>,
) -> Result<()> {
    let source_inventory = document_inventory(source_map);
    let target_inventory = document_inventory(target_map);
    let source_visual_system = canonical_visual_system(source_map)?;
    let target_visual_system = canonical_visual_system(target_map)?;
    let source_formatting = canonical_formatting(source_map)?;
    let target_formatting = canonical_formatting(target_map)?;
    let source_content = canonical_content(source_map)?;
    let target_content = canonical_content(target_map)?;

    let mut scopes = Vec::new();
    if matches!(scope, DocsCompareScope::All | DocsCompareScope::Inventory) {
        scopes.push(comparison_scope(
            "inventory",
            source_inventory,
            target_inventory,
            true,
            preview,
        )?);
    }
    if matches!(
        scope,
        DocsCompareScope::All | DocsCompareScope::VisualSystem
    ) {
        scopes.push(comparison_scope(
            "visual-system",
            source_visual_system,
            target_visual_system,
            false,
            preview,
        )?);
    }
    if matches!(scope, DocsCompareScope::All | DocsCompareScope::Formatting) {
        scopes.push(comparison_scope(
            "formatting",
            source_formatting,
            target_formatting,
            false,
            preview,
        )?);
    }
    if matches!(scope, DocsCompareScope::All | DocsCompareScope::Content) {
        scopes.push(comparison_scope(
            "content",
            source_content,
            target_content,
            false,
            preview,
        )?);
    }
    let total_difference_count = scopes.iter().map(|scope| scope.difference_count).sum();
    let total_displayed_difference_count = scopes
        .iter()
        .map(|scope| scope.displayed_difference_count)
        .sum();
    let total_difference_count_hidden_by_limit = scopes
        .iter()
        .map(|scope| scope.difference_count_hidden_by_limit)
        .sum();
    let total_difference_count_hidden_by_summary = scopes
        .iter()
        .map(|scope| scope.difference_count_hidden_by_summary)
        .sum();
    let total_preview_difference_count = preview.difference_pattern.map(|_| {
        scopes
            .iter()
            .map(|scope| scope.preview_difference_count.unwrap_or(0))
            .sum()
    });
    let total_difference_count_outside_preview = preview.difference_pattern.map(|_| {
        scopes
            .iter()
            .map(|scope| scope.difference_count_outside_preview.unwrap_or(0))
            .sum()
    });
    let mut report = DocumentComparisonReport {
        source_document_id: source_map.document_id.clone(),
        target_document_id: target_map.document_id.clone(),
        summary_only: preview.summary_only,
        difference_preview_pattern: preview.difference_pattern.map(str::to_owned),
        matches: scopes.iter().all(|scope| scope.matches),
        total_difference_count,
        total_displayed_difference_count,
        total_difference_count_hidden_by_limit,
        total_difference_count_hidden_by_summary,
        total_preview_difference_count,
        total_difference_count_outside_preview,
        scopes,
    };

    if let Some(pattern) = preview.difference_pattern {
        let pattern_is_reported = report.scopes.iter().any(|scope| {
            scope
                .difference_patterns
                .iter()
                .any(|reported| reported.path == pattern)
        });
        if !pattern_is_reported {
            let suggestions = closest_difference_patterns(&report, pattern, 3);
            let suggestion = if suggestions.is_empty() {
                String::new()
            } else {
                format!(
                    " Closest reported {}: {}.",
                    if suggestions.len() == 1 {
                        "pattern"
                    } else {
                        "patterns"
                    },
                    suggestions
                        .iter()
                        .map(|suggestion| format!("`{suggestion}`"))
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            };
            bail!(
                "difference pattern `{pattern}` was not found in the selected comparison scope.{suggestion} Run without --difference-pattern to list all reported patterns"
            );
        }
    }

    if preview.summary_only {
        for scope in &mut report.scopes {
            scope.difference_patterns.clear();
            scope.differences.clear();
        }
    }

    if json {
        write_json_line(out, &report, "failed to serialize Docs comparison")?;
    } else {
        writeln!(
            out,
            "{:<16} {:<7} {:<64} Target SHA-256",
            "Scope", "Match", "Source SHA-256"
        )
        .context("failed to write Docs comparison header")?;
        if let Some(pattern) = preview.difference_pattern {
            writeln!(out, "Difference preview filter: {pattern}")
                .context("failed to write Docs comparison difference filter")?;
        }
        for scope in &report.scopes {
            writeln!(
                out,
                "{:<16} {:<7} {:<64} {}",
                scope.scope,
                if scope.matches { "yes" } else { "no" },
                scope.source_fingerprint,
                scope.target_fingerprint
            )
            .context("failed to write Docs comparison row")?;
            for pattern in &scope.difference_patterns {
                writeln!(out, "  Pattern ({}): {}", pattern.count, pattern.path)
                    .context("failed to write Docs comparison difference pattern")?;
                writeln!(
                    out,
                    "    Example {}: source={}, target={}",
                    pattern.example.path, pattern.example.source, pattern.example.target
                )
                .context("failed to write Docs comparison difference pattern example")?;
            }
            for difference in &scope.differences {
                writeln!(
                    out,
                    "  {}: source={}, target={}",
                    difference.path, difference.source, difference.target
                )
                .context("failed to write Docs comparison difference")?;
            }
            let preview_difference_count = scope
                .preview_difference_count
                .unwrap_or(scope.difference_count);
            if preview_difference_count > scope.differences.len() {
                let hidden_count = preview_difference_count - scope.differences.len();
                writeln!(
                    out,
                    "  ... {hidden_count} more difference{}{}",
                    if hidden_count == 1 { "" } else { "s" },
                    if preview.difference_pattern.is_some() {
                        " matching the preview filter"
                    } else {
                        ""
                    }
                )
                .context("failed to write Docs comparison difference count")?;
            }
            if preview.difference_pattern.is_some()
                && scope.difference_count > preview_difference_count
            {
                let outside_count = scope.difference_count - preview_difference_count;
                writeln!(
                    out,
                    "  {outside_count} difference{} outside the preview filter",
                    if outside_count == 1 { "" } else { "s" }
                )
                .context("failed to write Docs comparison filtered difference count")?;
            }
        }
        if let (Some(preview_count), Some(outside_count)) = (
            report.total_preview_difference_count,
            report.total_difference_count_outside_preview,
        ) {
            if report.summary_only {
                writeln!(
                    out,
                    "Difference totals: {} overall, {preview_count} matching filter ({} displayed, {} hidden by limit, {} hidden by summary mode), {outside_count} outside filter",
                    report.total_difference_count,
                    report.total_displayed_difference_count,
                    report.total_difference_count_hidden_by_limit,
                    report.total_difference_count_hidden_by_summary
                )
                .context("failed to write filtered Docs comparison totals")?;
            } else {
                writeln!(
                    out,
                    "Difference totals: {} overall, {preview_count} matching filter ({} displayed, {} hidden by limit), {outside_count} outside filter",
                    report.total_difference_count,
                    report.total_displayed_difference_count,
                    report.total_difference_count_hidden_by_limit
                )
                .context("failed to write filtered Docs comparison totals")?;
            }
        } else {
            if report.summary_only {
                writeln!(
                    out,
                    "Difference totals: {} overall ({} displayed, {} hidden by limit, {} hidden by summary mode)",
                    report.total_difference_count,
                    report.total_displayed_difference_count,
                    report.total_difference_count_hidden_by_limit,
                    report.total_difference_count_hidden_by_summary
                )
                .context("failed to write Docs comparison totals")?;
            } else {
                writeln!(
                    out,
                    "Difference totals: {} overall ({} displayed, {} hidden by limit)",
                    report.total_difference_count,
                    report.total_displayed_difference_count,
                    report.total_difference_count_hidden_by_limit
                )
                .context("failed to write Docs comparison totals")?;
            }
        }
        writeln!(
            out,
            "Overall: {}",
            if report.matches { "match" } else { "different" }
        )
        .context("failed to write Docs comparison result")?;
    }

    if fail_on_difference && !report.matches {
        bail!("Google Docs comparison found semantic differences");
    }
    Ok(())
}

fn closest_difference_patterns(
    report: &DocumentComparisonReport,
    requested: &str,
    limit: usize,
) -> Vec<String> {
    let mut patterns = report
        .scopes
        .iter()
        .flat_map(|scope| &scope.difference_patterns)
        .map(|pattern| pattern.path.as_str())
        .collect::<Vec<_>>();
    patterns.sort_by(|left, right| {
        levenshtein_distance(requested, left)
            .cmp(&levenshtein_distance(requested, right))
            .then_with(|| left.cmp(right))
    });
    patterns.dedup();
    patterns
        .into_iter()
        .take(limit)
        .map(str::to_owned)
        .collect()
}

fn levenshtein_distance(left: &str, right: &str) -> usize {
    let right = right.chars().collect::<Vec<_>>();
    let mut previous = (0..=right.len()).collect::<Vec<_>>();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right.iter().enumerate() {
            current.push(
                (previous[right_index + 1] + 1)
                    .min(current[right_index] + 1)
                    .min(previous[right_index] + usize::from(left_char != *right_char)),
            );
        }
        previous = current;
    }
    previous[right.len()]
}

fn comparison_scope(
    scope: &'static str,
    source: serde_json::Value,
    target: serde_json::Value,
    include_inventory: bool,
    preview: DocumentComparisonPreview<'_>,
) -> Result<DocumentComparisonScope> {
    let matches = source == target;
    let mut differences = Vec::new();
    let mut difference_patterns = std::collections::BTreeMap::new();
    let difference_count = collect_json_differences_with_patterns(
        "",
        "",
        Some(&source),
        Some(&target),
        &mut differences,
        preview,
        &mut difference_patterns,
    );
    let mut difference_patterns = difference_patterns
        .into_iter()
        .map(
            |(path, (count, example))| DocumentComparisonDifferencePattern {
                path,
                count,
                example,
            },
        )
        .collect::<Vec<_>>();
    difference_patterns.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.path.cmp(&right.path))
    });
    let preview_difference_count = preview.difference_pattern.map(|filter| {
        difference_patterns
            .iter()
            .find(|pattern| pattern.path == filter)
            .map(|pattern| pattern.count)
            .unwrap_or(0)
    });
    let difference_count_outside_preview =
        preview_difference_count.map(|preview_count| difference_count - preview_count);
    if preview.summary_only {
        differences.clear();
    }
    let displayed_difference_count = differences.len();
    let displayable_difference_count = preview_difference_count.unwrap_or(difference_count);
    let difference_count_hidden_by_summary = if preview.summary_only {
        displayable_difference_count
    } else {
        0
    };
    let difference_count_hidden_by_limit = if preview.summary_only {
        0
    } else {
        displayable_difference_count.saturating_sub(displayed_difference_count)
    };
    Ok(DocumentComparisonScope {
        scope,
        matches,
        source_fingerprint: json_fingerprint(&source)?,
        target_fingerprint: json_fingerprint(&target)?,
        source_inventory: include_inventory.then_some(source),
        target_inventory: include_inventory.then_some(target),
        difference_count,
        displayed_difference_count,
        difference_count_hidden_by_limit,
        difference_count_hidden_by_summary,
        preview_difference_count,
        difference_count_outside_preview,
        difference_patterns,
        differences,
    })
}

#[cfg(test)]
pub(super) fn collect_json_differences(
    path: &str,
    source: Option<&serde_json::Value>,
    target: Option<&serde_json::Value>,
    differences: &mut Vec<DocumentComparisonDifference>,
    max_differences: usize,
) -> usize {
    let preview = DocumentComparisonPreview {
        max_differences,
        summary_only: false,
        difference_pattern: None,
    };
    collect_json_differences_with_patterns(
        path,
        path,
        source,
        target,
        differences,
        preview,
        &mut std::collections::BTreeMap::new(),
    )
}

fn collect_json_differences_with_patterns(
    path: &str,
    pattern_path: &str,
    source: Option<&serde_json::Value>,
    target: Option<&serde_json::Value>,
    differences: &mut Vec<DocumentComparisonDifference>,
    preview: DocumentComparisonPreview<'_>,
    difference_patterns: &mut std::collections::BTreeMap<
        String,
        (usize, DocumentComparisonDifference),
    >,
) -> usize {
    match (source, target) {
        (Some(serde_json::Value::Object(source)), Some(serde_json::Value::Object(target))) => {
            let keys = source
                .keys()
                .chain(target.keys())
                .collect::<std::collections::BTreeSet<_>>();
            keys.into_iter()
                .map(|key| {
                    let token = escape_json_pointer_token(key);
                    collect_json_differences_with_patterns(
                        &format!("{}/{}", path, escape_json_pointer_token(key)),
                        &format!("{pattern_path}/{token}"),
                        source.get(key),
                        target.get(key),
                        differences,
                        preview,
                        difference_patterns,
                    )
                })
                .sum()
        }
        (Some(serde_json::Value::Array(source)), Some(serde_json::Value::Array(target))) => (0
            ..source.len().max(target.len()))
            .map(|index| {
                collect_json_differences_with_patterns(
                    &format!("{path}/{index}"),
                    &format!("{pattern_path}/*"),
                    source.get(index),
                    target.get(index),
                    differences,
                    preview,
                    difference_patterns,
                )
            })
            .sum(),
        (source, target) if source == target => 0,
        (source, target) => {
            let difference = DocumentComparisonDifference {
                path: if path.is_empty() {
                    "/".to_owned()
                } else {
                    path.to_owned()
                },
                source: summarize_json_value(source),
                target: summarize_json_value(target),
            };
            let pattern_path = if pattern_path.is_empty() {
                "/".to_owned()
            } else {
                pattern_path.to_owned()
            };
            let include_preview = preview
                .difference_pattern
                .is_none_or(|filter| filter == pattern_path);
            difference_patterns
                .entry(pattern_path)
                .and_modify(|(count, _)| *count += 1)
                .or_insert_with(|| (1, difference.clone()));
            if include_preview && differences.len() < preview.max_differences {
                differences.push(difference);
            }
            1
        }
    }
}

fn escape_json_pointer_token(token: &str) -> String {
    token.replace('~', "~0").replace('/', "~1")
}

fn summarize_json_value(value: Option<&serde_json::Value>) -> String {
    match value {
        None => "<missing>".to_owned(),
        Some(value @ serde_json::Value::Array(values)) => {
            summarize_json_container(value, format!("<array: {} items>", values.len()))
        }
        Some(value @ serde_json::Value::Object(values)) => {
            summarize_json_container(value, format!("<object: {} fields>", values.len()))
        }
        Some(value) => value.to_string(),
    }
}

fn summarize_json_container(value: &serde_json::Value, fallback: String) -> String {
    let serialized = value.to_string();
    if serialized.chars().count() <= 160 {
        serialized
    } else {
        fallback
    }
}

fn document_inventory(document_map: &DocumentMap) -> serde_json::Value {
    let mut entry_counts = std::collections::BTreeMap::<String, usize>::new();
    for entry in &document_map.entries {
        let kind = serde_json::to_value(entry.kind)
            .ok()
            .and_then(|value| value.as_str().map(str::to_owned))
            .unwrap_or_else(|| format!("{:?}", entry.kind));
        *entry_counts.entry(kind).or_default() += 1;
    }

    serde_json::json!({
        "entriesByKind": entry_counts,
        "lists": document_map.lists.len(),
        "breaks": document_map.breaks.len(),
        "segments": document_map.segments.len(),
        "blankParagraphs": document_map.blank_paragraphs.len(),
        "namedStyleTabs": document_map.named_styles.len(),
        "documentStyleTabs": document_map.document_styles.len(),
    })
}

fn canonical_visual_system(document_map: &DocumentMap) -> Result<serde_json::Value> {
    let mut value = serde_json::json!({
        "namedStyles": document_map.named_styles,
        "documentStyles": document_map.document_styles,
    });
    remove_generated_ids(&mut value);
    remove_materialized_visual_defaults(&mut value);
    Ok(value)
}

fn remove_materialized_visual_defaults(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                remove_materialized_visual_defaults(value);
            }
        }
        serde_json::Value::Object(object) => {
            if object.get("pageNumberStart") == Some(&serde_json::json!(1)) {
                object.remove("pageNumberStart");
            }
            if let Some(serde_json::Value::Object(paragraph_style)) =
                object.get_mut("paragraphStyle")
            {
                paragraph_style.remove("namedStyleType");
                if paragraph_style.get("pageBreakBefore") == Some(&serde_json::Value::Bool(false)) {
                    paragraph_style.remove("pageBreakBefore");
                }
            }
            if let Some(serde_json::Value::Object(text_style)) = object.get_mut("textStyle") {
                if text_style.get("bold") == Some(&serde_json::Value::Bool(false)) {
                    text_style.remove("bold");
                }
            }
            for child in object.values_mut() {
                remove_materialized_visual_defaults(child);
            }
        }
        _ => {}
    }
}

fn canonical_content(document_map: &DocumentMap) -> Result<serde_json::Value> {
    let mut value = serde_json::json!({
        "entries": document_map.entries,
        "blankParagraphs": document_map.blank_paragraphs,
        "lists": document_map.lists,
        "breaks": document_map.breaks,
        "segments": document_map.segments,
        "documentLocations": document_map.document_locations,
    });
    remove_generated_ids(&mut value);
    Ok(value)
}

fn canonical_formatting(document_map: &DocumentMap) -> Result<serde_json::Value> {
    let mut value = canonical_content(document_map)?;
    remove_redundant_inherited_fonts(&mut value, document_map);
    remove_content_and_positions(&mut value);
    remove_materialized_visual_defaults(&mut value);
    remove_empty_text_runs(&mut value);
    Ok(value)
}

fn remove_redundant_inherited_fonts(value: &mut serde_json::Value, document_map: &DocumentMap) {
    let Some(entries) = value
        .get_mut("entries")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };

    for (entry, mapped_entry) in entries.iter_mut().zip(&document_map.entries) {
        let style = mapped_entry.style.as_deref().unwrap_or("NORMAL_TEXT");
        let Some(inherited_font) = consensus_inherited_font(document_map, style) else {
            continue;
        };
        let Some(text_runs) = entry
            .get_mut("textRuns")
            .and_then(serde_json::Value::as_array_mut)
        else {
            continue;
        };
        for text_run in text_runs {
            let Some(text_style) = text_run
                .get_mut("textStyle")
                .and_then(serde_json::Value::as_object_mut)
            else {
                continue;
            };
            if text_style.get("weightedFontFamily") == Some(&inherited_font) {
                text_style.remove("weightedFontFamily");
            }
        }
    }
}

fn consensus_inherited_font(document_map: &DocumentMap, style: &str) -> Option<serde_json::Value> {
    let fonts = document_map
        .named_styles
        .iter()
        .map(|tab| inherited_font_for_style(&tab.named_styles, style))
        .collect::<Option<Vec<_>>>()?;
    let first = fonts.first()?.clone();
    fonts.iter().all(|font| font == &first).then_some(first)
}

fn inherited_font_for_style(
    named_styles: &serde_json::Value,
    style: &str,
) -> Option<serde_json::Value> {
    let styles = named_styles.get("styles")?.as_array()?;
    let style_font = |name: &str| {
        styles
            .iter()
            .find(|candidate| {
                candidate
                    .get("namedStyleType")
                    .and_then(serde_json::Value::as_str)
                    == Some(name)
            })
            .and_then(|candidate| candidate.get("textStyle"))
            .and_then(|text_style| text_style.get("weightedFontFamily"))
            .cloned()
    };
    style_font(style).or_else(|| style_font("NORMAL_TEXT"))
}

fn remove_content_and_positions(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                remove_content_and_positions(value);
            }
        }
        serde_json::Value::Object(object) => {
            for field in [
                "content",
                "preview",
                "location",
                "startIndex",
                "endIndex",
                "entry",
                "imageAltText",
                "imageHandle",
                "tableHandle",
                "documentLocations",
            ] {
                object.remove(field);
            }
            for child in object.values_mut() {
                remove_content_and_positions(child);
            }
        }
        _ => {}
    }
}

fn remove_empty_text_runs(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                remove_empty_text_runs(value);
            }
        }
        serde_json::Value::Object(object) => {
            for child in object.values_mut() {
                remove_empty_text_runs(child);
            }
            for field in ["textStyle", "paragraphStyle"] {
                if object
                    .get(field)
                    .and_then(serde_json::Value::as_object)
                    .is_some_and(serde_json::Map::is_empty)
                {
                    object.remove(field);
                }
            }
            if let Some(serde_json::Value::Array(text_runs)) = object.get_mut("textRuns") {
                text_runs.retain(|text_run| {
                    text_run
                        .as_object()
                        .is_none_or(|text_run| !text_run.is_empty())
                });
                text_runs.dedup();
            }
        }
        _ => {}
    }
}

fn remove_generated_ids(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                remove_generated_ids(value);
            }
        }
        serde_json::Value::Object(object) => {
            for field in [
                "objectId",
                "headingId",
                "segmentId",
                "listId",
                "defaultHeaderId",
                "defaultFooterId",
                "firstPageHeaderId",
                "firstPageFooterId",
                "evenPageHeaderId",
                "evenPageFooterId",
            ] {
                object.remove(field);
            }
            if let Some(serde_json::Value::Object(heading)) = object.get_mut("heading") {
                heading.remove("id");
            }
            for child in object.values_mut() {
                remove_generated_ids(child);
            }
        }
        _ => {}
    }
}

fn json_fingerprint(value: &serde_json::Value) -> Result<String> {
    let bytes = serde_json::to_vec(value).context("failed to serialize Docs comparison scope")?;
    let digest = Sha256::digest(bytes);
    Ok(digest.iter().map(|byte| format!("{byte:02x}")).collect())
}

fn write_document_entries_table(out: &mut impl Write, entries: &[DocumentMapEntry]) -> Result<()> {
    writeln!(
        out,
        "{:<5} {:<7} {:<5} {:<4} {:<20} {:<16} {:<10} {:<10} {:<18} {:<15} {:<20} Preview",
        "Entry",
        "Index",
        "Page",
        "Line",
        "Kind",
        "Handle",
        "Object",
        "Size",
        "Style",
        "Confidence",
        "Image alt text"
    )
    .context("failed to write Docs Document Map header")?;

    for entry in entries {
        let style = entry.style.as_deref().unwrap_or("-");
        let target = entry
            .heading_id
            .as_deref()
            .or(entry.image_handle.as_deref())
            .or(entry.table_handle.as_deref())
            .unwrap_or("-");
        let object = entry.object_id.as_deref().unwrap_or("-");
        let size = match (entry.rows, entry.columns) {
            (Some(rows), Some(columns)) => format!("{rows}x{columns}"),
            _ => "-".into(),
        };
        let image_alt_text = entry
            .image_alt_text
            .as_ref()
            .map(|alt_text| match (&alt_text.title, &alt_text.description) {
                (Some(title), Some(description)) => {
                    format!("title: {title}; description: {description}")
                }
                (Some(title), None) => title.clone(),
                (None, Some(description)) => description.clone(),
                (None, None) => "-".into(),
            })
            .unwrap_or_else(|| "-".into());
        writeln!(
            out,
            "{:<5} {:<7} {:<5} {:<4} {:<20} {:<16} {:<10} {:<10} {:<18} {:<15} {:<20} {}",
            entry.entry,
            display_optional(entry.location.index),
            display_optional(entry.location.page),
            entry.location.content_line,
            format!("{:?}", entry.kind),
            target,
            object,
            size,
            style,
            format!("{:?}", entry.location.confidence),
            image_alt_text,
            entry.preview
        )
        .context("failed to write Docs Document Map row")?;
    }

    Ok(())
}

fn write_document_segments_table(out: &mut impl Write, segments: &[DocumentSegment]) -> Result<()> {
    writeln!(
        out,
        "{:<8} {:<24} {:<6} {:<6} {:<18} Preview",
        "Kind", "Segment", "Start", "End", "Auto text"
    )
    .context("failed to write Docs segment map header")?;

    for segment in segments {
        let auto_text = if segment.auto_text_types.is_empty() {
            "-".into()
        } else {
            segment.auto_text_types.join(",")
        };
        writeln!(
            out,
            "{:<8} {:<24} {:<6} {:<6} {:<18} {}",
            format!("{:?}", segment.kind),
            segment.segment_id,
            segment.start_index,
            segment.end_index,
            auto_text,
            segment.preview
        )
        .context("failed to write Docs segment map row")?;
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
