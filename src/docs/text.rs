use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

use super::map::{document_content, paragraph_text, DocumentListGlyph, DocumentMap};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentTextExport {
    pub document_id: Option<String>,
    pub title: Option<String>,
    pub revision_id: Option<String>,
    pub text: String,
}

pub fn extract_document_text(document_map: &DocumentMap) -> DocumentTextExport {
    let mut renderer = DocumentTextRenderer::new(document_map);
    for element in document_content(&document_map.raw_document) {
        renderer.push_structural_element(element);
    }

    DocumentTextExport {
        document_id: document_map.document_id.clone(),
        title: document_map.title.clone(),
        revision_id: document_map.revision_id.clone(),
        text: renderer.finish(),
    }
}

struct DocumentTextRenderer<'a> {
    document_map: &'a DocumentMap,
    text: String,
    list_counters: HashMap<String, Vec<Option<i64>>>,
}

impl<'a> DocumentTextRenderer<'a> {
    fn new(document_map: &'a DocumentMap) -> Self {
        Self {
            document_map,
            text: String::new(),
            list_counters: HashMap::new(),
        }
    }

    fn push_structural_element(&mut self, element: &Value) {
        let rendered = self.render_structural_element(element);
        self.text.push_str(&rendered);
    }

    fn render_structural_element(&mut self, element: &Value) -> String {
        if let Some(paragraph) = element.get("paragraph") {
            self.render_paragraph(paragraph)
        } else if let Some(table) = element.get("table") {
            self.render_table(table)
        } else if let Some(table_of_contents) = element.get("tableOfContents") {
            self.render_content(table_of_contents.get("content"))
        } else {
            String::new()
        }
    }

    fn render_content(&mut self, content: Option<&Value>) -> String {
        let mut rendered = String::new();
        for element in content.and_then(Value::as_array).into_iter().flatten() {
            rendered.push_str(&self.render_structural_element(element));
        }
        rendered
    }

    fn render_paragraph(&mut self, paragraph: &Value) -> String {
        let mut text = paragraph_text(paragraph);
        text.retain(|character| character != '\u{e907}');
        if let Some((nesting_level, marker)) = self.list_marker(paragraph) {
            let indent = "  ".repeat(nesting_level);
            if let Some(marker) = marker.filter(|marker| !marker.is_empty()) {
                return format!("{indent}{marker} {text}");
            }
            return format!("{indent}{text}");
        }
        text
    }

    fn render_table(&mut self, table: &Value) -> String {
        let mut rendered = String::new();
        for row in table
            .get("tableRows")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            for (cell_index, cell) in row
                .get("tableCells")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                if cell_index > 0 {
                    rendered.push('\t');
                }
                rendered.push_str(&self.render_table_cell(cell));
            }
            rendered.push('\n');
        }
        rendered
    }

    fn render_table_cell(&mut self, cell: &Value) -> String {
        let mut rendered = self.render_content(cell.get("content"));
        if rendered.ends_with('\n') {
            rendered.pop();
        }
        rendered
    }

    fn list_marker(&mut self, paragraph: &Value) -> Option<(usize, Option<String>)> {
        let bullet = paragraph.get("bullet")?;
        let list_id = bullet.get("listId")?.as_str()?;
        let nesting_level = bullet
            .get("nestingLevel")
            .and_then(Value::as_i64)
            .unwrap_or_default();
        let nesting_level = usize::try_from(nesting_level).ok()?;
        let Some(list) = self
            .document_map
            .lists
            .iter()
            .find(|list| list.list_id == list_id)
        else {
            return Some((nesting_level, None));
        };
        let glyphs = list.glyphs.clone();
        let Some(current_glyph) = glyphs
            .iter()
            .find(|glyph| glyph.nesting_level == nesting_level as i64)
        else {
            return Some((nesting_level, None));
        };

        let counters = self.list_counters.entry(list_id.to_string()).or_default();
        if counters.len() <= nesting_level {
            counters.resize(nesting_level + 1, None);
        }
        for counter in counters.iter_mut().skip(nesting_level + 1) {
            *counter = None;
        }
        if current_glyph
            .glyph_type
            .as_deref()
            .is_some_and(is_ordered_glyph_type)
        {
            let start_number = current_glyph.start_number.unwrap_or(1);
            counters[nesting_level] = Some(
                counters[nesting_level]
                    .map(|counter| counter.saturating_add(1))
                    .unwrap_or(start_number),
            );
        }

        let marker = render_glyph_format(current_glyph, &glyphs, counters);
        Some((nesting_level, marker))
    }

    fn finish(self) -> String {
        self.text
    }
}

fn render_glyph_format(
    current_glyph: &DocumentListGlyph,
    glyphs: &[DocumentListGlyph],
    counters: &[Option<i64>],
) -> Option<String> {
    let Some(glyph_format) = current_glyph.glyph_format.as_deref() else {
        return render_glyph(current_glyph, counters);
    };
    let mut marker = glyph_format.to_string();
    for glyph in glyphs {
        let nesting_level = usize::try_from(glyph.nesting_level).ok()?;
        let placeholder = format!("%{nesting_level}");
        if !marker.contains(&placeholder) {
            continue;
        }
        let replacement = render_glyph(glyph, counters)?;
        marker = marker.replace(&placeholder, &replacement);
    }
    if (0..=8).any(|nesting_level| marker.contains(&format!("%{nesting_level}"))) {
        return None;
    }
    Some(marker)
}

fn is_ordered_glyph_type(glyph_type: &str) -> bool {
    matches!(
        glyph_type,
        "DECIMAL" | "ZERO_DECIMAL" | "ALPHA" | "UPPER_ALPHA" | "ROMAN" | "UPPER_ROMAN"
    )
}

fn render_glyph(glyph: &DocumentListGlyph, counters: &[Option<i64>]) -> Option<String> {
    if let Some(symbol) = &glyph.glyph_symbol {
        return Some(symbol.clone());
    }
    let nesting_level = usize::try_from(glyph.nesting_level).ok()?;
    let number = counters
        .get(nesting_level)
        .copied()
        .flatten()
        .unwrap_or_else(|| glyph.start_number.unwrap_or(1));
    match glyph.glyph_type.as_deref()? {
        "DECIMAL" => Some(number.to_string()),
        "ZERO_DECIMAL" if (0..=9).contains(&number) => Some(format!("0{number}")),
        "ZERO_DECIMAL" => Some(number.to_string()),
        "ALPHA" => Some(format_alpha(number, false)),
        "UPPER_ALPHA" => Some(format_alpha(number, true)),
        "ROMAN" => Some(format_roman(number, false)),
        "UPPER_ROMAN" => Some(format_roman(number, true)),
        "NONE" => Some(String::new()),
        _ => None,
    }
}

fn format_alpha(number: i64, uppercase: bool) -> String {
    let mut number = u64::try_from(number.max(1)).unwrap_or(1);
    let mut output = Vec::new();
    while number > 0 {
        number -= 1;
        let base = if uppercase { b'A' } else { b'a' };
        output.push(char::from(
            base + u8::try_from(number % 26).unwrap_or_default(),
        ));
        number /= 26;
    }
    output.iter().rev().collect()
}

fn format_roman(number: i64, uppercase: bool) -> String {
    let mut number = number.max(1);
    let mut output = String::new();
    for (value, numeral) in [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ] {
        while number >= value {
            output.push_str(numeral);
            number -= value;
        }
    }
    if uppercase {
        output.make_ascii_uppercase();
    }
    output
}
