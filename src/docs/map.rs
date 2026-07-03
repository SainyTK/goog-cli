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

pub fn build_document_map(document: &Value) -> DocumentMap {
    let mut builder = DocumentMapBuilder::new(
        collect_table_of_contents_page_hints(document),
        document
            .get("positionedObjects")
            .cloned()
            .unwrap_or_else(|| serde_json::json!({})),
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

struct DocumentMapBuilder {
    entries: Vec<DocumentMapEntry>,
    text_blocks: Vec<DocumentTextBlock>,
    current_page: Option<usize>,
    current_confidence: LocationConfidence,
    content_line: usize,
    table_count: usize,
    image_count: usize,
    toc_page_hints: Vec<TableOfContentsPageHint>,
    positioned_objects: Value,
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

impl DocumentMapBuilder {
    fn new(toc_page_hints: Vec<TableOfContentsPageHint>, positioned_objects: Value) -> Self {
        Self {
            entries: Vec::new(),
            text_blocks: Vec::new(),
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
            self.image_count += 1;
            let mut location = self.current_location(element);
            location.index = image.start_index;
            self.push_entry_with_metadata(
                location,
                DocumentMapEntryKind::InlineImage,
                style.clone(),
                inline_image_preview(image_index, inline_image_count),
                DocumentMapEntryMetadata {
                    image_handle: Some(format!("image-{}", self.image_count)),
                    object_id: Some(image.object_id),
                    ..DocumentMapEntryMetadata::default()
                },
            );
        }

        for (object_index, object_id) in positioned_object_ids.into_iter().enumerate() {
            self.image_count += 1;
            let layout_metadata =
                positioned_image_layout_metadata(&self.positioned_objects, &object_id);
            self.push_entry_with_metadata(
                self.current_location(element),
                DocumentMapEntryKind::PositionedImage,
                style.clone(),
                format!("[positioned image {}]", object_index + 1),
                DocumentMapEntryMetadata {
                    image_handle: Some(format!("image-{}", self.image_count)),
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

fn document_content(document: &Value) -> impl Iterator<Item = &Value> {
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

fn tab_content(tab: &Value) -> impl Iterator<Item = &Value> {
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

fn positioned_image_layout_metadata(positioned_objects: &Value, object_id: &str) -> Option<Value> {
    let properties = positioned_objects
        .get(object_id)?
        .get("positionedObjectProperties")?;
    let mut metadata = serde_json::Map::new();

    if let Some(positioning) = properties.get("positioning") {
        metadata.insert("positioning".into(), positioning.clone());
    }

    if let Some(embedded_object) = properties.get("embeddedObject") {
        for field in [
            "size",
            "marginLeft",
            "marginRight",
            "marginTop",
            "marginBottom",
        ] {
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
