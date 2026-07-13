use std::io::Write;

use anyhow::{bail, Context, Result};
use serde::Serialize;

use crate::cli::{
    DocsListType, DocsParagraphAlignment, DocsParagraphDirection, DocsSectionBreakType,
    DocsTableCellAlignment,
};
use crate::docs::map::{
    resolve_insert_text_location, resolve_range_selector, resolve_replace_text_ranges,
    text_block_contains_range, DocumentLocation, DocumentMap, DocumentMapEntry,
    DocumentMapEntryKind, DocumentRange, InsertTextSelector, RangeSelector,
};
use crate::docs::style_template::StyleTemplate;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InsertTextCommand {
    pub document_id: String,
    pub text: String,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ReplaceTextCommand {
    pub document_id: String,
    pub old_text: String,
    pub new_text: String,
    pub match_number: Option<usize>,
    pub all: bool,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct InsertImageCommand {
    pub document_id: String,
    pub image_uri: String,
    pub selector: Option<InsertTextSelector>,
    pub segment_id: Option<String>,
    pub width: Option<f64>,
    pub height: Option<f64>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InsertPageBreakCommand {
    pub document_id: String,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InsertSectionBreakCommand {
    pub document_id: String,
    pub section_type: DocsSectionBreakType,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateHeaderCommand {
    pub document_id: String,
    pub text: Option<String>,
    pub section_break_index: Option<usize>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateFooterCommand {
    pub document_id: String,
    pub text: Option<String>,
    pub section_break_index: Option<usize>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateFootnoteCommand {
    pub document_id: String,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InsertTableCommand {
    pub document_id: String,
    pub data: Option<String>,
    pub rows: Option<usize>,
    pub columns: Option<usize>,
    pub selector: InsertTextSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
    pub no_auto_style: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct EditTableCommand {
    pub document_id: String,
    pub table_id: String,
    pub data: String,
    pub resize: bool,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct StyleTableRowCommand {
    pub document_id: String,
    pub table_id: String,
    pub row: usize,
    pub column: Option<usize>,
    pub background_color: Option<String>,
    pub content_alignment: Option<DocsTableCellAlignment>,
    pub border_color: Option<String>,
    pub border_width: Option<f64>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SetTableColumnWidthsCommand {
    pub document_id: String,
    pub table_id: String,
    pub widths: Vec<f64>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PinTableHeaderRowsCommand {
    pub document_id: String,
    pub table_id: String,
    pub rows: usize,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ApplyStylesCommand {
    pub document_id: String,
    pub selector: RangeSelector,
    pub segment_id: Option<String>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub font_size: Option<f64>,
    pub font_family: Option<String>,
    pub foreground_color: Option<String>,
    pub link_heading_id: Option<String>,
    pub alignment: Option<DocsParagraphAlignment>,
    pub direction: Option<DocsParagraphDirection>,
    pub space_above: Option<f64>,
    pub space_below: Option<f64>,
    pub line_spacing: Option<f64>,
    pub spacing_mode: Option<crate::cli::DocsParagraphSpacingMode>,
    pub indent_start: Option<f64>,
    pub indent_end: Option<f64>,
    pub indent_first_line: Option<f64>,
    pub keep_with_next: bool,
    pub keep_lines_together: bool,
    pub avoid_widow_and_orphan: bool,
    pub page_break_before: bool,
    pub heading: Option<String>,
    pub style_json: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
    pub no_auto_style: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UpdateNamedStyleCommand {
    pub document_id: String,
    pub named_style: String,
    pub style_json: String,
    pub tab_id: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyNamedStylesCommand {
    pub source_document_id: String,
    pub target_document_id: String,
    pub source_tab_id: Option<String>,
    pub target_tab_id: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CopyPageStyleCommand {
    pub source_document_id: String,
    pub target_document_id: String,
    pub source_tab_id: Option<String>,
    pub target_tab_id: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ApplyListCommand {
    pub document_id: String,
    pub selector: RangeSelector,
    pub list_type: Option<DocsListType>,
    pub preset: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
    pub no_auto_style: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ConfigurePageCommand {
    pub document_id: String,
    pub page_width: Option<f64>,
    pub page_height: Option<f64>,
    pub margin_top: Option<f64>,
    pub margin_bottom: Option<f64>,
    pub margin_left: Option<f64>,
    pub margin_right: Option<f64>,
    pub margin_header: Option<f64>,
    pub margin_footer: Option<f64>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CreateNamedRangeCommand {
    pub document_id: String,
    pub name: String,
    pub selector: RangeSelector,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DeleteNamedRangeCommand {
    pub document_id: String,
    pub named_range_id: Option<String>,
    pub name: Option<String>,
    pub dry_run: bool,
    pub json: bool,
    pub required_revision_id: Option<String>,
}

fn display_optional<T: std::fmt::Display>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".into())
}

fn write_json_line(
    out: &mut impl Write,
    value: &impl Serialize,
    error_context: &str,
) -> Result<()> {
    serde_json::to_writer(&mut *out, value).with_context(|| error_context.to_string())?;
    writeln!(out).context("failed to write newline")?;
    Ok(())
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

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableData {
    rows: Vec<Vec<String>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TableDimensions {
    rows: usize,
    columns: usize,
}

impl TableData {
    fn new(rows: Vec<Vec<String>>) -> Self {
        Self { rows }
    }

    fn dimensions(&self) -> TableDimensions {
        TableDimensions {
            rows: self.rows.len(),
            columns: self.rows[0].len(),
        }
    }

    fn rows(&self) -> &[Vec<String>] {
        &self.rows
    }
}

fn read_table_data(path: &str) -> Result<TableData> {
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
    Ok(TableData::new(rows))
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InsertTextDryRun {
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
pub(crate) struct ReplaceTextDryRun {
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
pub(crate) struct DocsHighLevelChange {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    before: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PreparedDocsChange {
    InsertText(InsertTextDryRun),
    ReplaceText(ReplaceTextDryRun),
    HighLevel(DocsHighLevelChange),
}

impl PreparedDocsChange {
    pub fn request_body(&self) -> &serde_json::Value {
        match self {
            Self::InsertText(change) => &change.request_body,
            Self::ReplaceText(change) => &change.request_body,
            Self::HighLevel(change) => &change.request_body,
        }
    }

    pub fn command_name(&self) -> &str {
        match self {
            Self::InsertText(_) => "text insert",
            Self::ReplaceText(_) => "text replace",
            Self::HighLevel(change) => change.preview.command.as_str(),
        }
    }

    pub fn location_index(&self) -> Option<i64> {
        match self {
            Self::InsertText(change) => change.location.index,
            Self::ReplaceText(_) => None,
            Self::HighLevel(change) => change.location.as_ref().and_then(|location| location.index),
        }
    }
}

impl DocsChangePreview {
    fn new(command: &str, summary: String) -> Self {
        Self {
            command: command.into(),
            summary,
            before: None,
            after: None,
        }
    }

    fn with_context(command: &str, summary: String, before: String, after: String) -> Self {
        Self {
            command: command.into(),
            summary,
            before: Some(before),
            after: Some(after),
        }
    }
}

pub(crate) fn prepare_insert_text_change(
    document_map: &DocumentMap,
    command: &InsertTextCommand,
) -> Result<PreparedDocsChange> {
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

    Ok(PreparedDocsChange::InsertText(InsertTextDryRun {
        revision_id: document_map.revision_id.clone(),
        location: resolved.location,
        request_body,
        preview,
    }))
}

pub(crate) fn prepare_replace_text_change(
    document_map: &DocumentMap,
    command: &ReplaceTextCommand,
) -> Result<PreparedDocsChange> {
    let ranges = resolve_replace_text_ranges(
        document_map,
        &command.old_text,
        command.match_number,
        command.all,
    )?;
    let request_body = replace_text_request_body(
        &ranges,
        &command.new_text,
        command.required_revision_id.as_deref(),
    );
    let preview = replace_text_preview(document_map, &ranges, &command.old_text, &command.new_text);

    Ok(PreparedDocsChange::ReplaceText(ReplaceTextDryRun {
        revision_id: document_map.revision_id.clone(),
        ranges,
        request_body,
        preview,
    }))
}

pub(crate) fn prepare_insert_image_change(
    document_map: &DocumentMap,
    command: &InsertImageCommand,
) -> Result<PreparedDocsChange> {
    let resolved = command
        .selector
        .as_ref()
        .map(|selector| resolve_insert_text_location(document_map, selector))
        .transpose()?;
    if resolved.is_some() == command.segment_id.is_some() {
        bail!("provide exactly one image location: a body selector or --segment-id");
    }
    let object_size = match (command.width, command.height) {
        (Some(width), Some(height)) => {
            if !width.is_finite() || width <= 0.0 {
                bail!("--width must be a finite number greater than zero");
            }
            if !height.is_finite() || height <= 0.0 {
                bail!("--height must be a finite number greater than zero");
            }
            Some(serde_json::json!({
                "width": { "magnitude": width, "unit": "PT" },
                "height": { "magnitude": height, "unit": "PT" }
            }))
        }
        (None, None) => None,
        _ => bail!("--width and --height must be provided together"),
    };
    let (location_field, location_value, location, preview) = if let Some(resolved) = resolved {
        let Some(index) = resolved.location.index else {
            bail!("image insert selector resolved without a Google Docs index");
        };
        (
            "location",
            serde_json::json!({ "index": index }),
            Some(resolved.location),
            DocsChangePreview::with_context(
                "image insert",
                format!(
                    "Insert inline image at index {index} from {}",
                    command.image_uri
                ),
                resolved.preview_before.clone(),
                insert_preview_text(
                    &resolved.preview_before,
                    resolved.preview_offset,
                    "[inline image]",
                ),
            ),
        )
    } else {
        let segment_id = command.segment_id.as_deref().unwrap().trim();
        if segment_id.is_empty() {
            bail!("--segment-id cannot be empty");
        }
        (
            "endOfSegmentLocation",
            serde_json::json!({ "segmentId": segment_id }),
            None,
            DocsChangePreview::new(
                "image insert",
                format!(
                    "Insert inline image at end of segment {segment_id} from {}",
                    command.image_uri
                ),
            ),
        )
    };
    let mut insert_inline_image = serde_json::json!({ "uri": command.image_uri });
    insert_inline_image[location_field] = location_value;
    if let Some(object_size) = object_size {
        insert_inline_image["objectSize"] = object_size;
    }
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "insertInlineImage": insert_inline_image
        })],
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location,
        range: None,
        request_body,
        preview,
    }))
}

pub(crate) fn prepare_insert_page_break_change(
    document_map: &DocumentMap,
    command: &InsertPageBreakCommand,
) -> Result<PreparedDocsChange> {
    let resolved = resolve_insert_text_location(document_map, &command.selector)?;
    let Some(index) = resolved.location.index else {
        bail!("break page selector resolved without a Google Docs index");
    };
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "insertPageBreak": {
                "location": { "index": index }
            }
        })],
        command.required_revision_id.as_deref(),
    );
    let preview_after = insert_preview_text(
        &resolved.preview_before,
        resolved.preview_offset,
        "[page break]",
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(resolved.location),
        range: None,
        request_body,
        preview: DocsChangePreview::with_context(
            "break page",
            format!("Insert page break at index {index}"),
            resolved.preview_before,
            preview_after,
        ),
    }))
}

pub(crate) fn prepare_insert_section_break_change(
    document_map: &DocumentMap,
    command: &InsertSectionBreakCommand,
) -> Result<PreparedDocsChange> {
    let resolved = resolve_insert_text_location(document_map, &command.selector)?;
    let Some(index) = resolved.location.index else {
        bail!("break section selector resolved without a Google Docs index");
    };
    let section_type = command.section_type.api_value();
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "insertSectionBreak": {
                "location": { "index": index },
                "sectionType": section_type
            }
        })],
        command.required_revision_id.as_deref(),
    );
    let preview_after = insert_preview_text(
        &resolved.preview_before,
        resolved.preview_offset,
        "[section break]",
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(resolved.location),
        range: None,
        request_body,
        preview: DocsChangePreview::with_context(
            "break section",
            format!("Insert {section_type} section break at index {index}"),
            resolved.preview_before,
            preview_after,
        ),
    }))
}

pub(crate) fn prepare_create_header_change(
    document_map: &DocumentMap,
    command: &CreateHeaderCommand,
) -> PreparedDocsChange {
    let mut create_header = serde_json::json!({ "type": "DEFAULT" });
    if let Some(index) = command.section_break_index {
        create_header["sectionBreakLocation"] = serde_json::json!({ "index": index });
    }
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "createHeader": create_header
        })],
        command.required_revision_id.as_deref(),
    );
    let target = command.section_break_index.map_or_else(
        || "the document's first section".to_string(),
        |index| format!("the section beginning at section break index {index}"),
    );
    let summary = command.text.as_ref().map_or_else(
        || format!("Create the DEFAULT header for {target}"),
        |text| format!("Create the DEFAULT header for {target} and insert {text:?}"),
    );
    PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: None,
        request_body,
        preview: DocsChangePreview::new("header create", summary),
    })
}

pub(crate) fn prepare_create_footer_change(
    document_map: &DocumentMap,
    command: &CreateFooterCommand,
) -> PreparedDocsChange {
    let mut create_footer = serde_json::json!({ "type": "DEFAULT" });
    if let Some(index) = command.section_break_index {
        create_footer["sectionBreakLocation"] = serde_json::json!({ "index": index });
    }
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "createFooter": create_footer
        })],
        command.required_revision_id.as_deref(),
    );
    let target = command.section_break_index.map_or_else(
        || "the document's first section".to_string(),
        |index| format!("the section beginning at section break index {index}"),
    );
    let summary = command.text.as_ref().map_or_else(
        || format!("Create the DEFAULT footer for {target}"),
        |text| format!("Create the DEFAULT footer for {target} and insert {text:?}"),
    );
    PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: None,
        request_body,
        preview: DocsChangePreview::new("footer create", summary),
    })
}

pub(crate) fn prepare_create_footnote_change(
    document_map: &DocumentMap,
    command: &CreateFootnoteCommand,
) -> Result<PreparedDocsChange> {
    let resolved = resolve_insert_text_location(document_map, &command.selector)?;
    let Some(index) = resolved.location.index else {
        bail!("footnote insert selector resolved without a Google Docs index");
    };
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "createFootnote": {
                "location": { "index": index }
            }
        })],
        command.required_revision_id.as_deref(),
    );
    let preview_after = insert_preview_text(
        &resolved.preview_before,
        resolved.preview_offset,
        "[footnote reference]",
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(resolved.location),
        range: None,
        request_body,
        preview: DocsChangePreview::with_context(
            "footnote insert",
            format!("Insert footnote reference at index {index}"),
            resolved.preview_before,
            preview_after,
        ),
    }))
}

pub(crate) fn prepare_insert_table_change(
    document_map: &DocumentMap,
    command: &InsertTableCommand,
) -> Result<PreparedDocsChange> {
    let data = match &command.data {
        Some(path) => Some(read_table_data(path)?),
        None => None,
    };
    let dimensions = insert_table_dimensions(command, data.as_ref())?;
    let resolved = resolve_insert_text_location(document_map, &command.selector)?;
    let Some(index) = resolved.location.index else {
        bail!("table insert selector resolved without a Google Docs index");
    };
    let requests = vec![serde_json::json!({
        "insertTable": {
            "location": { "index": index },
            "rows": dimensions.rows,
            "columns": dimensions.columns
        }
    })];
    let request_body =
        request_body_with_revision(requests, command.required_revision_id.as_deref());
    let summary = if let Some(data) = &data {
        format!(
            "Insert {}x{} table at index {index}: {}",
            dimensions.rows,
            dimensions.columns,
            compact_table_data_preview(data)
        )
    } else {
        format!(
            "Insert {}x{} table at index {index}",
            dimensions.rows, dimensions.columns
        )
    };
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(resolved.location),
        range: None,
        request_body,
        preview: DocsChangePreview::new("table insert", summary),
    }))
}

fn insert_table_dimensions(
    command: &InsertTableCommand,
    data: Option<&TableData>,
) -> Result<TableDimensions> {
    if data.is_some() && (command.rows.is_some() || command.columns.is_some()) {
        bail!("table insert accepts either --data or --rows with --columns, not both");
    }
    if let Some(data) = data {
        return Ok(data.dimensions());
    }
    let dimensions = explicit_table_dimensions(command.rows, command.columns)?;
    Ok(dimensions)
}

fn explicit_table_dimensions(
    rows: Option<usize>,
    columns: Option<usize>,
) -> Result<TableDimensions> {
    let (Some(rows), Some(columns)) = (rows, columns) else {
        bail!("table insert requires --data or --rows with --columns");
    };
    if rows == 0 || columns == 0 {
        bail!("table insert requires --rows and --columns to be greater than zero");
    }
    Ok(TableDimensions { rows, columns })
}

pub(crate) fn prepare_edit_table_change(
    document_map: &DocumentMap,
    command: &EditTableCommand,
) -> Result<PreparedDocsChange> {
    let data = read_table_data(&command.data)?;
    let data_dimensions = data.dimensions();
    let table = resolve_table_handle(document_map, &command.table_id)?;
    let table_dimensions = TableDimensions {
        rows: table.rows.unwrap_or(0),
        columns: table.columns.unwrap_or(0),
    };
    if !command.resize && data_dimensions != table_dimensions {
        bail!(
            "table edit data dimensions are {}x{} but {} is {}x{}; pass --resize when structural resizing is supported",
            data_dimensions.rows,
            data_dimensions.columns,
            command.table_id,
            table_dimensions.rows,
            table_dimensions.columns
        );
    }
    if command.resize {
        bail!("table edit --resize is not supported yet");
    }
    if table.table_cells.len() != table_dimensions.rows
        || table
            .table_cells
            .iter()
            .any(|row| row.len() != table_dimensions.columns)
    {
        bail!("selected table does not expose editable cell text ranges");
    }

    let request_body = request_body_with_revision(
        edit_table_requests(&table.table_cells, data.rows()),
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(table.location.clone()),
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "table edit",
            format!(
                "Replace {} with {}x{} table data",
                command.table_id, table_dimensions.rows, table_dimensions.columns
            ),
        ),
    }))
}

pub(crate) fn prepare_style_table_row_change(
    document_map: &DocumentMap,
    command: &StyleTableRowCommand,
) -> Result<PreparedDocsChange> {
    let table = resolve_table_handle(document_map, &command.table_id)?;
    if command.row == 0 || command.row > table.table_cells.len() {
        bail!(
            "table style --row must be between 1 and {} for {}",
            table.table_cells.len(),
            command.table_id
        );
    }
    let column_span = table.table_cells[command.row - 1].len();
    if column_span == 0 {
        bail!("selected table row does not expose editable cells");
    }
    let (column_index, column_span) = match command.column {
        Some(column) if column == 0 || column > column_span => bail!(
            "table style --column must be between 1 and {} for row {} of {}",
            column_span,
            command.row,
            command.table_id
        ),
        Some(column) => (column - 1, 1),
        None => (0, column_span),
    };
    if command.border_color.is_some() != command.border_width.is_some() {
        bail!("--border-color and --border-width must be provided together");
    }
    if command.background_color.is_none()
        && command.content_alignment.is_none()
        && command.border_color.is_none()
    {
        bail!(
            "table style requires --background-color, --content-alignment, or paired --border-color and --border-width"
        );
    }
    let table_start_index = table
        .location
        .index
        .context("selected table does not expose a Google Docs index")?;
    let mut table_cell_style = serde_json::Map::new();
    let mut fields = Vec::new();
    if let Some(background_color) = command.background_color.as_deref() {
        table_cell_style.insert(
            "backgroundColor".into(),
            foreground_color_payload(background_color)?,
        );
        fields.push("backgroundColor");
    }
    if let Some(content_alignment) = command.content_alignment {
        table_cell_style.insert(
            "contentAlignment".into(),
            serde_json::json!(content_alignment.api_value()),
        );
        fields.push("contentAlignment");
    }
    if let (Some(border_color), Some(border_width)) =
        (command.border_color.as_deref(), command.border_width)
    {
        if !border_width.is_finite() || border_width < 0.0 {
            bail!("--border-width must be a finite, non-negative point value");
        }
        let border = serde_json::json!({
            "color": foreground_color_payload(border_color)?,
            "dashStyle": "SOLID",
            "width": { "magnitude": border_width, "unit": "PT" }
        });
        for side in ["borderTop", "borderBottom", "borderLeft", "borderRight"] {
            table_cell_style.insert(side.into(), border.clone());
            fields.push(side);
        }
    }
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "updateTableCellStyle": {
                "tableCellStyle": table_cell_style,
                "tableRange": {
                    "tableCellLocation": {
                        "tableStartLocation": { "index": table_start_index },
                        "rowIndex": command.row - 1,
                        "columnIndex": column_index
                    },
                    "rowSpan": 1,
                    "columnSpan": column_span
                },
                "fields": fields.join(",")
            }
        })],
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(table.location.clone()),
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "table style",
            match command.column {
                Some(column) => format!(
                    "Style cell at row {}, column {} of {}",
                    command.row, column, command.table_id
                ),
                None => format!("Style row {} of {}", command.row, command.table_id),
            },
        ),
    }))
}

pub(crate) fn prepare_set_table_column_widths_change(
    document_map: &DocumentMap,
    command: &SetTableColumnWidthsCommand,
) -> Result<PreparedDocsChange> {
    let table = resolve_table_handle(document_map, &command.table_id)?;
    let column_count = table
        .columns
        .context("selected table does not expose its column count")?;
    if command.widths.len() != column_count {
        bail!(
            "table columns --widths requires {} values for {}, but received {}",
            column_count,
            command.table_id,
            command.widths.len()
        );
    }
    if let Some((column_index, width)) = command
        .widths
        .iter()
        .enumerate()
        .find(|(_, width)| !width.is_finite() || **width < 5.0)
    {
        bail!(
            "table columns width {} must be a finite value of at least 5 points, but received {}",
            column_index + 1,
            width
        );
    }
    let table_start_index = table
        .location
        .index
        .context("selected table does not expose a Google Docs index")?;
    let requests = command
        .widths
        .iter()
        .enumerate()
        .map(|(column_index, width)| {
            serde_json::json!({
                "updateTableColumnProperties": {
                    "tableStartLocation": { "index": table_start_index },
                    "columnIndices": [column_index],
                    "tableColumnProperties": {
                        "widthType": "FIXED_WIDTH",
                        "width": { "magnitude": width, "unit": "PT" }
                    },
                    "fields": "width,widthType"
                }
            })
        })
        .collect();
    let request_body =
        request_body_with_revision(requests, command.required_revision_id.as_deref());
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(table.location.clone()),
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "table columns",
            format!(
                "Set {} column widths on {} to {} points",
                column_count,
                command.table_id,
                command
                    .widths
                    .iter()
                    .map(|width| width.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        ),
    }))
}

pub(crate) fn prepare_pin_table_header_rows_change(
    document_map: &DocumentMap,
    command: &PinTableHeaderRowsCommand,
) -> Result<PreparedDocsChange> {
    let table = resolve_table_handle(document_map, &command.table_id)?;
    let row_count = table
        .rows
        .context("selected table does not expose its row count")?;
    if command.rows > row_count {
        bail!(
            "table header-rows --rows must be between 0 and {} for {}, but received {}",
            row_count,
            command.table_id,
            command.rows
        );
    }
    let table_start_index = table
        .location
        .index
        .context("selected table does not expose a Google Docs index")?;
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "pinTableHeaderRows": {
                "tableStartLocation": { "index": table_start_index },
                "pinnedHeaderRowsCount": command.rows
            }
        })],
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: Some(table.location.clone()),
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "table header-rows",
            format!(
                "Pin {} leading header row(s) on {}",
                command.rows, command.table_id
            ),
        ),
    }))
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

pub(crate) fn prepare_apply_styles_change(
    document_map: &DocumentMap,
    command: &ApplyStylesCommand,
    style_template: Option<&StyleTemplate>,
) -> Result<PreparedDocsChange> {
    let segment_id = command.segment_id.as_deref().map(str::trim);
    if segment_id.is_some_and(str::is_empty) {
        bail!("--segment-id cannot be empty");
    }
    if segment_id.is_some() && !matches!(command.selector, RangeSelector::IndexRange { .. }) {
        bail!("style apply --segment-id requires --from-index and --to-index");
    }
    let range = resolve_range_selector(document_map, &command.selector)?;
    let request_range = docs_range_in_segment(&range, segment_id);
    let raw_payload = raw_style_payload(command.style_json.as_deref())?;

    let has_heading = command.heading.is_some();
    let cached_named_style = command
        .heading
        .as_ref()
        .and_then(|heading| style_template.and_then(|template| template.named_styles.get(heading)));
    let cached_text_style = cached_named_style.map(|named| &named.text_style);
    let cached_paragraph_style = cached_named_style.and_then(|named| named.paragraph_style.clone());

    let (text_style, fields) = text_style_payload(
        command,
        raw_payload.text_style,
        has_heading,
        cached_text_style,
    )?;
    let (paragraph_style, paragraph_fields) =
        paragraph_style_payload(command, raw_payload.paragraph_style, cached_paragraph_style)?;
    let mut requests = Vec::new();
    if !paragraph_fields.is_empty() {
        requests.push(serde_json::json!({
            "updateParagraphStyle": {
                "range": request_range,
                "paragraphStyle": paragraph_style,
                "fields": paragraph_fields.join(",")
            }
        }));
    }
    if !fields.is_empty() {
        requests.push(serde_json::json!({
            "updateTextStyle": {
                "range": request_range,
                "textStyle": text_style,
                "fields": fields.join(",")
            }
        }));
    }
    if requests.is_empty() {
        bail!("style apply requires at least one style flag");
    }
    let request_body =
        request_body_with_revision(requests, command.required_revision_id.as_deref());
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: Some(range.clone()),
        request_body,
        preview: DocsChangePreview::new(
            "style apply",
            format!(
                "Apply styles to range {}..{}",
                range.start_index, range.end_index
            ),
        ),
    }))
}

pub(crate) fn prepare_update_named_style_change(
    document_map: &DocumentMap,
    command: &UpdateNamedStyleCommand,
) -> Result<PreparedDocsChange> {
    let named_style = command.named_style.trim();
    const NAMED_STYLE_TYPES: [&str; 9] = [
        "NORMAL_TEXT",
        "TITLE",
        "SUBTITLE",
        "HEADING_1",
        "HEADING_2",
        "HEADING_3",
        "HEADING_4",
        "HEADING_5",
        "HEADING_6",
    ];
    if !NAMED_STYLE_TYPES.contains(&named_style) {
        bail!(
            "named style must be one of NORMAL_TEXT, TITLE, SUBTITLE, or HEADING_1 through HEADING_6"
        );
    }
    let tab_id = command.tab_id.as_deref().map(str::trim);
    if tab_id.is_some_and(str::is_empty) {
        bail!("--tab-id cannot be empty");
    }

    let style_value: serde_json::Value = serde_json::from_str(&command.style_json)
        .context("failed to parse --style-json as Google Docs style JSON")?;
    let has_style_container = style_value.as_object().is_some_and(|style| {
        style.contains_key("textStyle") || style.contains_key("paragraphStyle")
    });
    if !has_style_container {
        bail!("style named requires textStyle and/or paragraphStyle in --style-json");
    }
    let raw_payload = raw_style_payload(Some(&command.style_json))?;
    if raw_payload
        .paragraph_style
        .as_ref()
        .is_some_and(|style| style.contains_key("namedStyleType"))
    {
        bail!(
            "paragraphStyle.namedStyleType cannot be updated; select the native style with the NAMED_STYLE argument"
        );
    }

    let mut named_style_payload = serde_json::Map::new();
    named_style_payload.insert(
        "namedStyleType".into(),
        serde_json::Value::String(named_style.into()),
    );
    let mut fields = vec!["namedStyleType".to_string()];
    if let Some(text_style) = raw_payload.text_style {
        fields.push("textStyle".into());
        fields.extend(text_style.keys().map(|key| format!("textStyle.{key}")));
        named_style_payload.insert("textStyle".into(), serde_json::Value::Object(text_style));
    }
    if let Some(paragraph_style) = raw_payload.paragraph_style {
        fields.push("paragraphStyle".into());
        fields.extend(
            paragraph_style
                .keys()
                .map(|key| format!("paragraphStyle.{key}")),
        );
        named_style_payload.insert(
            "paragraphStyle".into(),
            serde_json::Value::Object(paragraph_style),
        );
    }

    let mut update = serde_json::json!({
        "namedStyle": serde_json::Value::Object(named_style_payload),
        "fields": fields.join(",")
    });
    if let Some(tab_id) = tab_id {
        update["tabId"] = serde_json::Value::String(tab_id.into());
    }
    let request_body = request_body_with_revision(
        vec![serde_json::json!({ "updateNamedStyle": update })],
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "style named",
            format!("Update native named style {named_style}"),
        ),
    }))
}

pub(crate) fn prepare_copy_named_styles_change(
    source_map: &DocumentMap,
    target_map: &DocumentMap,
    command: &CopyNamedStylesCommand,
) -> Result<PreparedDocsChange> {
    let source_tab_id = command.source_tab_id.as_deref().map(str::trim);
    let target_tab_id = command.target_tab_id.as_deref().map(str::trim);
    if source_tab_id.is_some_and(str::is_empty) {
        bail!("--source-tab-id cannot be empty");
    }
    if target_tab_id.is_some_and(str::is_empty) {
        bail!("--target-tab-id cannot be empty");
    }
    let source = if let Some(tab_id) = source_tab_id {
        source_map
            .named_styles
            .iter()
            .find(|styles| styles.tab_id.as_deref() == Some(tab_id))
            .with_context(|| format!("source document has no named styles for tab {tab_id}"))?
    } else {
        source_map
            .named_styles
            .first()
            .context("source document has no native named styles")?
    };
    let styles = source
        .named_styles
        .get("styles")
        .and_then(serde_json::Value::as_array)
        .context("source document returned malformed native named styles")?;

    let mut requests = Vec::new();
    for source_style in styles {
        let Some(named_style) = source_style
            .get("namedStyleType")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let mut payload = serde_json::Map::new();
        if let Some(text_style) = source_style
            .get("textStyle")
            .and_then(serde_json::Value::as_object)
        {
            payload.insert(
                "textStyle".into(),
                serde_json::Value::Object(text_style.clone()),
            );
        }
        if let Some(paragraph_style) = source_style
            .get("paragraphStyle")
            .and_then(serde_json::Value::as_object)
        {
            let mut paragraph_style = paragraph_style.clone();
            paragraph_style.remove("namedStyleType");
            paragraph_style.remove("headingId");
            if !paragraph_style.is_empty() {
                payload.insert(
                    "paragraphStyle".into(),
                    serde_json::Value::Object(paragraph_style),
                );
            }
        }
        if payload.is_empty() {
            continue;
        }
        let single = prepare_update_named_style_change(
            target_map,
            &UpdateNamedStyleCommand {
                document_id: command.target_document_id.clone(),
                named_style: named_style.into(),
                style_json: serde_json::Value::Object(payload).to_string(),
                tab_id: target_tab_id.map(str::to_string),
                dry_run: command.dry_run,
                json: command.json,
                required_revision_id: None,
            },
        )?;
        let PreparedDocsChange::HighLevel(single) = single else {
            unreachable!("named style updates are always high-level changes")
        };
        requests.extend(
            single.request_body["requests"]
                .as_array()
                .expect("named style request list")
                .iter()
                .cloned(),
        );
    }
    if requests.is_empty() {
        bail!("source document has no copyable native named styles");
    }
    let copied_count = requests.len();
    let request_body =
        request_body_with_revision(requests, command.required_revision_id.as_deref());
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: target_map.revision_id.clone(),
        location: None,
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "style copy-named",
            format!("Copy {copied_count} native named styles from the source document"),
        ),
    }))
}

pub(crate) fn prepare_apply_list_change(
    document_map: &DocumentMap,
    command: &ApplyListCommand,
    style_template: Option<&StyleTemplate>,
) -> Result<PreparedDocsChange> {
    if command.list_type.is_some() && command.preset.is_some() {
        bail!("list apply accepts either --type or --preset, not both");
    }
    let preset = command
        .preset
        .clone()
        .or_else(|| command.list_type.map(list_type_preset).map(str::to_string))
        .or_else(|| {
            style_template
                .and_then(|template| template.list.as_ref())
                .map(|list| list.preset.clone())
        })
        .context(
            "list apply requires --type or --preset, and no cached style template was found for this document",
        )?;
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
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: Some(range.clone()),
        request_body,
        preview: DocsChangePreview::new(
            "list apply",
            format!(
                "Apply list preset to range {}..{}",
                range.start_index, range.end_index
            ),
        ),
    }))
}

pub(crate) fn prepare_configure_page_change(
    document_map: &DocumentMap,
    command: &ConfigurePageCommand,
) -> Result<PreparedDocsChange> {
    if command.page_width.is_some() != command.page_height.is_some() {
        bail!("--page-width and --page-height must be provided together");
    }

    let mut style = serde_json::Map::new();
    let mut fields = Vec::new();
    if let (Some(width), Some(height)) = (command.page_width, command.page_height) {
        if !width.is_finite() || width <= 0.0 {
            bail!("--page-width must be a finite, positive point value");
        }
        if !height.is_finite() || height <= 0.0 {
            bail!("--page-height must be a finite, positive point value");
        }
        style.insert(
            "pageSize".into(),
            serde_json::json!({
                "width": { "magnitude": width, "unit": "PT" },
                "height": { "magnitude": height, "unit": "PT" }
            }),
        );
        fields.push("pageSize");
    }

    for (field, flag, value) in [
        ("marginTop", "--margin-top", command.margin_top),
        ("marginBottom", "--margin-bottom", command.margin_bottom),
        ("marginLeft", "--margin-left", command.margin_left),
        ("marginRight", "--margin-right", command.margin_right),
        ("marginHeader", "--margin-header", command.margin_header),
        ("marginFooter", "--margin-footer", command.margin_footer),
    ] {
        let Some(value) = value else {
            continue;
        };
        if !value.is_finite() || value < 0.0 {
            bail!("{flag} must be a finite, non-negative point value");
        }
        style.insert(
            field.into(),
            serde_json::json!({ "magnitude": value, "unit": "PT" }),
        );
        fields.push(field);
    }

    if fields.is_empty() {
        bail!("style page requires a page size or at least one margin");
    }
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "updateDocumentStyle": {
                "documentStyle": style,
                "fields": fields.join(",")
            }
        })],
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "style page",
            format!("Configure document page fields: {}", fields.join(", ")),
        ),
    }))
}

pub(crate) fn prepare_copy_page_style_change(
    source_map: &DocumentMap,
    target_map: &DocumentMap,
    command: &CopyPageStyleCommand,
) -> Result<PreparedDocsChange> {
    let source_tab_id = command.source_tab_id.as_deref().map(str::trim);
    let target_tab_id = command.target_tab_id.as_deref().map(str::trim);
    if source_tab_id.is_some_and(str::is_empty) {
        bail!("--source-tab-id cannot be empty");
    }
    if target_tab_id.is_some_and(str::is_empty) {
        bail!("--target-tab-id cannot be empty");
    }
    let source = if let Some(tab_id) = source_tab_id {
        source_map
            .document_styles
            .iter()
            .find(|style| style.tab_id.as_deref() == Some(tab_id))
            .with_context(|| format!("source document has no page style for tab {tab_id}"))?
    } else {
        source_map
            .document_styles
            .first()
            .context("source document has no page style")?
    };

    let source_style = source
        .document_style
        .as_object()
        .context("source document returned malformed page style")?;
    let mut style = serde_json::Map::new();
    let mut fields = Vec::new();
    for field in [
        "documentFormat",
        "pageSize",
        "marginTop",
        "marginBottom",
        "marginLeft",
        "marginRight",
        "marginHeader",
        "marginFooter",
        "useFirstPageHeaderFooter",
        "useEvenPageHeaderFooter",
    ] {
        if let Some(value) = source_style.get(field) {
            style.insert(field.into(), value.clone());
            fields.push(field);
        }
    }
    if fields.is_empty() {
        bail!("source document has no copyable page layout");
    }
    let mut update = serde_json::json!({
        "documentStyle": style,
        "fields": fields.join(",")
    });
    if let Some(tab_id) = target_tab_id {
        update["tabId"] = serde_json::Value::String(tab_id.into());
    }
    let request_body = request_body_with_revision(
        vec![serde_json::json!({ "updateDocumentStyle": update })],
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: target_map.revision_id.clone(),
        location: None,
        range: None,
        request_body,
        preview: DocsChangePreview::new(
            "style copy-page",
            format!(
                "Copy {} page layout fields from the source document",
                fields.len()
            ),
        ),
    }))
}

pub(crate) fn prepare_create_named_range_change(
    document_map: &DocumentMap,
    command: &CreateNamedRangeCommand,
) -> Result<PreparedDocsChange> {
    let range = resolve_range_selector(document_map, &command.selector)?;
    let request_body = request_body_with_revision(
        vec![serde_json::json!({
            "createNamedRange": {
                "name": command.name,
                "range": docs_range(&range)
            }
        })],
        command.required_revision_id.as_deref(),
    );
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: Some(range.clone()),
        request_body,
        preview: DocsChangePreview::new(
            "named-range create",
            format!(
                "Create named range '{}' over {}..{}",
                command.name, range.start_index, range.end_index
            ),
        ),
    }))
}

pub(crate) fn prepare_delete_named_range_change(
    document_map: &DocumentMap,
    command: &DeleteNamedRangeCommand,
) -> Result<PreparedDocsChange> {
    let target = match (&command.named_range_id, &command.name) {
        (Some(named_range_id), None) => {
            serde_json::json!({ "namedRangeId": named_range_id })
        }
        (None, Some(name)) => serde_json::json!({ "name": name }),
        _ => bail!("named-range delete requires exactly one of --named-range-id or --name"),
    };
    let request_body = request_body_with_revision(
        vec![serde_json::json!({ "deleteNamedRange": target })],
        command.required_revision_id.as_deref(),
    );
    let summary = match (&command.named_range_id, &command.name) {
        (Some(named_range_id), None) => format!("Delete named range {named_range_id}"),
        (None, Some(name)) => format!("Delete named range(s) named '{name}'"),
        _ => unreachable!("validated above"),
    };
    Ok(PreparedDocsChange::HighLevel(DocsHighLevelChange {
        revision_id: document_map.revision_id.clone(),
        location: None,
        range: None,
        request_body,
        preview: DocsChangePreview::new("named-range delete", summary),
    }))
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

pub(crate) fn request_body_with_revision(
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

fn docs_range_in_segment(range: &DocumentRange, segment_id: Option<&str>) -> serde_json::Value {
    let mut value = serde_json::json!({
        "startIndex": range.start_index,
        "endIndex": range.end_index
    });
    if let Some(segment_id) = segment_id {
        value["segmentId"] = serde_json::json!(segment_id);
    }
    value
}

fn docs_range(range: &DocumentRange) -> serde_json::Value {
    docs_range_in_segment(range, None)
}

#[derive(Debug, Default)]
struct RawStylePayload {
    text_style: Option<StyleObject>,
    paragraph_style: Option<StyleObject>,
}

type StyleObject = serde_json::Map<String, serde_json::Value>;

fn raw_style_payload(style_json: Option<&str>) -> Result<RawStylePayload> {
    let Some(style_json) = style_json else {
        return Ok(RawStylePayload::default());
    };
    let value: serde_json::Value = serde_json::from_str(style_json)
        .context("failed to parse --style-json as Google Docs style JSON")?;
    let mut object = expect_json_object(value, "--style-json")?;
    let text_style = object
        .remove("textStyle")
        .map(|value| expect_json_object(value, "--style-json textStyle"))
        .transpose()?;
    let paragraph_style = object
        .remove("paragraphStyle")
        .map(|value| expect_json_object(value, "--style-json paragraphStyle"))
        .transpose()?;

    if text_style.is_some() || paragraph_style.is_some() {
        if !object.is_empty() {
            let unknown_fields = object.keys().cloned().collect::<Vec<_>>().join(", ");
            bail!(
                "--style-json with textStyle or paragraphStyle cannot include unknown top-level fields: {unknown_fields}"
            );
        }
        return Ok(RawStylePayload {
            text_style,
            paragraph_style,
        });
    }

    Ok(RawStylePayload {
        text_style: Some(object),
        paragraph_style: None,
    })
}

fn expect_json_object(value: serde_json::Value, label: &str) -> Result<StyleObject> {
    match value {
        serde_json::Value::Object(object) => Ok(object),
        _ => bail!("{label} must be a JSON object"),
    }
}

fn text_style_payload(
    command: &ApplyStylesCommand,
    raw_text_style: Option<StyleObject>,
    has_heading: bool,
    cached_text_style: Option<&crate::docs::style_template::TextStyleTemplate>,
) -> Result<(serde_json::Value, Vec<String>)> {
    let mut payload = StylePayloadParts::from_raw(raw_text_style);
    if command.bold {
        payload.set_field("bold", serde_json::Value::Bool(true));
    } else if has_heading && !payload.contains_field("bold") {
        if let Some(bold) = cached_text_style.and_then(|style| style.bold) {
            payload.set_field("bold", serde_json::Value::Bool(bold));
        }
    }
    if command.italic {
        payload.set_field("italic", serde_json::Value::Bool(true));
    } else if has_heading && !payload.contains_field("italic") {
        if let Some(italic) = cached_text_style.and_then(|style| style.italic) {
            payload.set_field("italic", serde_json::Value::Bool(italic));
        }
    }
    if command.underline {
        payload.set_field("underline", serde_json::Value::Bool(true));
    }
    if let Some(font_size) = command.font_size {
        payload.set_field(
            "fontSize",
            serde_json::json!({ "magnitude": font_size, "unit": "PT" }),
        );
    } else if has_heading && !payload.contains_field("fontSize") {
        if let Some(font_size) = cached_text_style.and_then(|style| style.font_size_pt) {
            payload.set_field(
                "fontSize",
                serde_json::json!({ "magnitude": font_size, "unit": "PT" }),
            );
        }
    }
    if let Some(font_family) = &command.font_family {
        if font_family.trim().is_empty() {
            bail!("--font-family cannot be empty");
        }
        payload.set_field(
            "weightedFontFamily",
            serde_json::json!({ "fontFamily": font_family }),
        );
    }
    if let Some(color) = &command.foreground_color {
        payload.set_field("foregroundColor", foreground_color_payload(color)?);
    } else if has_heading && !payload.contains_field("foregroundColor") {
        if let Some(color) = cached_text_style.and_then(|style| style.foreground_color.as_deref()) {
            payload.set_field("foregroundColor", foreground_color_payload(color)?);
        }
    }
    if let Some(heading_id) = &command.link_heading_id {
        if heading_id.trim().is_empty() {
            bail!("--link-heading-id cannot be empty");
        }
        payload.set_field("link", serde_json::json!({ "headingId": heading_id }));
    }
    Ok(payload.into_json_parts())
}

fn paragraph_style_payload(
    command: &ApplyStylesCommand,
    raw_paragraph_style: Option<StyleObject>,
    cached_paragraph_style: Option<serde_json::Value>,
) -> Result<(serde_json::Value, Vec<String>)> {
    let base_paragraph_style = cached_paragraph_style
        .map(|value| expect_json_object(value, "cached paragraph style"))
        .transpose()?;
    let mut payload =
        StylePayloadParts::from_base_and_raw(base_paragraph_style, raw_paragraph_style);
    if let Some(heading) = &command.heading {
        payload.set_field_first("namedStyleType", serde_json::Value::String(heading.into()));
    }
    if let Some(alignment) = command.alignment {
        payload.set_field(
            "alignment",
            serde_json::Value::String(alignment.api_value().into()),
        );
    }
    if let Some(direction) = command.direction {
        payload.set_field(
            "direction",
            serde_json::Value::String(direction.api_value().into()),
        );
    }
    set_paragraph_spacing(
        &mut payload,
        "spaceAbove",
        "--space-above",
        command.space_above,
    )?;
    set_paragraph_spacing(
        &mut payload,
        "spaceBelow",
        "--space-below",
        command.space_below,
    )?;
    if let Some(line_spacing) = command.line_spacing {
        if !line_spacing.is_finite() || line_spacing <= 0.0 {
            bail!("--line-spacing must be a finite, positive percentage");
        }
        payload.set_field("lineSpacing", serde_json::json!(line_spacing));
    }
    if let Some(spacing_mode) = command.spacing_mode {
        payload.set_field(
            "spacingMode",
            serde_json::Value::String(spacing_mode.api_value().into()),
        );
    }
    set_paragraph_dimension(
        &mut payload,
        "indentStart",
        "--indent-start",
        command.indent_start,
    )?;
    set_paragraph_dimension(
        &mut payload,
        "indentEnd",
        "--indent-end",
        command.indent_end,
    )?;
    set_paragraph_dimension(
        &mut payload,
        "indentFirstLine",
        "--indent-first-line",
        command.indent_first_line,
    )?;
    if command.keep_with_next {
        payload.set_field("keepWithNext", serde_json::Value::Bool(true));
    }
    if command.keep_lines_together {
        payload.set_field("keepLinesTogether", serde_json::Value::Bool(true));
    }
    if command.avoid_widow_and_orphan {
        payload.set_field("avoidWidowAndOrphan", serde_json::Value::Bool(true));
    }
    if command.page_break_before {
        payload.set_field("pageBreakBefore", serde_json::Value::Bool(true));
    }
    Ok(payload.into_json_parts())
}

fn set_paragraph_dimension(
    payload: &mut StylePayloadParts,
    field: &str,
    flag: &str,
    value: Option<f64>,
) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if !value.is_finite() || value < 0.0 {
        bail!("{flag} must be a finite, non-negative point value");
    }
    payload.set_field(
        field,
        serde_json::json!({ "magnitude": value, "unit": "PT" }),
    );
    Ok(())
}

fn set_paragraph_spacing(
    payload: &mut StylePayloadParts,
    field: &str,
    flag: &str,
    value: Option<f64>,
) -> Result<()> {
    let Some(value) = value else {
        return Ok(());
    };
    if !value.is_finite() || value < 0.0 {
        bail!("{flag} must be a finite, non-negative point value");
    }
    payload.set_field(
        field,
        serde_json::json!({ "magnitude": value, "unit": "PT" }),
    );
    Ok(())
}

struct StylePayloadParts {
    style: StyleObject,
    fields: Vec<String>,
}

impl StylePayloadParts {
    fn from_raw(raw_style: Option<StyleObject>) -> Self {
        Self::from_base_and_raw(None, raw_style)
    }

    fn from_base_and_raw(base_style: Option<StyleObject>, raw_style: Option<StyleObject>) -> Self {
        let mut style = base_style.unwrap_or_default();
        let mut fields = style.keys().cloned().collect::<Vec<_>>();
        if let Some(raw_style) = raw_style {
            for (key, value) in raw_style {
                style.insert(key.clone(), value);
                if !fields.iter().any(|existing| existing == &key) {
                    fields.push(key);
                }
            }
        }
        Self { style, fields }
    }

    fn set_field(&mut self, field: &str, value: serde_json::Value) {
        self.style.insert(field.into(), value);
        if !self.fields.iter().any(|existing| existing == field) {
            self.fields.push(field.to_string());
        }
    }

    fn contains_field(&self, field: &str) -> bool {
        self.style.contains_key(field)
    }

    fn set_field_first(&mut self, field: &str, value: serde_json::Value) {
        self.style.insert(field.into(), value);
        self.fields.retain(|existing| existing != field);
        self.fields.insert(0, field.to_string());
    }

    fn into_json_parts(self) -> (serde_json::Value, Vec<String>) {
        (serde_json::Value::Object(self.style), self.fields)
    }
}

pub(crate) fn foreground_color_payload(color: &str) -> Result<serde_json::Value> {
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

fn compact_table_data_preview(data: &TableData) -> String {
    let preview = data
        .rows()
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

fn write_insert_text_dry_run(
    out: &mut impl Write,
    dry_run: &InsertTextDryRun,
    json: bool,
) -> Result<()> {
    if json {
        write_json_line(out, dry_run, "failed to serialize Docs text insert dry run")
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
            "failed to serialize Docs text replace dry run",
        )
    } else {
        write_replace_text_preview(out, dry_run)
    }
}

pub(crate) fn write_docs_change_preview(
    out: &mut impl Write,
    change: &PreparedDocsChange,
    json: bool,
) -> Result<()> {
    match change {
        PreparedDocsChange::InsertText(change) => write_insert_text_dry_run(out, change, json),
        PreparedDocsChange::ReplaceText(change) => write_replace_text_dry_run(out, change, json),
        PreparedDocsChange::HighLevel(change) => write_high_level_change_preview(out, change, json),
    }
}

fn write_high_level_change_preview(
    out: &mut impl Write,
    change: &DocsHighLevelChange,
    json: bool,
) -> Result<()> {
    if json {
        return write_json_line(out, change, "failed to serialize Docs dry run");
    }
    writeln!(
        out,
        "{}: {}",
        change.preview.command, change.preview.summary
    )
    .context("failed to write Docs dry-run preview")?;
    if let (Some(before), Some(after)) = (&change.preview.before, &change.preview.after) {
        writeln!(out, "Before: {before}").context("failed to write Docs dry-run before preview")?;
        writeln!(out, "After: {after}").context("failed to write Docs dry-run after preview")?;
    }
    Ok(())
}

pub(crate) fn split_docs_request_bodies(
    request_body: &serde_json::Value,
    command_name: &str,
) -> Vec<serde_json::Value> {
    if command_name != "style apply" {
        return vec![request_body.clone()];
    }

    let Some(requests) = request_body
        .get("requests")
        .and_then(serde_json::Value::as_array)
    else {
        return vec![request_body.clone()];
    };
    if requests.len() <= 1 {
        return vec![request_body.clone()];
    }

    requests
        .iter()
        .map(|request| serde_json::json!({ "requests": [request.clone()] }))
        .collect()
}

pub(crate) fn request_body_required_revision_id(
    request_body: &serde_json::Value,
) -> Option<String> {
    request_body
        .get("writeControl")
        .and_then(|write_control| write_control.get("requiredRevisionId"))
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
}

pub(crate) fn set_request_body_required_revision_id(
    request_body: &mut serde_json::Value,
    required_revision_id: Option<&str>,
) {
    let Some(object) = request_body.as_object_mut() else {
        return;
    };

    match required_revision_id {
        Some(required_revision_id) => {
            object.insert(
                "writeControl".into(),
                serde_json::json!({ "requiredRevisionId": required_revision_id }),
            );
        }
        None => {
            object.remove("writeControl");
        }
    }
}

fn write_insert_text_preview(out: &mut impl Write, dry_run: &InsertTextDryRun) -> Result<()> {
    writeln!(
        out,
        "Insert text at index {}",
        display_optional(dry_run.location.index)
    )
    .context("failed to write Docs text insert preview header")?;
    writeln!(out, "Before: {}", dry_run.preview.before)
        .context("failed to write Docs text insert before preview")?;
    writeln!(out, "After: {}", dry_run.preview.after)
        .context("failed to write Docs text insert after preview")?;
    Ok(())
}

fn write_replace_text_preview(out: &mut impl Write, dry_run: &ReplaceTextDryRun) -> Result<()> {
    writeln!(out, "Replace text in {} match(es)", dry_run.ranges.len())
        .context("failed to write Docs text replace preview header")?;
    for (index, change) in dry_run.preview.changes.iter().enumerate() {
        writeln!(
            out,
            "Match {} at index {}",
            index + 1,
            change.range.start_index
        )
        .context("failed to write Docs text replace match preview")?;
        writeln!(out, "Before: {}", change.before)
            .context("failed to write Docs text replace before preview")?;
        writeln!(out, "After: {}", change.after)
            .context("failed to write Docs text replace after preview")?;
    }
    Ok(())
}
pub(crate) fn table_header_style_requests(
    document_map: &DocumentMap,
    table_style: &crate::docs::style_template::TableStyleTemplate,
) -> Option<Vec<serde_json::Value>> {
    let table_entry = document_map
        .entries
        .iter()
        .rev()
        .find(|entry| entry.kind == DocumentMapEntryKind::Table)?;
    let table_start_index = table_entry.location.index?;
    let header_row = table_entry.table_cells.first()?;
    if header_row.is_empty() {
        return None;
    }

    let mut requests = Vec::new();

    if let Some(color) = &table_style.header_row.background_color {
        let background_color = foreground_color_payload(color).ok()?;
        for column_index in 0..header_row.len() {
            requests.push(serde_json::json!({
                "updateTableCellStyle": {
                    "tableCellStyle": { "backgroundColor": background_color },
                    "tableRange": {
                        "tableCellLocation": {
                            "tableStartLocation": { "index": table_start_index },
                            "rowIndex": 0,
                            "columnIndex": column_index
                        },
                        "rowSpan": 1,
                        "columnSpan": 1
                    },
                    "fields": "backgroundColor"
                }
            }));
        }
    }

    if !table_style.header_row.text_style.is_empty() {
        let (style, fields) = direct_text_style_payload(
            table_style.header_row.text_style.bold,
            table_style.header_row.text_style.italic,
            table_style.header_row.text_style.font_size_pt,
            table_style
                .header_row
                .text_style
                .foreground_color
                .as_deref(),
        )
        .ok()?;
        if !fields.is_empty() {
            for range in header_row {
                if range.end_index > range.start_index {
                    requests.push(serde_json::json!({
                        "updateTextStyle": {
                            "range": docs_range(range),
                            "textStyle": style,
                            "fields": fields.join(",")
                        }
                    }));
                }
            }
        }
    }

    Some(requests)
}

fn direct_text_style_payload(
    bold: Option<bool>,
    italic: Option<bool>,
    font_size: Option<f64>,
    foreground_color: Option<&str>,
) -> Result<(serde_json::Value, Vec<String>)> {
    let mut style = serde_json::Map::new();
    let mut fields = Vec::new();

    if let Some(bold) = bold {
        style.insert("bold".into(), serde_json::Value::Bool(bold));
        fields.push("bold".to_string());
    }
    if let Some(italic) = italic {
        style.insert("italic".into(), serde_json::Value::Bool(italic));
        fields.push("italic".to_string());
    }
    if let Some(font_size) = font_size {
        style.insert(
            "fontSize".into(),
            serde_json::json!({ "magnitude": font_size, "unit": "PT" }),
        );
        fields.push("fontSize".to_string());
    }
    if let Some(color) = foreground_color {
        style.insert("foregroundColor".into(), foreground_color_payload(color)?);
        fields.push("foregroundColor".to_string());
    }

    Ok((serde_json::Value::Object(style), fields))
}
