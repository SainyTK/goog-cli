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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentMapEntry {
    pub entry: usize,
    pub location: DocumentLocation,
    pub kind: DocumentMapEntryKind,
    pub style: Option<String>,
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
    let mut builder = DocumentMapBuilder::new(collect_table_of_contents_page_hints(document));

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
    }
}

struct DocumentMapBuilder {
    entries: Vec<DocumentMapEntry>,
    current_page: Option<usize>,
    current_confidence: LocationConfidence,
    content_line: usize,
    toc_page_hints: Vec<TableOfContentsPageHint>,
}

impl DocumentMapBuilder {
    fn new(toc_page_hints: Vec<TableOfContentsPageHint>) -> Self {
        Self {
            entries: Vec::new(),
            current_page: None,
            current_confidence: LocationConfidence::Unknown,
            content_line: 0,
            toc_page_hints,
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
        let has_inline_image = paragraph_has_inline_image(paragraph);
        let positioned_count = paragraph_positioned_object_count(paragraph);
        let style = paragraph_style(paragraph);
        let is_heading = style.as_deref().map_or(false, is_heading_style);

        if !trimmed_text.is_empty() {
            self.push_content_line();
            let kind = if is_heading {
                DocumentMapEntryKind::Heading
            } else {
                DocumentMapEntryKind::Paragraph
            };
            self.push_entry(
                self.text_location(element, is_heading, trimmed_text),
                kind,
                style.clone(),
                preview(&text),
            );
        } else if has_inline_image || positioned_count > 0 {
            self.push_content_line();
        }

        if has_inline_image {
            self.push_entry(
                self.current_location(element),
                DocumentMapEntryKind::InlineImage,
                style.clone(),
                "[inline image]".into(),
            );
        }

        for object_number in 1..=positioned_count {
            self.push_entry(
                self.current_location(element),
                DocumentMapEntryKind::PositionedImage,
                style.clone(),
                format!("[positioned image {object_number}]"),
            );
        }
    }

    fn push_table(&mut self, element: &Value, table: &Value) {
        self.push_content_line();
        self.push_entry(
            self.current_location(element),
            DocumentMapEntryKind::Table,
            None,
            preview(&table_preview(table)),
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
        self.entries.push(DocumentMapEntry {
            entry: self.entries.len() + 1,
            location,
            kind,
            style,
            preview,
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

fn paragraph_has_inline_image(paragraph: &Value) -> bool {
    paragraph
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .any(|element| element.get("inlineObjectElement").is_some())
}

fn paragraph_positioned_object_count(paragraph: &Value) -> usize {
    paragraph
        .get("positionedObjectIds")
        .and_then(Value::as_array)
        .map_or(0, Vec::len)
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

fn preview(text: &str) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    const MAX_PREVIEW_CHARS: usize = 80;
    if compact.chars().count() <= MAX_PREVIEW_CHARS {
        compact
    } else {
        let mut truncated = compact.chars().take(MAX_PREVIEW_CHARS - 3).collect::<String>();
        truncated.push_str("...");
        truncated
    }
}

fn string_field(value: &Value, field: &str) -> Option<String> {
    value.get(field).and_then(Value::as_str).map(str::to_string)
}
