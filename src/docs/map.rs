use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMap {
    pub document_id: Option<String>,
    pub title: Option<String>,
    pub revision_id: Option<String>,
    pub entries: Vec<DocumentMapEntry>,
    pub document_locations: Vec<DocumentLocation>,
    #[serde(skip)]
    pub text_blocks: Vec<DocumentTextBlock>,
    #[serde(skip)]
    pub insertion_locations: Vec<DocumentLocation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMapEntry {
    pub entry: usize,
    pub location: DocumentLocation,
    pub kind: DocumentMapEntryKind,
    pub style: Option<String>,
    pub preview: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image_handle: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub object_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub layout_metadata: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub columns: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_handle: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub table_cells: Vec<Vec<DocumentRange>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentTextBlock {
    pub location: DocumentLocation,
    pub start_index: i64,
    pub text: String,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentRange {
    pub start_index: i64,
    pub end_index: i64,
    pub location: DocumentLocation,
    pub preview: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentLocation {
    pub index: Option<i64>,
    pub page: Option<usize>,
    pub content_line: usize,
    pub confidence: LocationConfidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LocationConfidence {
    ExplicitPageBreak,
    TableOfContents,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum DocumentMapEntryKind {
    Heading,
    Paragraph,
    Table,
    InlineImage,
    PositionedImage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContentSelector {
    Index(i64),
    Entry(usize),
    PageLine { page: usize, line: usize },
    Heading(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InsertTextSelector {
    Index(i64),
    Entry(usize),
    PageLine { page: usize, line: usize },
    AfterHeading(String),
    BeforeHeading(String),
    AfterText(String),
    BeforeText(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RangeSelector {
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedInsertLocation {
    pub location: DocumentLocation,
    pub preview_before: String,
    pub preview_offset: usize,
}

pub fn build_document_map(document: &Value) -> DocumentMap {
    let mut builder = DocumentMapBuilder::new(
        collect_table_of_contents_page_hints(document),
        document.get("positionedObjects"),
    );

    for content in document_content(document) {
        builder.push_structural_element(content);
    }

    let entries = builder.entries;
    DocumentMap {
        document_id: string_field(document, "documentId"),
        title: string_field(document, "title"),
        revision_id: string_field(document, "revisionId"),
        document_locations: entries.iter().map(|entry| entry.location.clone()).collect(),
        entries,
        text_blocks: builder.text_blocks,
        insertion_locations: builder.insertion_locations,
    }
}

pub fn search_document_text(document_map: &DocumentMap, needle: &str) -> Vec<DocumentRange> {
    if needle.is_empty() {
        return Vec::new();
    }

    document_map
        .text_blocks
        .iter()
        .flat_map(|block| text_matches(block, needle))
        .collect()
}

pub fn resolve_content_entry<'a>(
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

pub fn resolve_range_selector(
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
                confidence: LocationConfidence::Unknown,
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
            let ranges = resolve_text_ranges(document_map, text, *match_number, false)?;
            ranges
                .into_iter()
                .next()
                .context("text range selector did not resolve a match")
        }
    }
}

pub fn resolve_insert_text_location(
    document_map: &DocumentMap,
    selector: &InsertTextSelector,
) -> Result<ResolvedInsertLocation> {
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
            Ok(ResolvedInsertLocation {
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
            Ok(ResolvedInsertLocation {
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

pub fn resolve_replace_text_ranges(
    document_map: &DocumentMap,
    old_text: &str,
    match_number: Option<usize>,
    all: bool,
) -> Result<Vec<DocumentRange>> {
    resolve_text_ranges(document_map, old_text, match_number, all)
}

fn range_for_entry(document_map: &DocumentMap, entry: &DocumentMapEntry) -> Result<DocumentRange> {
    let start_index = entry
        .location
        .index
        .context("selected Document Map entry has no Google Docs index")?;
    let end_index = text_block_starting_at(document_map, start_index)
        .map(text_block_end_index)
        .or_else(|| next_entry_index_after(document_map, start_index))
        .unwrap_or(start_index + 1);
    Ok(DocumentRange {
        start_index,
        end_index,
        location: entry.location.clone(),
        preview: entry.preview.clone(),
    })
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

fn resolved_for_index(document_map: &DocumentMap, index: i64) -> Result<ResolvedInsertLocation> {
    if let Ok(entry) = resolve_content_entry(document_map, &ContentSelector::Index(index)) {
        let preview_offset = entry
            .location
            .index
            .map(|start| preview_offset_for_index(&entry.preview, start, index))
            .unwrap_or(0);
        return Ok(ResolvedInsertLocation {
            location: DocumentLocation {
                index: Some(index),
                ..entry.location.clone()
            },
            preview_before: entry.preview.clone(),
            preview_offset,
        });
    }

    let location = document_map
        .insertion_locations
        .iter()
        .find(|location| location.index == Some(index))
        .cloned()
        .with_context(|| format!("no content found at Google Docs index {index}"))?;

    Ok(ResolvedInsertLocation {
        location,
        preview_before: String::new(),
        preview_offset: 0,
    })
}

fn resolved_for_entry_start(entry: &DocumentMapEntry) -> Result<ResolvedInsertLocation> {
    let Some(index) = entry.location.index else {
        bail!(
            "Document Map entry {} does not have a Google Docs index",
            entry.entry
        );
    };
    Ok(ResolvedInsertLocation {
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
) -> Result<ResolvedInsertLocation> {
    let Some(start_index) = entry.location.index else {
        bail!(
            "Document Map entry {} does not have a Google Docs index",
            entry.entry
        );
    };
    let end_index = text_block_starting_at(document_map, start_index)
        .map(text_block_end_index)
        .unwrap_or(start_index);
    Ok(ResolvedInsertLocation {
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

pub(crate) fn text_block_contains_range(block: &DocumentTextBlock, range: &DocumentRange) -> bool {
    block.start_index <= range.start_index && range.end_index <= text_block_end_index(block)
}

fn text_block_starting_at(
    document_map: &DocumentMap,
    start_index: i64,
) -> Option<&DocumentTextBlock> {
    document_map
        .text_blocks
        .iter()
        .find(|block| block.start_index == start_index)
}

fn next_entry_index_after(document_map: &DocumentMap, start_index: i64) -> Option<i64> {
    document_map
        .entries
        .iter()
        .filter_map(|candidate| candidate.location.index)
        .find(|candidate_index| *candidate_index > start_index)
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

fn resolve_text_ranges(
    document_map: &DocumentMap,
    text: &str,
    match_number: Option<usize>,
    all: bool,
) -> Result<Vec<DocumentRange>> {
    if text.is_empty() {
        bail!("replace-text old text must not be empty");
    }
    if all && match_number.is_some() {
        bail!("provide only one replace-text disambiguator: --match or --all");
    }
    if match_number == Some(0) {
        bail!("--match must be 1 or greater");
    }

    let matches = search_document_text(document_map, text);
    if matches.is_empty() {
        bail!("replace-text did not match {text:?}");
    }
    if all {
        return Ok(matches);
    }
    if let Some(match_number) = match_number {
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
            bail!("ambiguous replace-text match {text:?}; candidates: {candidate_list}")
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

fn preview_offset_for_index(preview: &str, block_start_index: i64, insertion_index: i64) -> usize {
    let offset = insertion_index.saturating_sub(block_start_index) as usize;
    preview.chars().take(offset).count()
}

fn display_optional<T: ToString>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".into())
}

struct DocumentMapBuilder<'a> {
    entries: Vec<DocumentMapEntry>,
    text_blocks: Vec<DocumentTextBlock>,
    insertion_locations: Vec<DocumentLocation>,
    current_page: Option<usize>,
    current_confidence: LocationConfidence,
    content_line: usize,
    table_count: usize,
    image_count: usize,
    toc_page_hints: Vec<TableOfContentsPageHint>,
    positioned_objects: Option<&'a Value>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct DocumentMapEntryMetadata {
    image_handle: Option<String>,
    object_id: Option<String>,
    layout_metadata: Option<Value>,
    rows: Option<usize>,
    columns: Option<usize>,
    table_handle: Option<String>,
    table_cells: Vec<Vec<DocumentRange>>,
}

impl<'a> DocumentMapBuilder<'a> {
    fn new(
        toc_page_hints: Vec<TableOfContentsPageHint>,
        positioned_objects: Option<&'a Value>,
    ) -> Self {
        Self {
            entries: Vec::new(),
            text_blocks: Vec::new(),
            insertion_locations: Vec::new(),
            current_page: None,
            current_confidence: LocationConfidence::Unknown,
            content_line: 0,
            table_count: 0,
            image_count: 0,
            toc_page_hints,
            positioned_objects,
        }
    }

    fn push_structural_element(&mut self, element: &Value) {
        if contains_page_break(element) {
            self.advance_explicit_page();
        }

        self.push_insertion_location(element);

        if let Some(paragraph) = element.get("paragraph") {
            self.push_paragraph(element, paragraph);
        } else if let Some(table) = element.get("table") {
            self.push_table(element, table);
        }
    }

    fn push_paragraph(&mut self, element: &Value, paragraph: &Value) {
        let text = paragraph_text(paragraph);
        let trimmed_text = text.trim();
        let inline_images = paragraph_inline_images(paragraph);
        let positioned_object_ids = paragraph_positioned_object_ids(paragraph);
        let style = paragraph_style(paragraph);
        let is_heading = style.as_deref().is_some_and(is_heading_style);

        if !trimmed_text.is_empty() {
            self.push_content_line();
            let location = self.text_location(element, is_heading, trimmed_text);
            let kind = if is_heading {
                DocumentMapEntryKind::Heading
            } else {
                DocumentMapEntryKind::Paragraph
            };
            let preview = preview(&text);
            if let Some(start_index) = location.index {
                self.text_blocks.push(DocumentTextBlock {
                    location: location.clone(),
                    start_index,
                    text: text.clone(),
                    preview: preview.clone(),
                });
            }
            self.push_entry(location, kind, style.clone(), preview);
        } else if !inline_images.is_empty() || !positioned_object_ids.is_empty() {
            self.push_content_line();
        }

        let inline_image_count = inline_images.len();
        for (image_index, image) in inline_images.into_iter().enumerate() {
            let image_handle = self.next_image_handle();
            let mut location = self.current_location(element);
            location.index = image.start_index;
            self.push_entry_with_metadata(
                location,
                DocumentMapEntryKind::InlineImage,
                style.clone(),
                inline_image_preview(image_index, inline_image_count),
                DocumentMapEntryMetadata {
                    image_handle: Some(image_handle),
                    object_id: Some(image.object_id),
                    ..DocumentMapEntryMetadata::default()
                },
            );
        }

        for (object_index, object_id) in positioned_object_ids.into_iter().enumerate() {
            let image_handle = self.next_image_handle();
            let layout_metadata =
                positioned_image_layout_metadata(self.positioned_objects, &object_id);
            self.push_entry_with_metadata(
                self.current_location(element),
                DocumentMapEntryKind::PositionedImage,
                style.clone(),
                format!("[positioned image {}]", object_index + 1),
                DocumentMapEntryMetadata {
                    image_handle: Some(image_handle),
                    object_id: Some(object_id),
                    layout_metadata,
                    ..DocumentMapEntryMetadata::default()
                },
            );
        }
    }

    fn push_table(&mut self, element: &Value, table: &Value) {
        self.push_content_line();
        self.table_count += 1;
        let (rows, columns) = table_dimensions(table);
        let location = self.current_location(element);
        let table_cells = table_cell_ranges(table, &location);
        let table_handle = format!("table-{}", self.table_count);
        self.push_entry_with_metadata(
            location,
            DocumentMapEntryKind::Table,
            None,
            preview(&table_preview(table)),
            DocumentMapEntryMetadata {
                rows: Some(rows),
                columns: Some(columns),
                table_handle: Some(table_handle),
                table_cells,
                ..DocumentMapEntryMetadata::default()
            },
        );
    }

    fn advance_explicit_page(&mut self) {
        self.current_page = Some(self.current_page.unwrap_or(1) + 1);
        self.current_confidence = LocationConfidence::ExplicitPageBreak;
        self.content_line = 0;
    }

    fn push_content_line(&mut self) {
        self.content_line += 1;
    }

    fn push_insertion_location(&mut self, element: &Value) {
        let location = self.current_location(element);
        if location.index.is_some() {
            self.insertion_locations.push(location);
        }
    }

    fn next_image_handle(&mut self) -> String {
        self.image_count += 1;
        format!("image-{}", self.image_count)
    }

    fn push_entry(
        &mut self,
        location: DocumentLocation,
        kind: DocumentMapEntryKind,
        style: Option<String>,
        preview: String,
    ) {
        self.push_entry_with_metadata(
            location,
            kind,
            style,
            preview,
            DocumentMapEntryMetadata::default(),
        );
    }

    fn push_entry_with_metadata(
        &mut self,
        location: DocumentLocation,
        kind: DocumentMapEntryKind,
        style: Option<String>,
        preview: String,
        metadata: DocumentMapEntryMetadata,
    ) {
        self.entries.push(DocumentMapEntry {
            entry: self.entries.len() + 1,
            location,
            kind,
            style,
            preview,
            image_handle: metadata.image_handle,
            object_id: metadata.object_id,
            layout_metadata: metadata.layout_metadata,
            rows: metadata.rows,
            columns: metadata.columns,
            table_handle: metadata.table_handle,
            table_cells: metadata.table_cells,
        });
    }

    fn text_location(
        &self,
        element: &Value,
        is_heading: bool,
        trimmed_text: &str,
    ) -> DocumentLocation {
        if !is_heading {
            return self.current_location(element);
        }

        if let Some(hint) = self
            .toc_page_hints
            .iter()
            .find(|hint| hint.heading == trimmed_text)
        {
            return self.location_for(
                element,
                Some(hint.page),
                LocationConfidence::TableOfContents,
            );
        }

        self.current_location(element)
    }

    fn current_location(&self, element: &Value) -> DocumentLocation {
        self.location_for(element, self.current_page, self.current_confidence)
    }

    fn location_for(
        &self,
        element: &Value,
        page: Option<usize>,
        confidence: LocationConfidence,
    ) -> DocumentLocation {
        DocumentLocation {
            index: element.get("startIndex").and_then(Value::as_i64),
            page,
            content_line: self.content_line,
            confidence,
        }
    }
}

fn is_heading_style(style: &str) -> bool {
    style == "TITLE" || style.starts_with("HEADING")
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct TableOfContentsPageHint {
    heading: String,
    page: usize,
}

fn collect_table_of_contents_page_hints(document: &Value) -> Vec<TableOfContentsPageHint> {
    let mut hints = Vec::new();
    for element in document_content(document) {
        let Some(toc_content) = element
            .get("tableOfContents")
            .and_then(|toc| toc.get("content"))
            .and_then(Value::as_array)
        else {
            continue;
        };

        for toc_element in toc_content {
            let Some(paragraph) = toc_element.get("paragraph") else {
                continue;
            };
            if let Some(hint) = parse_table_of_contents_hint(&paragraph_text(paragraph)) {
                hints.push(hint);
            }
        }
    }
    hints
}

fn parse_table_of_contents_hint(text: &str) -> Option<TableOfContentsPageHint> {
    let trimmed = text.trim();
    let page_number_start = trimmed
        .char_indices()
        .rev()
        .find(|(_, c)| !c.is_ascii_digit())
        .map(|(index, c)| index + c.len_utf8())?;
    let page_number = &trimmed[page_number_start..];
    if page_number_start >= trimmed.len() || !page_number.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }

    let page = page_number.parse().ok()?;
    let heading = trimmed[..page_number_start]
        .trim_end_matches(|c: char| c.is_whitespace() || c == '.')
        .trim()
        .to_string();
    if heading.is_empty() {
        return None;
    }

    Some(TableOfContentsPageHint { heading, page })
}

pub(crate) fn document_content(document: &Value) -> impl Iterator<Item = &Value> {
    document
        .get("body")
        .and_then(|body| body.get("content"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .chain(
            document
                .get("tabs")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .flat_map(tab_content),
        )
}

pub(crate) fn tab_content(tab: &Value) -> impl Iterator<Item = &Value> {
    tab.get("documentTab")
        .and_then(|document_tab| document_tab.get("body"))
        .and_then(|body| body.get("content"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
}

fn paragraph_text(paragraph: &Value) -> String {
    paragraph
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|element| {
            element
                .get("textRun")
                .and_then(|run| run.get("content"))
                .and_then(Value::as_str)
        })
        .collect::<String>()
}

fn paragraph_style(paragraph: &Value) -> Option<String> {
    paragraph
        .get("paragraphStyle")
        .and_then(|style| style.get("namedStyleType"))
        .and_then(Value::as_str)
        .map(str::to_string)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct InlineImageElement {
    start_index: Option<i64>,
    object_id: String,
}

fn paragraph_inline_images(paragraph: &Value) -> Vec<InlineImageElement> {
    paragraph
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|element| {
            let object_id = element
                .get("inlineObjectElement")
                .and_then(|inline| inline.get("inlineObjectId"))
                .and_then(Value::as_str)?;
            Some(InlineImageElement {
                start_index: element.get("startIndex").and_then(Value::as_i64),
                object_id: object_id.to_string(),
            })
        })
        .collect()
}

fn inline_image_preview(image_index: usize, inline_image_count: usize) -> String {
    if inline_image_count == 1 {
        "[inline image]".into()
    } else {
        format!("[inline image {}]", image_index + 1)
    }
}

fn paragraph_positioned_object_ids(paragraph: &Value) -> Vec<String> {
    paragraph
        .get("positionedObjectIds")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

const POSITIONED_IMAGE_EMBEDDED_METADATA_FIELDS: [&str; 5] = [
    "size",
    "marginLeft",
    "marginRight",
    "marginTop",
    "marginBottom",
];

fn positioned_image_layout_metadata(
    positioned_objects: Option<&Value>,
    object_id: &str,
) -> Option<Value> {
    let properties = positioned_objects?
        .get(object_id)?
        .get("positionedObjectProperties")?;
    let mut metadata = serde_json::Map::new();

    if let Some(positioning) = properties.get("positioning") {
        metadata.insert("positioning".into(), positioning.clone());
    }

    if let Some(embedded_object) = properties.get("embeddedObject") {
        for field in POSITIONED_IMAGE_EMBEDDED_METADATA_FIELDS {
            if let Some(value) = embedded_object.get(field) {
                metadata.insert(field.into(), value.clone());
            }
        }
    }

    if metadata.is_empty() {
        None
    } else {
        Some(Value::Object(metadata))
    }
}

fn contains_page_break(element: &Value) -> bool {
    element
        .get("paragraph")
        .and_then(|paragraph| paragraph.get("elements"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|paragraph_element| paragraph_element.get("pageBreak").is_some())
}

fn table_preview(table: &Value) -> String {
    let mut cells = Vec::new();
    if let Some(rows) = table.get("tableRows").and_then(Value::as_array) {
        for row in rows.iter().take(2) {
            let Some(table_cells) = row.get("tableCells").and_then(Value::as_array) else {
                continue;
            };
            let row_text = table_cells
                .iter()
                .take(3)
                .map(table_cell_text)
                .collect::<Vec<_>>()
                .join(" | ");
            if !row_text.trim().is_empty() {
                cells.push(row_text);
            }
        }
    }

    if cells.is_empty() {
        "[table]".into()
    } else {
        cells.join(" / ")
    }
}

fn table_dimensions(table: &Value) -> (usize, usize) {
    let Some(rows) = table.get("tableRows").and_then(Value::as_array) else {
        return (0, 0);
    };
    let columns = rows
        .iter()
        .filter_map(|row| {
            row.get("tableCells")
                .and_then(Value::as_array)
                .map(Vec::len)
        })
        .max()
        .unwrap_or(0);
    (rows.len(), columns)
}

fn table_cell_text(cell: &Value) -> String {
    cell.get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|element| element.get("paragraph"))
        .map(paragraph_text)
        .collect::<String>()
        .trim()
        .to_string()
}

fn table_cell_ranges(table: &Value, table_location: &DocumentLocation) -> Vec<Vec<DocumentRange>> {
    table
        .get("tableRows")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| {
            row.get("tableCells")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|cell| table_cell_range(cell, table_location))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn table_cell_range(cell: &Value, table_location: &DocumentLocation) -> DocumentRange {
    let text = table_cell_text(cell);
    let mut start_index = None;
    let mut end_index = None;
    for element in cell
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|element| element.get("paragraph"))
        .filter_map(|paragraph| paragraph.get("elements").and_then(Value::as_array))
        .flatten()
        .filter(|element| element.get("textRun").is_some())
    {
        if start_index.is_none() {
            start_index = element.get("startIndex").and_then(Value::as_i64);
        }
        end_index = element.get("endIndex").and_then(Value::as_i64);
    }
    let start_index = start_index.unwrap_or(0);
    let mut end_index = end_index.unwrap_or(start_index);
    if end_index > start_index {
        end_index -= 1;
    }
    DocumentRange {
        start_index,
        end_index,
        location: DocumentLocation {
            index: Some(start_index),
            ..table_location.clone()
        },
        preview: text,
    }
}

fn text_matches(block: &DocumentTextBlock, needle: &str) -> Vec<DocumentRange> {
    block
        .text
        .match_indices(needle)
        .map(|(byte_offset, _)| {
            let start_index = block.start_index + utf16_len(&block.text[..byte_offset]);
            let end_index = start_index + utf16_len(needle);
            DocumentRange {
                start_index,
                end_index,
                location: DocumentLocation {
                    index: Some(start_index),
                    ..block.location.clone()
                },
                preview: block.preview.clone(),
            }
        })
        .collect()
}

fn utf16_len(text: &str) -> i64 {
    text.encode_utf16().count() as i64
}

fn preview(text: &str) -> String {
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

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}
