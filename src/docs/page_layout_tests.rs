use serde_json::{json, Value};

use super::page_layout::resolve_body_page_geometry;

fn dimension(magnitude: f64) -> Value {
    json!({ "magnitude": magnitude, "unit": "PT" })
}

fn document_style(width: f64, height: f64) -> Value {
    json!({
        "documentFormat": { "documentMode": "PAGES" },
        "pageSize": {
            "width": dimension(width),
            "height": dimension(height)
        },
        "marginTop": dimension(72.0),
        "marginBottom": dimension(72.0),
        "marginLeft": dimension(72.0),
        "marginRight": dimension(72.0)
    })
}

fn section(start_index: i64, style: Value) -> Value {
    json!({
        "startIndex": start_index,
        "endIndex": start_index + 1,
        "sectionBreak": { "sectionStyle": style }
    })
}

#[test]
fn resolves_letter_and_a4_body_geometry() {
    let letter = json!({
        "documentStyle": document_style(612.0, 792.0),
        "body": { "content": [section(0, json!({}))] }
    });
    let a4 = json!({
        "documentStyle": document_style(595.276, 841.89),
        "body": { "content": [section(0, json!({}))] }
    });

    let letter = resolve_body_page_geometry(&letter, None, 10).unwrap();
    let a4 = resolve_body_page_geometry(&a4, None, 10).unwrap();

    assert_eq!(letter.available_width_points, 468.0);
    assert_eq!(letter.available_height_points, 648.0);
    assert_eq!(a4.available_width_points, 451.276);
    assert_eq!(a4.available_height_points, 697.89);
}

#[test]
fn section_specific_margins_apply_after_the_latest_section_break() {
    let document = json!({
        "documentStyle": document_style(612.0, 792.0),
        "body": {
            "content": [
                section(0, json!({})),
                section(40, json!({
                    "marginLeft": dimension(90.0),
                    "marginRight": dimension(54.0),
                    "marginTop": dimension(36.0)
                }))
            ]
        }
    });

    let first = resolve_body_page_geometry(&document, None, 39).unwrap();
    let second = resolve_body_page_geometry(&document, None, 40).unwrap();

    assert_eq!(first.section_start_index, 0);
    assert_eq!(first.available_width_points, 468.0);
    assert_eq!(second.section_start_index, 40);
    assert_eq!(second.available_width_points, 468.0);
    assert_eq!(second.available_height_points, 684.0);
    assert_eq!(second.margin_left_points, 90.0);
    assert_eq!(second.margin_right_points, 54.0);
}

#[test]
fn implicit_first_section_falls_back_to_document_margins() {
    let document = json!({
        "documentStyle": document_style(612.0, 792.0),
        "body": {
            "content": [{
                "startIndex": 1,
                "endIndex": 2,
                "paragraph": { "elements": [] }
            }]
        }
    });

    let geometry = resolve_body_page_geometry(&document, None, 1).unwrap();

    assert_eq!(geometry.section_start_index, 0);
    assert_eq!(geometry.available_width_points, 468.0);
    assert_eq!(geometry.available_height_points, 648.0);
}

#[test]
fn resolves_geometry_from_the_requested_nested_tab() {
    let document = json!({
        "tabs": [{
            "tabProperties": { "tabId": "first" },
            "documentTab": {
                "documentStyle": document_style(612.0, 792.0),
                "body": { "content": [section(0, json!({}))] }
            },
            "childTabs": [{
                "tabProperties": { "tabId": "nested" },
                "documentTab": {
                    "documentStyle": document_style(595.276, 841.89),
                    "body": { "content": [section(0, json!({
                        "marginLeft": dimension(50.0),
                        "marginRight": dimension(50.0)
                    }))] }
                }
            }]
        }]
    });

    let geometry = resolve_body_page_geometry(&document, Some("nested"), 1).unwrap();

    assert_eq!(geometry.tab_id.as_deref(), Some("nested"));
    assert_eq!(geometry.available_width_points, 495.276);
}

#[test]
fn pageless_documents_return_an_actionable_error() {
    let document = json!({
        "documentStyle": {
            "documentFormat": { "documentMode": "PAGELESS" }
        },
        "body": { "content": [section(0, json!({}))] }
    });

    let error = resolve_body_page_geometry(&document, None, 1).unwrap_err();

    assert_eq!(
        error.to_string(),
        "--fit-page is unavailable for pageless documents; provide --max-width and --max-height"
    );
}

#[test]
fn malformed_page_geometry_fails_instead_of_claiming_that_content_fits() {
    let mut style = document_style(100.0, 100.0);
    style["marginLeft"] = dimension(60.0);
    style["marginRight"] = dimension(60.0);
    let document = json!({
        "documentStyle": style,
        "body": { "content": [section(0, json!({}))] }
    });

    let error = resolve_body_page_geometry(&document, None, 1).unwrap_err();

    assert_eq!(
        error.to_string(),
        "page margins leave no positive body area for --fit-page"
    );
}
