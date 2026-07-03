use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::DocsError;
use super::map::document_content;

const NAMED_STYLE_TYPES: &[&str] = &[
    "HEADING_1",
    "HEADING_2",
    "HEADING_3",
    "HEADING_4",
    "HEADING_5",
    "HEADING_6",
    "TITLE",
    "SUBTITLE",
    "NORMAL_TEXT",
];

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct StyleTemplate {
    pub document_id: String,
    pub source_revision_id: Option<String>,
    #[serde(default)]
    pub named_styles: HashMap<String, NamedStyleTemplate>,
    pub table: Option<TableStyleTemplate>,
    pub list: Option<ListStyleTemplate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct NamedStyleTemplate {
    pub text_style: TextStyleTemplate,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub paragraph_style: Option<Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TextStyleTemplate {
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub font_size_pt: Option<f64>,
    pub foreground_color: Option<String>,
}

impl TextStyleTemplate {
    pub fn is_empty(&self) -> bool {
        self.bold.is_none()
            && self.italic.is_none()
            && self.font_size_pt.is_none()
            && self.foreground_color.is_none()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableStyleTemplate {
    pub header_row: TableRowStyleTemplate,
    pub body_row: TableRowStyleTemplate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableRowStyleTemplate {
    pub background_color: Option<String>,
    pub text_style: TextStyleTemplate,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ListStyleTemplate {
    pub list_type: Option<String>,
    pub preset: String,
}

/// Extracts a [`StyleTemplate`] from a Google Docs Document JSON payload.
///
/// Returns `None` when the document has neither a `body` nor a `namedStyles`
/// key, which is the shape returned for a partial `--fields` fetch (e.g.
/// `--fields title`). Callers use that signal to skip writing to the cache
/// rather than overwrite a good cache entry with an empty one.
pub fn extract_style_template(document_id: &str, document: &Value) -> Option<StyleTemplate> {
    if document.get("body").is_none() && document.get("namedStyles").is_none() {
        return None;
    }

    Some(StyleTemplate {
        document_id: document_id.to_string(),
        source_revision_id: document
            .get("revisionId")
            .and_then(Value::as_str)
            .map(str::to_string),
        named_styles: extract_named_styles(document),
        table: extract_table_style(document),
        list: extract_list_style(document),
    })
}

fn extract_named_styles(document: &Value) -> HashMap<String, NamedStyleTemplate> {
    let mut named_styles = HashMap::new();
    for style_type in NAMED_STYLE_TYPES {
        if let Some(observed_style) = observed_named_style(document, style_type) {
            named_styles.insert((*style_type).to_string(), observed_style);
            continue;
        }

        if let Some(text_style) = default_named_style_text(document, style_type) {
            if !text_style.is_empty() {
                named_styles.insert(
                    (*style_type).to_string(),
                    NamedStyleTemplate {
                        text_style,
                        paragraph_style: None,
                    },
                );
            }
        }
    }
    named_styles
}

fn observed_named_style(document: &Value, style_type: &str) -> Option<NamedStyleTemplate> {
    for element in document_content(document) {
        let Some(paragraph) = element.get("paragraph") else {
            continue;
        };
        let named_style_type = paragraph
            .get("paragraphStyle")
            .and_then(|style| style.get("namedStyleType"))
            .and_then(Value::as_str);
        if named_style_type != Some(style_type) {
            continue;
        }
        if let Some(text_run_style) = first_text_run_style(paragraph) {
            let text_style = text_style_from_value(text_run_style);
            if !text_style.is_empty() {
                return Some(NamedStyleTemplate {
                    text_style,
                    paragraph_style: cached_paragraph_style(paragraph),
                });
            }
        }
    }
    None
}

fn cached_paragraph_style(paragraph: &Value) -> Option<Value> {
    let mut paragraph_style = paragraph.get("paragraphStyle")?.clone();
    let style_object = paragraph_style.as_object_mut()?;
    style_object.remove("namedStyleType");
    style_object.remove("headingId");
    if style_object.is_empty() {
        None
    } else {
        Some(paragraph_style)
    }
}

fn default_named_style_text(document: &Value, style_type: &str) -> Option<TextStyleTemplate> {
    let styles = document
        .get("namedStyles")
        .and_then(|named_styles| named_styles.get("styles"))
        .and_then(Value::as_array)?;
    let entry = styles
        .iter()
        .find(|entry| entry.get("namedStyleType").and_then(Value::as_str) == Some(style_type))?;
    let text_style = entry.get("textStyle")?;
    Some(text_style_from_value(text_style))
}

fn first_text_run_style(paragraph: &Value) -> Option<&Value> {
    paragraph
        .get("elements")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .find_map(|element| element.get("textRun").and_then(|run| run.get("textStyle")))
}

fn text_style_from_value(text_style: &Value) -> TextStyleTemplate {
    TextStyleTemplate {
        bold: text_style.get("bold").and_then(Value::as_bool),
        italic: text_style.get("italic").and_then(Value::as_bool),
        font_size_pt: text_style
            .get("fontSize")
            .and_then(|font_size| font_size.get("magnitude"))
            .and_then(Value::as_f64),
        foreground_color: text_style
            .get("foregroundColor")
            .and_then(|color| color.get("color"))
            .and_then(|color| color.get("rgbColor"))
            .map(rgb_to_hex),
    }
}

fn rgb_to_hex(rgb_color: &Value) -> String {
    let channel = |name: &str| rgb_color.get(name).and_then(Value::as_f64).unwrap_or(0.0);
    let red = (channel("red") * 255.0).round() as u8;
    let green = (channel("green") * 255.0).round() as u8;
    let blue = (channel("blue") * 255.0).round() as u8;
    format!("#{red:02X}{green:02X}{blue:02X}")
}

fn find_tables(document: &Value) -> Vec<&Value> {
    document_content(document)
        .filter_map(|element| element.get("table"))
        .collect()
}

fn extract_table_style(document: &Value) -> Option<TableStyleTemplate> {
    let tables = find_tables(document);
    let table = tables.into_iter().find(|table| {
        table
            .get("tableRows")
            .and_then(Value::as_array)
            .map(|rows| rows.len() > 1)
            .unwrap_or(false)
    })?;

    let rows = table.get("tableRows").and_then(Value::as_array)?;
    let header_row = rows.first()?;
    let body_row = rows.get(1)?;

    Some(TableStyleTemplate {
        header_row: extract_row_style(header_row),
        body_row: extract_row_style(body_row),
    })
}

fn extract_row_style(row: &Value) -> TableRowStyleTemplate {
    let cells = row
        .get("tableCells")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let background_color = cells.first().and_then(|cell| {
        cell.get("tableCellStyle")
            .and_then(|style| style.get("backgroundColor"))
            .and_then(|color| color.get("color"))
            .and_then(|color| color.get("rgbColor"))
            .map(rgb_to_hex)
    });

    let text_style = cells
        .iter()
        .find_map(|cell| first_cell_text_run_style(cell))
        .map(text_style_from_value)
        .unwrap_or_default();

    TableRowStyleTemplate {
        background_color,
        text_style,
    }
}

fn first_cell_text_run_style(cell: &Value) -> Option<&Value> {
    cell.get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|element| element.get("paragraph"))
        .find_map(first_text_run_style)
}

fn extract_list_style(document: &Value) -> Option<ListStyleTemplate> {
    let lists = document.get("lists").and_then(Value::as_object);
    let Some(lists) = lists else {
        return Some(ListStyleTemplate {
            list_type: None,
            preset: "BULLET_DISC_CIRCLE_SQUARE".to_string(),
        });
    };
    let Some((_, list)) = lists.iter().next() else {
        return Some(ListStyleTemplate {
            list_type: None,
            preset: "BULLET_DISC_CIRCLE_SQUARE".to_string(),
        });
    };

    let nesting_level = list
        .get("listProperties")
        .and_then(|props| props.get("nestingLevels"))
        .and_then(Value::as_array)
        .and_then(|levels| levels.first());

    let Some(nesting_level) = nesting_level else {
        return Some(ListStyleTemplate {
            list_type: None,
            preset: "BULLET_DISC_CIRCLE_SQUARE".to_string(),
        });
    };

    let glyph_type = nesting_level.get("glyphType").and_then(Value::as_str);
    let glyph_symbol = nesting_level.get("glyphSymbol").and_then(Value::as_str);

    if matches!(
        glyph_type,
        Some("DECIMAL") | Some("ALPHA") | Some("UPPER_ALPHA") | Some("ROMAN") | Some("UPPER_ROMAN")
    ) {
        return Some(ListStyleTemplate {
            list_type: Some("Numbered".to_string()),
            preset: "NUMBERED_DECIMAL_ALPHA_ROMAN".to_string(),
        });
    }

    if matches!(glyph_symbol, Some("☐") | Some("☑")) {
        return Some(ListStyleTemplate {
            list_type: Some("Checkbox".to_string()),
            preset: "CHECKBOX".to_string(),
        });
    }

    if matches!(glyph_symbol, Some("●") | Some("○") | Some("■") | Some("•")) {
        return Some(ListStyleTemplate {
            list_type: Some("Bullet".to_string()),
            preset: "BULLET_DISC_CIRCLE_SQUARE".to_string(),
        });
    }

    if glyph_type.is_some() || glyph_symbol.is_some() {
        return Some(ListStyleTemplate {
            list_type: Some("Dash".to_string()),
            preset: "BULLET_DIAMONDX_ARROW3D_SQUARE".to_string(),
        });
    }

    Some(ListStyleTemplate {
        list_type: None,
        preset: "BULLET_DISC_CIRCLE_SQUARE".to_string(),
    })
}

fn is_valid_document_id(document_id: &str) -> bool {
    !document_id.is_empty()
        && document_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

pub fn style_template_path(document_id: &str) -> Result<PathBuf, DocsError> {
    style_template_path_in(None, document_id)
}

/// Same as [`style_template_path`] but allows overriding the cache directory
/// (the directory that directly contains the per-document JSON files) for
/// tests, so unit tests never touch the real `~/.config/goog` directory.
pub(crate) fn style_template_path_in(
    cache_dir: Option<&Path>,
    document_id: &str,
) -> Result<PathBuf, DocsError> {
    if !is_valid_document_id(document_id) {
        return Err(DocsError::InvalidDocumentId(document_id.to_string()));
    }
    let dir = match cache_dir {
        Some(dir) => dir.to_path_buf(),
        None => dirs::config_dir()
            .ok_or(DocsError::ConfigDirNotFound)?
            .join("goog")
            .join("doc-styles"),
    };
    Ok(dir.join(format!("{document_id}.json")))
}

pub fn load_style_template(document_id: &str) -> Result<Option<StyleTemplate>, DocsError> {
    load_style_template_in(None, document_id)
}

pub(crate) fn load_style_template_in(
    cache_dir: Option<&Path>,
    document_id: &str,
) -> Result<Option<StyleTemplate>, DocsError> {
    load_style_template_from_path(&style_template_path_in(cache_dir, document_id)?)
}

pub(crate) fn load_style_template_from_path(
    path: &Path,
) -> Result<Option<StyleTemplate>, DocsError> {
    if !path.exists() {
        return Ok(None);
    }
    let contents = std::fs::read_to_string(path).map_err(DocsError::StyleTemplateIo)?;
    serde_json::from_str(&contents)
        .map(Some)
        .map_err(|e| DocsError::StyleTemplateMalformed(e.to_string()))
}

pub fn save_style_template(template: &StyleTemplate) -> Result<(), DocsError> {
    save_style_template_in(None, template)
}

pub(crate) fn save_style_template_in(
    cache_dir: Option<&Path>,
    template: &StyleTemplate,
) -> Result<(), DocsError> {
    save_style_template_to_path(
        template,
        &style_template_path_in(cache_dir, &template.document_id)?,
    )
}

pub(crate) fn save_style_template_to_path(
    template: &StyleTemplate,
    path: &Path,
) -> Result<(), DocsError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(DocsError::StyleTemplateIo)?;
    }
    let contents = serde_json::to_string_pretty(template)
        .map_err(|e| DocsError::StyleTemplateMalformed(e.to_string()))?;
    std::fs::write(path, contents).map_err(DocsError::StyleTemplateIo)
}
