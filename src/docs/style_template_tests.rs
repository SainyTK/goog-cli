use serde_json::json;
use tempfile::TempDir;

use super::style_template::{
    extract_style_template, load_style_template_from_path, load_style_template_in,
    save_style_template_in, ListStyleTemplate, NamedStyleTemplate, StyleTemplate,
    TableRowStyleTemplate, TableStyleTemplate, TextStyleTemplate,
};

fn document_with_heading_and_table() -> serde_json::Value {
    json!({
        "documentId": "doc-1",
        "revisionId": "rev-1",
        "namedStyles": {
            "styles": [
                {
                    "namedStyleType": "HEADING_1",
                    "textStyle": {
                        "bold": true,
                        "fontSize": { "magnitude": 20.0, "unit": "PT" }
                    }
                },
                {
                    "namedStyleType": "HEADING_2",
                    "textStyle": {
                        "italic": true
                    }
                }
            ]
        },
        "lists": {
            "list-1": {
                "listProperties": {
                    "nestingLevels": [
                        { "glyphType": "DECIMAL" }
                    ]
                }
            }
        },
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 1,
                                "endIndex": 10,
                                "textRun": {
                                    "content": "Overview\n",
                                    "textStyle": {
                                        "bold": true,
                                        "italic": true,
                                        "fontSize": { "magnitude": 24.0, "unit": "PT" },
                                        "foregroundColor": {
                                            "color": { "rgbColor": { "red": 1.0, "green": 0.0, "blue": 0.0 } }
                                        }
                                    }
                                }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 10,
                    "table": {
                        "tableRows": [
                            {
                                "tableCells": [
                                    {
                                        "tableCellStyle": {
                                            "backgroundColor": {
                                                "color": { "rgbColor": { "red": 0.2, "green": 0.2, "blue": 0.2 } }
                                            }
                                        },
                                        "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        {
                                                            "textRun": {
                                                                "content": "Header",
                                                                "textStyle": { "bold": true }
                                                            }
                                                        }
                                                    ]
                                                }
                                            }
                                        ]
                                    }
                                ]
                            },
                            {
                                "tableCells": [
                                    {
                                        "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        {
                                                            "textRun": {
                                                                "content": "Body",
                                                                "textStyle": { "italic": true }
                                                            }
                                                        }
                                                    ]
                                                }
                                            }
                                        ]
                                    }
                                ]
                            }
                        ]
                    }
                }
            ]
        }
    })
}

#[test]
fn extract_returns_none_when_body_and_named_styles_absent() {
    let document = json!({ "documentId": "doc-1", "title": "Only Title" });
    assert!(extract_style_template("doc-1", &document).is_none());
}

#[test]
fn extract_prefers_observed_heading_style_over_named_styles_default() {
    let document = document_with_heading_and_table();
    let template = extract_style_template("doc-1", &document).unwrap();

    let heading_1 = template.named_styles.get("HEADING_1").unwrap();
    // The body paragraph's own textRun textStyle (24pt, red) should win over
    // the namedStyles default (20pt, no color).
    assert_eq!(heading_1.text_style.font_size_pt, Some(24.0));
    assert_eq!(heading_1.text_style.bold, Some(true));
    assert_eq!(heading_1.text_style.italic, Some(true));
    assert_eq!(
        heading_1.text_style.foreground_color.as_deref(),
        Some("#FF0000")
    );
}

#[test]
fn extract_observed_heading_survives_non_paragraph_content_before_first_heading() {
    let document = json!({
        "documentId": "doc-1",
        "namedStyles": {
            "styles": [
                {
                    "namedStyleType": "HEADING_1",
                    "textStyle": {
                        "fontSize": { "magnitude": 20.0, "unit": "PT" }
                    }
                }
            ]
        },
        "body": {
            "content": [
                {
                    "table": {
                        "tableRows": [
                            { "tableCells": [ { "content": [] } ] }
                        ]
                    }
                },
                {
                    "startIndex": 10,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "textRun": {
                                    "content": "Observed heading\n",
                                    "textStyle": {
                                        "bold": true,
                                        "fontSize": { "magnitude": 15.0, "unit": "PT" },
                                        "foregroundColor": {
                                            "color": {
                                                "rgbColor": {
                                                    "green": 0.34901962,
                                                    "blue": 0.35686275
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        ]
                    }
                }
            ]
        }
    });

    let template = extract_style_template("doc-1", &document).unwrap();
    let heading_1 = template.named_styles.get("HEADING_1").unwrap();
    assert_eq!(heading_1.text_style.bold, Some(true));
    assert_eq!(heading_1.text_style.font_size_pt, Some(15.0));
    assert_eq!(
        heading_1.text_style.foreground_color.as_deref(),
        Some("#00595B")
    );
}

#[test]
fn extract_falls_back_to_named_styles_default_when_no_body_paragraph_matches() {
    let document = document_with_heading_and_table();
    let template = extract_style_template("doc-1", &document).unwrap();

    // No body paragraph uses HEADING_2, so it should fall back to the
    // namedStyles default entry.
    let heading_2 = template.named_styles.get("HEADING_2").unwrap();
    assert_eq!(heading_2.text_style.italic, Some(true));
    assert_eq!(heading_2.text_style.bold, None);
}

#[test]
fn extract_splits_table_header_and_body_row_styles() {
    let document = document_with_heading_and_table();
    let template = extract_style_template("doc-1", &document).unwrap();

    let table = template.table.unwrap();
    assert_eq!(
        table.header_row.background_color.as_deref(),
        Some("#333333")
    );
    assert_eq!(table.header_row.text_style.bold, Some(true));
    assert_eq!(table.body_row.background_color, None);
    assert_eq!(table.body_row.text_style.italic, Some(true));
}

#[test]
fn extract_returns_no_table_for_single_row_table() {
    let document = json!({
        "documentId": "doc-1",
        "body": {
            "content": [
                {
                    "table": {
                        "tableRows": [
                            { "tableCells": [ { "content": [] } ] }
                        ]
                    }
                }
            ]
        }
    });
    let template = extract_style_template("doc-1", &document).unwrap();
    assert!(template.table.is_none());
}

#[test]
fn extract_maps_decimal_glyph_to_numbered_preset() {
    let document = document_with_heading_and_table();
    let template = extract_style_template("doc-1", &document).unwrap();

    let list = template.list.unwrap();
    assert_eq!(list.list_type.as_deref(), Some("Numbered"));
    assert_eq!(list.preset, "NUMBERED_DECIMAL_ALPHA_ROMAN");
}

#[test]
fn extract_maps_disc_glyph_to_bullet_preset() {
    let document = json!({
        "documentId": "doc-1",
        "body": { "content": [] },
        "lists": {
            "list-1": {
                "listProperties": {
                    "nestingLevels": [
                        { "glyphSymbol": "●" }
                    ]
                }
            }
        }
    });
    let template = extract_style_template("doc-1", &document).unwrap();

    let list = template.list.unwrap();
    assert_eq!(list.list_type.as_deref(), Some("Bullet"));
    assert_eq!(list.preset, "BULLET_DISC_CIRCLE_SQUARE");
}

#[test]
fn extract_defaults_list_when_no_lists_present() {
    let document = json!({ "documentId": "doc-1", "body": { "content": [] } });
    let template = extract_style_template("doc-1", &document).unwrap();

    let list = template.list.unwrap();
    assert_eq!(list.list_type, None);
    assert_eq!(list.preset, "BULLET_DISC_CIRCLE_SQUARE");
}

#[test]
fn save_and_load_round_trips_through_tempdir() {
    let dir = TempDir::new().unwrap();
    let template = StyleTemplate {
        document_id: "doc-1".into(),
        source_revision_id: Some("rev-1".into()),
        named_styles: [(
            "HEADING_1".to_string(),
            NamedStyleTemplate {
                text_style: TextStyleTemplate {
                    bold: Some(true),
                    italic: None,
                    font_size_pt: Some(20.0),
                    foreground_color: None,
                },
                paragraph_style: None,
            },
        )]
        .into_iter()
        .collect(),
        table: Some(TableStyleTemplate {
            header_row: TableRowStyleTemplate {
                background_color: Some("#333333".into()),
                text_style: TextStyleTemplate::default(),
            },
            body_row: TableRowStyleTemplate::default(),
        }),
        list: Some(ListStyleTemplate {
            list_type: Some("Bullet".into()),
            preset: "BULLET_DISC_CIRCLE_SQUARE".into(),
        }),
    };

    save_style_template_in(Some(dir.path()), &template).unwrap();
    let loaded = load_style_template_in(Some(dir.path()), "doc-1").unwrap();
    assert_eq!(loaded, Some(template));
}

#[test]
fn load_returns_none_when_cache_file_missing() {
    let dir = TempDir::new().unwrap();
    let loaded = load_style_template_in(Some(dir.path()), "doc-missing").unwrap();
    assert_eq!(loaded, None);

    let missing_path = dir.path().join("does-not-exist.json");
    assert_eq!(load_style_template_from_path(&missing_path).unwrap(), None);
}
