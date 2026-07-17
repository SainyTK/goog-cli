use serde_json::json;

use super::change::{
    prepare_apply_list_change, prepare_apply_styles_change, prepare_edit_table_change,
    prepare_insert_image_change, prepare_insert_table_change, prepare_insert_text_change,
    prepare_replace_text_change, request_body_required_revision_id,
    set_request_body_required_revision_id, split_docs_request_bodies, write_docs_change_preview,
    ApplyListCommand, ApplyStylesCommand, EditTableCommand, InsertImageCommand, InsertImageFit,
    InsertTableCommand, InsertTextCommand, ReplaceTextCommand,
};
use super::image_fit::{ImageFitConstraints, SourceImageDimensions};
use super::map::{
    build_document_map, DocumentLocation, DocumentMap, DocumentMapEntry, DocumentMapEntryKind,
    DocumentRange, InsertTextSelector, LocationConfidence, RangeSelector,
};
use crate::cli::DocsListType;

fn searchable_map() -> DocumentMap {
    build_document_map(&json!({
        "documentId": "document-123",
        "title": "Plan",
        "revisionId": "rev-search",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 14,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "NORMAL_TEXT" },
                        "elements": [
                            {
                                "startIndex": 1,
                                "endIndex": 14,
                                "textRun": { "content": "Project Plan\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 14,
                    "endIndex": 37,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "NORMAL_TEXT" },
                        "elements": [
                            {
                                "startIndex": 14,
                                "endIndex": 37,
                                "textRun": { "content": "No matching text here\n" }
                            }
                        ]
                    }
                }
            ]
        }
    }))
}

fn preview_json(change: &super::change::PreparedDocsChange) -> serde_json::Value {
    let mut out = Vec::new();
    write_docs_change_preview(&mut out, change, true).unwrap();
    serde_json::from_slice(&out).unwrap()
}

#[test]
fn text_changes_build_native_requests_and_compatible_dry_run_preview() {
    let document_map = searchable_map();

    let insert = prepare_insert_text_change(
        &document_map,
        &InsertTextCommand {
            document_id: "document-123".into(),
            text: "Hello ".into(),
            selector: InsertTextSelector::Index(9),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-required".into()),
        },
    )
    .unwrap();
    let insert = preview_json(&insert);
    assert_eq!(insert["revisionId"], "rev-search");
    assert_eq!(insert["location"]["index"], 9);
    assert_eq!(
        insert["requestBody"]["requests"][0]["insertText"]["location"]["index"],
        9
    );
    assert_eq!(
        insert["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-required"
    );
    assert_eq!(insert["preview"]["after"], "Project Hello Plan");

    let replace = prepare_replace_text_change(
        &document_map,
        &ReplaceTextCommand {
            document_id: "document-123".into(),
            old_text: "Plan".into(),
            new_text: "Strategy".into(),
            match_number: None,
            all: false,
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
    )
    .unwrap();
    let replace = preview_json(&replace);
    assert_eq!(replace["ranges"][0]["startIndex"], 9);
    assert_eq!(
        replace["requestBody"]["requests"][0]["deleteContentRange"]["range"]["startIndex"],
        9
    );
    assert_eq!(
        replace["requestBody"]["requests"][1]["insertText"]["text"],
        "Strategy"
    );
    assert_eq!(
        replace["preview"]["changes"][0]["after"],
        "Project Strategy"
    );
}

#[test]
fn image_table_style_and_list_changes_build_native_requests() {
    let document_map = searchable_map();

    let image = prepare_insert_image_change(
        &document_map,
        &InsertImageCommand {
            document_id: "document-123".into(),
            image_uri: "https://example.test/image.png".into(),
            width: Some(468.0),
            height: Some(500.0),
            fit: None,
            selector: InsertTextSelector::AfterText("Project".into()),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-image".into()),
        },
    )
    .unwrap();
    let image = preview_json(&image);
    assert_eq!(
        image["requestBody"]["requests"][0]["insertInlineImage"]["location"]["index"],
        8
    );
    assert_eq!(
        image["requestBody"]["requests"][0]["insertInlineImage"]["objectSize"],
        serde_json::json!({
            "width": { "magnitude": 468.0, "unit": "PT" },
            "height": { "magnitude": 500.0, "unit": "PT" }
        })
    );
    assert_eq!(image["preview"]["after"], "Project[inline image] Plan");

    let temp_dir = tempfile::tempdir().unwrap();
    let table_data = temp_dir.path().join("table.csv");
    std::fs::write(&table_data, "A1,B1\nA2,B2\n").unwrap();
    let table = prepare_insert_table_change(
        &document_map,
        &InsertTableCommand {
            document_id: "document-123".into(),
            data: Some(table_data.to_string_lossy().into_owned()),
            rows: None,
            columns: None,
            selector: InsertTextSelector::Index(14),
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
    )
    .unwrap();
    let table = preview_json(&table);
    assert_eq!(
        table["requestBody"]["requests"][0]["insertTable"]["rows"],
        2
    );
    assert_eq!(
        table["requestBody"]["requests"][1]["insertText"]["text"],
        "B2"
    );

    let wide_table_data = temp_dir.path().join("wide-table.tsv");
    std::fs::write(
        &wide_table_data,
        "Time\tEvent\tOutcome\n09:12\tDeployment completed\tTimeout changed\n09:18\tAlert fired\tRetries increased\n09:30\tMitigation applied\tAccess restored\n",
    )
    .unwrap();
    let wide_table = prepare_insert_table_change(
        &document_map,
        &InsertTableCommand {
            document_id: "document-123".into(),
            data: Some(wide_table_data.to_string_lossy().into_owned()),
            rows: None,
            columns: None,
            selector: InsertTextSelector::Index(438),
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
    )
    .unwrap();
    let wide_table = preview_json(&wide_table);
    assert_eq!(
        wide_table["requestBody"]["requests"][1]["insertText"]["location"]["index"],
        467
    );
    assert_eq!(
        wide_table["requestBody"]["requests"][1]["insertText"]["text"],
        "Access restored"
    );

    let styles = prepare_apply_styles_change(
        &document_map,
        &ApplyStylesCommand {
            document_id: "document-123".into(),
            selector: RangeSelector::IndexRange {
                start_index: 1,
                end_index: 13,
            },
            bold: true,
            italic: false,
            font_size: Some(16.0),
            foreground_color: Some("#336699".into()),
            heading: Some("HEADING_2".into()),
            style_json: None,
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        None,
    )
    .unwrap();
    let styles = preview_json(&styles);
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["namedStyleType"],
        "HEADING_2"
    );
    assert_eq!(
        styles["requestBody"]["requests"][1]["updateTextStyle"]["fields"],
        "bold,fontSize,foregroundColor"
    );

    let list = prepare_apply_list_change(
        &document_map,
        &ApplyListCommand {
            document_id: "document-123".into(),
            selector: RangeSelector::IndexRange {
                start_index: 1,
                end_index: 13,
            },
            list_type: Some(DocsListType::Checkbox),
            preset: None,
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        None,
    )
    .unwrap();
    let list = preview_json(&list);
    assert_eq!(
        list["requestBody"]["requests"][0]["createParagraphBullets"]["bulletPreset"],
        "BULLET_CHECKBOX"
    );
}

#[test]
fn aspect_fit_image_change_builds_resolved_native_size() {
    let document_map = searchable_map();
    let image = prepare_insert_image_change(
        &document_map,
        &InsertImageCommand {
            document_id: "document-123".into(),
            image_uri: "https://example.test/portrait.png".into(),
            width: None,
            height: None,
            fit: Some(InsertImageFit {
                source: SourceImageDimensions {
                    width_px: 1_440,
                    height_px: 2_534,
                },
                constraints: ImageFitConstraints {
                    max_width_pt: Some(468.0),
                    max_height_pt: Some(500.0),
                    allow_upscale: false,
                },
            }),
            selector: InsertTextSelector::AfterText("Project".into()),
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
    )
    .unwrap();
    let image = preview_json(&image);

    assert_eq!(
        image["requestBody"]["requests"][0]["insertInlineImage"]["objectSize"],
        serde_json::json!({
            "width": { "magnitude": 284.136, "unit": "PT" },
            "height": { "magnitude": 500.0, "unit": "PT" }
        })
    );
    assert_eq!(
        image["preview"]["imageSizing"],
        serde_json::json!({
            "sourceDimensions": { "widthPixels": 1_440, "heightPixels": 2_534 },
            "nativeDimensions": { "widthPoints": 1_080.0, "heightPoints": 1_900.5 },
            "constraints": { "maxWidthPoints": 468.0, "maxHeightPoints": 500.0 },
            "scale": 0.263,
            "finalDimensions": { "widthPoints": 284.136, "heightPoints": 500.0 },
            "upscaled": false
        })
    );
}

#[test]
fn edit_table_and_split_apply_style_requests_are_module_level_behavior() {
    let table_map = DocumentMap {
        document_id: Some("document-123".into()),
        title: Some("Table".into()),
        revision_id: Some("rev-table".into()),
        entries: vec![DocumentMapEntry {
            entry: 1,
            location: DocumentLocation {
                index: Some(1),
                page: None,
                content_line: 1,
                confidence: LocationConfidence::Unknown,
            },
            kind: DocumentMapEntryKind::Table,
            style: None,
            preview: "A | B / C | D".into(),
            image_handle: None,
            object_id: None,
            layout_metadata: None,
            rows: Some(2),
            columns: Some(2),
            table_handle: Some("table-1".into()),
            table_cells: vec![
                vec![range(4, 5), range(8, 9)],
                vec![range(12, 13), range(16, 17)],
            ],
        }],
        document_locations: Vec::new(),
        text_blocks: Vec::new(),
        insertion_locations: Vec::new(),
    };
    let temp_dir = tempfile::tempdir().unwrap();
    let table_data = temp_dir.path().join("table.csv");
    std::fs::write(&table_data, "New A,New B\nNew C,New D\n").unwrap();

    let edit = prepare_edit_table_change(
        &table_map,
        &EditTableCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            data: table_data.to_string_lossy().into_owned(),
            resize: false,
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-required".into()),
        },
    )
    .unwrap();
    let edit = preview_json(&edit);
    assert_eq!(
        edit["requestBody"]["requests"][0]["deleteContentRange"]["range"]["startIndex"],
        16
    );
    assert_eq!(
        edit["requestBody"]["requests"][1]["insertText"]["text"],
        "New D"
    );

    let style_body = json!({
        "requests": [
            { "updateParagraphStyle": { "fields": "namedStyleType" } },
            { "updateTextStyle": { "fields": "bold" } }
        ],
        "writeControl": { "requiredRevisionId": "rev-1" }
    });
    let split = split_docs_request_bodies(&style_body, "style apply");
    assert_eq!(split.len(), 2);
    assert!(split[0]["writeControl"].is_null());
    let mut second = split[1].clone();
    set_request_body_required_revision_id(&mut second, Some("rev-2"));
    assert_eq!(
        request_body_required_revision_id(&second).as_deref(),
        Some("rev-2")
    );
}

fn range(start_index: i64, end_index: i64) -> DocumentRange {
    DocumentRange {
        start_index,
        end_index,
        location: DocumentLocation {
            index: Some(start_index),
            page: None,
            content_line: 1,
            confidence: LocationConfidence::Unknown,
        },
        preview: format!("{start_index}..{end_index}"),
    }
}
