use serde_json::json;

use super::map::build_document_map;
use super::{extract_document_text, DocumentTextExport};

const TEXT_EXPORT_DOCUMENT_FIXTURE: &str =
    include_str!("../../tests/fixtures/docs/text_export_document.json");

#[test]
fn document_text_export_matches_docs_response_fixture() {
    let document = serde_json::from_str(TEXT_EXPORT_DOCUMENT_FIXTURE).unwrap();
    let export = extract_document_text(&build_document_map(&document));

    assert_eq!(export.document_id.as_deref(), Some("document-fixture"));
    assert_eq!(export.title.as_deref(), Some("Text export fixture"));
    assert_eq!(export.revision_id.as_deref(), Some("revision-fixture"));
    assert_eq!(
        export.text,
        "Overview\n1. First item\n  a) Nested item\nName\tValue\nAlpha\tOne\nEnding paragraph\n"
    );
}

#[test]
fn document_text_export_preserves_paragraphs_and_split_runs() {
    let document_map = build_document_map(&json!({
        "documentId": "document-123",
        "title": "Meeting notes",
        "revisionId": "revision-456",
        "body": {
            "content": [
                {
                    "paragraph": {
                        "elements": [
                            { "textRun": { "content": "Hello " } },
                            { "textRun": { "content": "world\n" } }
                        ]
                    }
                },
                {
                    "paragraph": {
                        "elements": [
                            { "textRun": { "content": "Second paragraph\n" } }
                        ]
                    }
                }
            ]
        }
    }));

    let export = extract_document_text(&document_map);
    assert_eq!(
        export,
        DocumentTextExport {
            document_id: Some("document-123".into()),
            title: Some("Meeting notes".into()),
            revision_id: Some("revision-456".into()),
            text: "Hello world\nSecond paragraph\n".into(),
        }
    );
    assert_eq!(
        serde_json::to_value(export).unwrap(),
        json!({
            "documentId": "document-123",
            "title": "Meeting notes",
            "revisionId": "revision-456",
            "text": "Hello world\nSecond paragraph\n"
        })
    );
}

#[test]
fn document_text_export_preserves_blank_paragraphs_without_extra_output() {
    let document_map = build_document_map(&json!({
        "body": {
            "content": [
                text_paragraph("First"),
                text_paragraph(""),
                text_paragraph("Last")
            ]
        }
    }));

    assert_eq!(extract_document_text(&document_map).text, "First\n\nLast\n");
}

#[test]
fn document_text_export_renders_nested_google_list_glyphs() {
    let document_map = build_document_map(&json!({
        "lists": {
            "list-abc": {
                "listProperties": {
                    "nestingLevels": [
                        { "glyphSymbol": "*" },
                        { "glyphSymbol": "o" }
                    ]
                }
            }
        },
        "body": {
            "content": [
                {
                    "paragraph": {
                        "bullet": { "listId": "list-abc", "nestingLevel": 0 },
                        "elements": [{ "textRun": { "content": "Top level\n" } }]
                    }
                },
                {
                    "paragraph": {
                        "bullet": { "listId": "list-abc", "nestingLevel": 1 },
                        "elements": [{ "textRun": { "content": "Nested\n" } }]
                    }
                },
                {
                    "paragraph": {
                        "bullet": { "listId": "list-abc", "nestingLevel": 0 },
                        "elements": [{ "textRun": { "content": "Top level again\n" } }]
                    }
                }
            ]
        }
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "* Top level\n  o Nested\n* Top level again\n"
    );
}

#[test]
fn document_text_export_formats_and_resets_nested_numbering() {
    let document_map = build_document_map(&json!({
        "lists": {
            "list-numbered": {
                "listProperties": {
                    "nestingLevels": [
                        {
                            "glyphType": "DECIMAL",
                            "glyphFormat": "%0.",
                            "startNumber": 3
                        },
                        {
                            "glyphType": "ALPHA",
                            "glyphFormat": "%0.%1)",
                            "startNumber": 1
                        }
                    ]
                }
            }
        },
        "body": {
            "content": [
                numbered_paragraph(0, "First"),
                numbered_paragraph(1, "First child"),
                numbered_paragraph(1, "Second child"),
                numbered_paragraph(0, "Second"),
                numbered_paragraph(1, "Reset child")
            ]
        }
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "3. First\n  3.a) First child\n  3.b) Second child\n4. Second\n  4.a) Reset child\n"
    );
}

fn numbered_paragraph(nesting_level: i64, text: &str) -> serde_json::Value {
    json!({
        "paragraph": {
            "bullet": {
                "listId": "list-numbered",
                "nestingLevel": nesting_level
            },
            "elements": [{ "textRun": { "content": format!("{text}\n") } }]
        }
    })
}

#[test]
fn document_text_export_supports_google_ordered_glyph_types() {
    let document_map = build_document_map(&json!({
        "lists": {
            "list-mixed": {
                "listProperties": {
                    "nestingLevels": [
                        {
                            "glyphType": "ZERO_DECIMAL",
                            "glyphFormat": "%0.",
                            "startNumber": 9
                        },
                        {
                            "glyphType": "UPPER_ALPHA",
                            "glyphFormat": "%1)",
                            "startNumber": 27
                        },
                        {
                            "glyphType": "ROMAN",
                            "glyphFormat": "%2.",
                            "startNumber": 4
                        }
                    ]
                }
            },
            "list-upper-roman": {
                "listProperties": {
                    "nestingLevels": [{
                        "glyphType": "UPPER_ROMAN",
                        "glyphFormat": "%0.",
                        "startNumber": 9
                    }]
                }
            },
            "list-without-glyph": {
                "listProperties": {
                    "nestingLevels": [{
                        "glyphType": "NONE",
                        "glyphFormat": "%0"
                    }]
                }
            }
        },
        "body": {
            "content": [
                list_paragraph("list-mixed", 0, "Leading zero"),
                list_paragraph("list-mixed", 0, "Ten"),
                list_paragraph("list-mixed", 1, "Letters"),
                list_paragraph("list-mixed", 2, "Roman"),
                list_paragraph("list-upper-roman", 0, "Upper Roman"),
                list_paragraph("list-without-glyph", 0, "No marker")
            ]
        }
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "09. Leading zero\n10. Ten\n  AA) Letters\n    iv. Roman\nIX. Upper Roman\nNo marker\n"
    );
}

#[test]
fn document_text_export_keeps_known_markers_and_fallback_indentation() {
    let document_map = build_document_map(&json!({
        "lists": {
            "list-partial": {
                "listProperties": {
                    "nestingLevels": [
                        {
                            "glyphType": "DECIMAL",
                            "glyphFormat": "%0."
                        },
                        {
                            "glyphType": "GLYPH_TYPE_UNSPECIFIED",
                            "glyphFormat": "%1."
                        }
                    ]
                }
            }
        },
        "body": {
            "content": [
                list_paragraph("list-partial", 0, "Known marker"),
                list_paragraph("list-partial", 1, "Unsupported marker")
            ]
        }
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "1. Known marker\n  Unsupported marker\n"
    );
}

fn list_paragraph(list_id: &str, nesting_level: i64, text: &str) -> serde_json::Value {
    json!({
        "paragraph": {
            "bullet": { "listId": list_id, "nestingLevel": nesting_level },
            "elements": [{ "textRun": { "content": format!("{text}\n") } }]
        }
    })
}

#[test]
fn document_text_export_renders_table_cells_and_rows_in_source_order() {
    let document_map = build_document_map(&json!({
        "body": {
            "content": [
                text_paragraph("Before"),
                {
                    "table": {
                        "tableRows": [
                            {
                                "tableCells": [
                                    table_cell("A1"),
                                    table_cell("B1")
                                ]
                            },
                            {
                                "tableCells": [
                                    table_cell("A2"),
                                    table_cell("")
                                ]
                            }
                        ]
                    }
                },
                text_paragraph("After")
            ]
        }
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "Before\nA1\tB1\nA2\t\nAfter\n"
    );
}

#[test]
fn document_text_export_preserves_trailing_blank_paragraph_in_table_cell() {
    let document_map = build_document_map(&json!({
        "body": {
            "content": [{
                "table": {
                    "tableRows": [{
                        "tableCells": [
                            {
                                "content": [
                                    text_paragraph("First"),
                                    text_paragraph("Second"),
                                    text_paragraph("")
                                ]
                            },
                            table_cell("Other")
                        ]
                    }]
                }
            }]
        }
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "First\nSecond\n\tOther\n"
    );
}

fn text_paragraph(text: &str) -> serde_json::Value {
    json!({
        "paragraph": {
            "elements": [{ "textRun": { "content": format!("{text}\n") } }]
        }
    })
}

fn table_cell(text: &str) -> serde_json::Value {
    json!({ "content": [text_paragraph(text)] })
}

#[test]
fn document_text_export_traverses_nested_tabs_in_display_order() {
    let document_map = build_document_map(&json!({
        "tabs": [
            {
                "tabProperties": { "tabId": "tab-1", "index": 0 },
                "documentTab": {
                    "body": { "content": [text_paragraph("First tab")] }
                },
                "childTabs": [{
                    "tabProperties": {
                        "tabId": "tab-1-child",
                        "parentTabId": "tab-1",
                        "index": 0
                    },
                    "documentTab": {
                        "lists": {
                            "child-list": {
                                "listProperties": {
                                    "nestingLevels": [{
                                        "glyphSymbol": "-",
                                        "glyphFormat": "%0"
                                    }]
                                }
                            }
                        },
                        "body": {
                            "content": [list_paragraph("child-list", 0, "Child tab")]
                        }
                    }
                }]
            },
            {
                "tabProperties": { "tabId": "tab-2", "index": 1 },
                "documentTab": {
                    "body": { "content": [text_paragraph("Second tab")] }
                }
            }
        ]
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "First tab\n- Child tab\nSecond tab\n"
    );
}

#[test]
fn document_text_export_omits_objects_and_empty_structural_content() {
    let empty_document = build_document_map(&json!({
        "body": { "content": [] }
    }));
    assert_eq!(extract_document_text(&empty_document).text, "");

    let object_only_document = build_document_map(&json!({
        "body": {
            "content": [
                {
                    "paragraph": {
                        "elements": [{
                            "inlineObjectElement": { "inlineObjectId": "image-1" }
                        }]
                    }
                },
                {
                    "paragraph": {
                        "positionedObjectIds": ["image-2"],
                        "elements": []
                    }
                },
                {
                    "paragraph": {
                        "elements": [{
                            "textRun": { "content": "Visible\u{e907} text\n" }
                        }]
                    }
                },
                { "sectionBreak": {} }
            ]
        }
    }));

    assert_eq!(
        extract_document_text(&object_only_document).text,
        "Visible text\n"
    );
}

#[test]
fn document_text_export_includes_table_of_contents_text_in_source_order() {
    let document_map = build_document_map(&json!({
        "body": {
            "content": [
                text_paragraph("Before"),
                {
                    "tableOfContents": {
                        "content": [
                            text_paragraph("First heading\t1"),
                            text_paragraph("Second heading\t2")
                        ]
                    }
                },
                text_paragraph("After")
            ]
        }
    }));

    assert_eq!(
        extract_document_text(&document_map).text,
        "Before\nFirst heading\t1\nSecond heading\t2\nAfter\n"
    );
}
