use serde_json::json;

use super::change::{
    prepare_apply_list_change, prepare_apply_styles_change, prepare_edit_table_change,
    prepare_insert_image_change, prepare_insert_table_change, prepare_insert_text_change,
    prepare_replace_text_change, prepare_set_table_column_widths_change,
    prepare_style_table_row_change, request_body_required_revision_id,
    set_request_body_required_revision_id, split_docs_request_bodies, write_docs_change_preview,
    ApplyListCommand, ApplyStylesCommand, EditTableCommand, InsertImageCommand, InsertTableCommand,
    InsertTextCommand, ReplaceTextCommand, SetTableColumnWidthsCommand, StyleTableRowCommand,
};
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
            selector: InsertTextSelector::AfterText("Project".into()),
            width: Some(468.0),
            height: Some(240.0),
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
        json!({
            "width": { "magnitude": 468.0, "unit": "PT" },
            "height": { "magnitude": 240.0, "unit": "PT" }
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
        table["requestBody"]["requests"].as_array().unwrap().len(),
        1
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
        wide_table["requestBody"]["requests"]
            .as_array()
            .unwrap()
            .len(),
        1
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
            underline: true,
            font_size: Some(16.0),
            font_family: Some("Bai Jamjuree".into()),
            foreground_color: Some("#336699".into()),
            alignment: Some(crate::cli::DocsParagraphAlignment::Center),
            space_above: Some(6.0),
            space_below: Some(10.0),
            line_spacing: Some(115.0),
            indent_start: Some(36.0),
            indent_end: Some(12.0),
            indent_first_line: Some(18.0),
            keep_with_next: true,
            keep_lines_together: true,
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
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]["alignment"],
        "CENTER"
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["spaceAbove"],
        serde_json::json!({ "magnitude": 6.0, "unit": "PT" })
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["spaceBelow"],
        serde_json::json!({ "magnitude": 10.0, "unit": "PT" })
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["lineSpacing"],
        115.0
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["keepWithNext"],
        true
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["keepLinesTogether"],
        true
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["fields"],
        "namedStyleType,alignment,spaceAbove,spaceBelow,lineSpacing,indentStart,indentEnd,indentFirstLine,keepWithNext,keepLinesTogether"
    );
    assert_eq!(
        styles["requestBody"]["requests"][1]["updateTextStyle"]["fields"],
        "bold,underline,fontSize,weightedFontFamily,foregroundColor"
    );
    assert_eq!(
        styles["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["underline"],
        true
    );
    assert_eq!(
        styles["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["weightedFontFamily"]
            ["fontFamily"],
        "Bai Jamjuree"
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
fn paragraph_spacing_rejects_invalid_point_values() {
    let document_map = searchable_map();
    let command = ApplyStylesCommand {
        document_id: "document-123".into(),
        selector: RangeSelector::IndexRange {
            start_index: 1,
            end_index: 13,
        },
        bold: false,
        italic: false,
        underline: false,
        font_size: None,
        font_family: None,
        foreground_color: None,
        alignment: None,
        space_above: Some(-1.0),
        space_below: None,
        line_spacing: None,
        indent_start: None,
        indent_end: None,
        indent_first_line: None,
        keep_with_next: false,
        keep_lines_together: false,
        heading: None,
        style_json: None,
        dry_run: true,
        json: true,
        required_revision_id: None,
        no_auto_style: false,
    };

    let error = prepare_apply_styles_change(&document_map, &command, None).unwrap_err();
    assert_eq!(
        error.to_string(),
        "--space-above must be a finite, non-negative point value"
    );

    let error = prepare_apply_styles_change(
        &document_map,
        &ApplyStylesCommand {
            space_above: None,
            space_below: Some(f64::NAN),
            ..command.clone()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "--space-below must be a finite, non-negative point value"
    );

    let error = prepare_apply_styles_change(
        &document_map,
        &ApplyStylesCommand {
            space_above: None,
            space_below: None,
            line_spacing: Some(0.0),
            ..command.clone()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "--line-spacing must be a finite, positive percentage"
    );

    let error = prepare_apply_styles_change(
        &document_map,
        &ApplyStylesCommand {
            space_above: None,
            indent_start: Some(-1.0),
            ..command.clone()
        },
        None,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "--indent-start must be a finite, non-negative point value"
    );

    let error = prepare_apply_styles_change(
        &document_map,
        &ApplyStylesCommand {
            space_above: None,
            indent_first_line: Some(f64::INFINITY),
            ..command
        },
        None,
    )
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "--indent-first-line must be a finite, non-negative point value"
    );
}

#[test]
fn image_dimensions_must_be_positive_and_finite() {
    let command = InsertImageCommand {
        document_id: "document-123".into(),
        image_uri: "https://example.test/image.png".into(),
        selector: InsertTextSelector::AfterText("Project".into()),
        width: Some(0.0),
        height: Some(240.0),
        dry_run: true,
        json: true,
        required_revision_id: None,
    };

    let error = prepare_insert_image_change(&searchable_map(), &command).unwrap_err();
    assert!(error.to_string().contains("--width"));

    let error = prepare_insert_image_change(
        &searchable_map(),
        &InsertImageCommand {
            width: Some(468.0),
            height: Some(f64::INFINITY),
            ..command
        },
    )
    .unwrap_err();
    assert!(error.to_string().contains("--height"));
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

    let style = prepare_style_table_row_change(
        &table_map,
        &StyleTableRowCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            row: 1,
            background_color: "#D9EAF7".into(),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-required".into()),
        },
    )
    .unwrap();
    let style = preview_json(&style);
    let update = &style["requestBody"]["requests"][0]["updateTableCellStyle"];
    assert_eq!(update["tableRange"]["tableCellLocation"]["rowIndex"], 0);
    assert_eq!(update["tableRange"]["columnSpan"], 2);
    assert_eq!(update["fields"], "backgroundColor");
    assert_eq!(
        update["tableCellStyle"]["backgroundColor"]["color"]["rgbColor"],
        json!({
            "red": 217.0 / 255.0,
            "green": 234.0 / 255.0,
            "blue": 247.0 / 255.0
        })
    );

    let columns = prepare_set_table_column_widths_change(
        &table_map,
        &SetTableColumnWidthsCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            widths: vec![104.25, 363.75],
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-required".into()),
        },
    )
    .unwrap();
    let columns = preview_json(&columns);
    let requests = columns["requestBody"]["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0]["updateTableColumnProperties"]["columnIndices"],
        json!([0])
    );
    assert_eq!(
        requests[0]["updateTableColumnProperties"]["tableColumnProperties"]["width"],
        json!({ "magnitude": 104.25, "unit": "PT" })
    );
    assert_eq!(
        requests[1]["updateTableColumnProperties"]["tableColumnProperties"]["widthType"],
        "FIXED_WIDTH"
    );
    assert_eq!(
        requests[1]["updateTableColumnProperties"]["fields"],
        "width,widthType"
    );

    let wrong_count = prepare_set_table_column_widths_change(
        &table_map,
        &SetTableColumnWidthsCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            widths: vec![468.0],
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
    )
    .unwrap_err();
    assert!(wrong_count.to_string().contains("requires 2 values"));

    let too_narrow = prepare_set_table_column_widths_change(
        &table_map,
        &SetTableColumnWidthsCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            widths: vec![4.99, 463.01],
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
    )
    .unwrap_err();
    assert!(too_narrow.to_string().contains("at least 5 points"));

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
