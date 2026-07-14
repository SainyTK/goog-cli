use chrono::{Duration, Utc};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::state::{
    load_runtime_state_from_path, resource_key, save_runtime_state_to_path, RuntimeState,
};
use crate::auth::testing::MemoryStore;
use crate::cli::{DocsCompareScope, DocsListType, DocsMapType, DocsSectionBreakType};
use crate::docs::change::{
    ApplyListCommand, ApplyStylesCommand, CreateFooterCommand, CreateFootnoteCommand,
    CreateHeaderCommand, CreateNamedRangeCommand, DeleteNamedRangeCommand, EditTableCommand,
    InsertImageCommand, InsertPageBreakCommand, InsertSectionBreakCommand, InsertTableCommand,
    InsertTextCommand, PinTableHeaderRowsCommand, ReplaceTextCommand, SetTableColumnWidthsCommand,
    StyleTableRowCommand,
};
use crate::docs::map::{build_document_map, ContentSelector, InsertTextSelector, RangeSelector};
use crate::docs::style_template::{
    load_style_template_in, save_style_template_in, ListStyleTemplate, NamedStyleTemplate,
    StyleTemplate, TextStyleTemplate,
};
use crate::docs::DOCS_SCOPE;
use crate::drive::{DriveError, DRIVE_SCOPE};

use super::docs::*;

fn test_config() -> Config {
    Config {
        oauth_app: Some(OAuthAppConfig {
            client_id: "client-123".into(),
            client_secret: "secret-456".into(),
            app_type: OAuthAppType::Desktop,
        }),
        settings: Some(SettingsConfig {
            active_account: Some("alice@example.com".into()),
            output: None,
        }),
        accounts: vec!["alice@example.com".into()],
    }
}

fn comparison_preview(
    max_differences: usize,
    difference_pattern: Option<&str>,
) -> DocumentComparisonPreview<'_> {
    DocumentComparisonPreview {
        max_differences,
        difference_pattern,
    }
}

fn docs_token() -> Token {
    scoped_docs_token("docs-write-access")
}

fn scoped_docs_token(access_token: &str) -> Token {
    Token {
        access_token: access_token.into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![DOCS_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &docs_token())
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

fn dry_run_apply_list_command(
    selector: RangeSelector,
    list_type: DocsListType,
) -> ApplyListCommand {
    ApplyListCommand {
        document_id: "document-123".into(),
        selector,
        list_type: Some(list_type),
        preset: None,
        dry_run: true,
        json: true,
        required_revision_id: None,
        no_auto_style: false,
    }
}

fn multi_account_config() -> Config {
    Config {
        oauth_app: Some(OAuthAppConfig {
            client_id: "client-123".into(),
            client_secret: "secret-456".into(),
            app_type: OAuthAppType::Desktop,
        }),
        settings: Some(SettingsConfig {
            active_account: Some("alice@example.com".into()),
            output: None,
        }),
        accounts: vec![
            "alice@example.com".into(),
            "bob@example.com".into(),
            "carol@example.com".into(),
        ],
    }
}

fn multi_account_store() -> MemoryStore {
    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &scoped_docs_token("alice-access"))
        .unwrap();
    store
        .save_token("bob@example.com", &scoped_docs_token("bob-access"))
        .unwrap();
    store
        .save_token("carol@example.com", &scoped_docs_token("carol-access"))
        .unwrap();
    store
}

#[tokio::test]
async fn run_get_prints_document_json_to_stdout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "title": "Roadmap"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_get_to(
        &client,
        "document-123".into(),
        None,
        false,
        &mut out,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"title\":\"Roadmap\"}\n"
    );
}

#[tokio::test]
async fn run_get_refreshes_style_template_cache_for_full_document() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "title": "Styled",
            "revisionId": "rev-search",
            "body": {
                "content": [
                    {
                        "startIndex": 1,
                        "endIndex": 12,
                        "paragraph": {
                            "paragraphStyle": { "namedStyleType": "HEADING_1" },
                            "elements": [
                                {
                                    "startIndex": 1,
                                    "endIndex": 12,
                                    "textRun": {
                                        "content": "Overview\\n",
                                        "textStyle": {
                                            "bold": true,
                                            "fontSize": { "magnitude": 24.0, "unit": "PT" }
                                        }
                                    }
                                }
                            ]
                        }
                    }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let cache_dir = tempfile::tempdir().unwrap();

    run_get_to(
        &client,
        "document-123".into(),
        None,
        false,
        &mut out,
        Some(&documents_url),
        Some(cache_dir.path()),
    )
    .await
    .unwrap();

    let template = load_style_template_in(Some(cache_dir.path()), "document-123")
        .unwrap()
        .unwrap();
    assert_eq!(template.document_id, "document-123");
    assert_eq!(template.source_revision_id.as_deref(), Some("rev-search"));
    assert!(template.named_styles.contains_key("HEADING_1"));
}

#[tokio::test]
async fn run_get_partial_fields_does_not_overwrite_existing_style_template_cache() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "title": "Only Title"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let cache_dir = tempfile::tempdir().unwrap();
    let existing = StyleTemplate {
        document_id: "document-123".into(),
        source_revision_id: Some("rev-existing".into()),
        named_styles: [(
            "HEADING_2".to_string(),
            NamedStyleTemplate {
                text_style: TextStyleTemplate {
                    bold: Some(true),
                    italic: Some(true),
                    font_size_pt: Some(18.0),
                    foreground_color: Some("#336699".into()),
                },
                paragraph_style: None,
            },
        )]
        .into_iter()
        .collect(),
        table: None,
        list: Some(ListStyleTemplate {
            list_type: Some("Bullet".into()),
            preset: "BULLET_DISC_CIRCLE_SQUARE".into(),
        }),
    };
    save_style_template_in(Some(cache_dir.path()), &existing).unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_get_to(
        &client,
        "document-123".into(),
        Some("title".into()),
        false,
        &mut out,
        Some(&documents_url),
        Some(cache_dir.path()),
    )
    .await
    .unwrap();

    let persisted = load_style_template_in(Some(cache_dir.path()), "document-123")
        .unwrap()
        .unwrap();
    assert_eq!(persisted, existing);
}

#[tokio::test]
async fn run_map_prints_human_document_map_for_manual_page_breaks() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(short_document_with_page_break()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::All,
        false,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("Entry Index"));
    assert!(output.contains("1     1       -     1    Heading"));
    assert!(output.contains("TITLE"));
    assert!(output.contains("Project Plan"));
    assert!(output.contains("2     15      2     1    Heading"));
    assert!(output.contains("ExplicitPageBreak"));
    assert!(output.contains("Second Page"));
}

#[tokio::test]
async fn run_compare_reports_semantic_match_while_ignoring_generated_ids() {
    let server = MockServer::start().await;
    let mut source = searchable_document();
    source["documentId"] = serde_json::json!("source-123");
    source["body"]["content"][3]["paragraph"]["paragraphStyle"]["headingId"] =
        serde_json::json!("source-heading");
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["title"] = serde_json::json!("Copied title");
    target["revisionId"] = serde_json::json!("target-revision");
    target["body"]["content"][3]["paragraph"]["paragraphStyle"]["headingId"] =
        serde_json::json!("target-heading");

    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/source-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(source))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/target-456"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(target))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    run_compare_to(
        &client,
        CompareDocumentsCommand {
            source_document_id: "source-123".into(),
            target_document_id: "target-456".into(),
            json: true,
            scope: DocsCompareScope::All,
            fail_on_difference: false,
            max_differences: 20,
            difference_pattern: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["sourceDocumentId"], "source-123");
    assert_eq!(output["targetDocumentId"], "target-456");
    assert_eq!(output["matches"], true);
    assert_eq!(output["scopes"][0]["scope"], "inventory");
    assert_eq!(output["scopes"][0]["sourceInventory"]["breaks"], 1);
    assert_eq!(output["scopes"][1]["scope"], "visual-system");
    assert_eq!(output["scopes"][2]["scope"], "formatting");
    assert_eq!(output["scopes"][3]["scope"], "content");
    assert!(output["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|scope| scope["matches"] == true));
    assert!(output["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .all(|scope| scope["differenceCount"] == 0));
}

#[tokio::test]
async fn run_compare_reports_content_difference() {
    let server = MockServer::start().await;
    let mut source = searchable_document();
    source["documentId"] = serde_json::json!("source-123");
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["elements"][0]["textRun"]["content"] =
        serde_json::json!("Changed title\n");

    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/source-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(source))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/target-456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(target))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let error = run_compare_to(
        &client,
        CompareDocumentsCommand {
            source_document_id: "source-123".into(),
            target_document_id: "target-456".into(),
            json: false,
            scope: DocsCompareScope::All,
            fail_on_difference: true,
            max_differences: 20,
            difference_pattern: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap_err();
    assert_eq!(
        error.to_string(),
        "Google Docs comparison found semantic differences"
    );

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("inventory        yes"));
    assert!(output.contains("visual-system    yes"));
    assert!(output.contains("formatting       yes"));
    assert!(output.contains("content          no"));
    assert!(output.contains("Pattern (1): /entries/*/preview"));
    assert!(output
        .contains("Example /entries/0/preview: source=\"Project Plan\", target=\"Changed title\""));
    assert!(
        output.contains("/entries/0/preview: source=\"Project Plan\", target=\"Changed title\"")
    );
    assert!(output.contains("Overall: different"));
}

#[test]
fn compare_scope_limits_output_and_acceptance_to_selected_scope() {
    let source = searchable_document();
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["elements"][0]["textRun"]["content"] =
        serde_json::json!("Changed title\n");
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::VisualSystem,
        true,
        comparison_preview(20, None),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["matches"], true);
    assert_eq!(output["scopes"].as_array().unwrap().len(), 1);
    assert_eq!(output["scopes"][0]["scope"], "visual-system");
}

#[test]
fn formatting_comparison_ignores_prose_positions_and_empty_run_splits() {
    let source = searchable_document();
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["elements"][0]["textRun"]["content"] =
        serde_json::json!("Different title with a different length\n");
    target["body"]["content"][0]["paragraph"]["elements"] = serde_json::json!([
        {
            "startIndex": 50,
            "endIndex": 94,
            "textRun": { "content": "Different title with a different length" }
        },
        {
            "startIndex": 94,
            "endIndex": 95,
            "textRun": { "content": "\n" }
        }
    ]);
    target["body"]["content"][0]["startIndex"] = serde_json::json!(50);
    target["body"]["content"][0]["endIndex"] = serde_json::json!(95);
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::Formatting,
        true,
        comparison_preview(20, None),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["matches"], true);
    assert_eq!(output["scopes"][0]["scope"], "formatting");
    assert_eq!(output["scopes"][0]["differenceCount"], 0);
}

#[test]
fn formatting_comparison_reports_paragraph_style_changes() {
    let source = searchable_document();
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["paragraphStyle"]["alignment"] =
        serde_json::json!("CENTER");
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    let error = write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::Formatting,
        true,
        comparison_preview(20, None),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "Google Docs comparison found semantic differences"
    );
    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["matches"], false);
    assert_eq!(
        output["scopes"][0]["differences"][0]["path"],
        "/entries/0/paragraphStyle"
    );
    assert_eq!(
        output["scopes"][0]["differencePatterns"][0],
        serde_json::json!({
            "path": "/entries/*/paragraphStyle",
            "count": 1,
            "example": {
                "path": "/entries/0/paragraphStyle",
                "source": "<missing>",
                "target": "{\"alignment\":\"CENTER\"}"
            }
        })
    );
}

#[test]
fn comparison_groups_repeated_array_differences_by_path_pattern() {
    let mut source = searchable_document();
    let second_paragraph = source["body"]["content"][0].clone();
    source["body"]["content"] =
        serde_json::json!([source["body"]["content"][0].clone(), second_paragraph]);
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    for paragraph in target["body"]["content"].as_array_mut().unwrap() {
        paragraph["paragraph"]["paragraphStyle"]["alignment"] = serde_json::json!("CENTER");
    }
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::Formatting,
        false,
        comparison_preview(1, None),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["totalDifferenceCount"], 2);
    assert_eq!(output["totalDisplayedDifferenceCount"], 1);
    assert_eq!(output["totalDifferenceCountHiddenByLimit"], 1);
    assert!(output.get("totalPreviewDifferenceCount").is_none());
    assert!(output.get("totalDifferenceCountOutsidePreview").is_none());
    assert_eq!(output["scopes"][0]["differenceCount"], 2);
    assert_eq!(output["scopes"][0]["displayedDifferenceCount"], 1);
    assert_eq!(output["scopes"][0]["differenceCountHiddenByLimit"], 1);
    assert_eq!(
        output["scopes"][0]["differences"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        output["scopes"][0]["differencePatterns"][0],
        serde_json::json!({
            "path": "/entries/*/paragraphStyle",
            "count": 2,
            "example": {
                "path": "/entries/0/paragraphStyle",
                "source": "<missing>",
                "target": "{\"alignment\":\"CENTER\"}"
            }
        })
    );
    assert_eq!(
        output["scopes"][0]["differencePatterns"][0]["example"]["path"],
        output["scopes"][0]["differences"][0]["path"]
    );
}

#[test]
fn comparison_can_suppress_raw_difference_paths() {
    let source = searchable_document();
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["paragraphStyle"]["alignment"] =
        serde_json::json!("CENTER");
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::Formatting,
        false,
        comparison_preview(0, None),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["totalDifferenceCount"], 1);
    assert_eq!(output["totalDisplayedDifferenceCount"], 0);
    assert_eq!(output["totalDifferenceCountHiddenByLimit"], 1);
    assert_eq!(output["scopes"][0]["displayedDifferenceCount"], 0);
    assert_eq!(output["scopes"][0]["differenceCountHiddenByLimit"], 1);
    assert!(output["scopes"][0].get("differences").is_none());
    assert_eq!(output["scopes"][0]["differencePatterns"][0]["count"], 1);
    assert_eq!(
        output["scopes"][0]["differencePatterns"][0]["example"]["path"],
        "/entries/0/paragraphStyle"
    );
}

#[test]
fn comparison_filters_path_previews_by_reported_pattern() {
    let mut source = searchable_document();
    let second_paragraph = source["body"]["content"][0].clone();
    source["body"]["content"] =
        serde_json::json!([source["body"]["content"][0].clone(), second_paragraph]);
    for paragraph in source["body"]["content"].as_array_mut().unwrap() {
        paragraph["paragraph"]["paragraphStyle"] = serde_json::json!({
            "alignment": "START",
            "direction": "LEFT_TO_RIGHT"
        });
    }
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    for paragraph in target["body"]["content"].as_array_mut().unwrap() {
        paragraph["paragraph"]["paragraphStyle"]["alignment"] = serde_json::json!("CENTER");
        paragraph["paragraph"]["paragraphStyle"]["direction"] = serde_json::json!("RIGHT_TO_LEFT");
    }
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::Formatting,
        false,
        comparison_preview(1, Some("/entries/*/paragraphStyle/direction")),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["differencePreviewPattern"],
        "/entries/*/paragraphStyle/direction"
    );
    assert_eq!(output["totalDifferenceCount"], 4);
    assert_eq!(output["totalDisplayedDifferenceCount"], 1);
    assert_eq!(output["totalDifferenceCountHiddenByLimit"], 1);
    assert_eq!(output["totalPreviewDifferenceCount"], 2);
    assert_eq!(output["totalDifferenceCountOutsidePreview"], 2);
    assert_eq!(output["scopes"][0]["differenceCount"], 4);
    assert_eq!(output["scopes"][0]["displayedDifferenceCount"], 1);
    assert_eq!(output["scopes"][0]["differenceCountHiddenByLimit"], 1);
    assert_eq!(output["scopes"][0]["previewDifferenceCount"], 2);
    assert_eq!(output["scopes"][0]["differenceCountOutsidePreview"], 2);
    assert_eq!(
        output["scopes"][0]["differencePatterns"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        output["scopes"][0]["differences"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        output["scopes"][0]["differences"][0]["path"],
        "/entries/0/paragraphStyle/direction"
    );
}

#[test]
fn filtered_human_comparison_distinguishes_matching_and_other_differences() {
    let mut source = searchable_document();
    let second_paragraph = source["body"]["content"][0].clone();
    source["body"]["content"] =
        serde_json::json!([source["body"]["content"][0].clone(), second_paragraph]);
    for paragraph in source["body"]["content"].as_array_mut().unwrap() {
        paragraph["paragraph"]["paragraphStyle"] = serde_json::json!({
            "alignment": "START",
            "direction": "LEFT_TO_RIGHT"
        });
    }
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    for paragraph in target["body"]["content"].as_array_mut().unwrap() {
        paragraph["paragraph"]["paragraphStyle"]["alignment"] = serde_json::json!("CENTER");
        paragraph["paragraph"]["paragraphStyle"]["direction"] = serde_json::json!("RIGHT_TO_LEFT");
    }
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        false,
        DocsCompareScope::Formatting,
        false,
        comparison_preview(1, Some("/entries/*/paragraphStyle/direction")),
    )
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("... 1 more difference matching the preview filter"));
    assert!(output.contains("2 differences outside the preview filter"));
    assert!(output.contains(
        "Difference totals: 4 overall, 2 matching filter (1 displayed, 1 hidden by limit), 2 outside filter"
    ));
    assert!(!output.contains("... 3 more differences"));
}

#[test]
fn unfiltered_human_comparison_reports_aggregate_difference_totals() {
    let mut source = searchable_document();
    let second_paragraph = source["body"]["content"][0].clone();
    source["body"]["content"] =
        serde_json::json!([source["body"]["content"][0].clone(), second_paragraph]);
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    for paragraph in target["body"]["content"].as_array_mut().unwrap() {
        paragraph["paragraph"]["paragraphStyle"]["alignment"] = serde_json::json!("CENTER");
    }
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        false,
        DocsCompareScope::Formatting,
        false,
        comparison_preview(1, None),
    )
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("Difference totals: 2 overall (1 displayed, 1 hidden by limit)"));
}

#[test]
fn filtered_human_comparison_treats_an_absent_scope_pattern_as_outside() {
    let mut source = searchable_document();
    let second_paragraph = source["body"]["content"][0].clone();
    source["body"]["content"] =
        serde_json::json!([source["body"]["content"][0].clone(), second_paragraph]);
    let mut target = searchable_document();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["paragraphStyle"]["alignment"] =
        serde_json::json!("CENTER");
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        false,
        DocsCompareScope::All,
        false,
        comparison_preview(1, Some("/entries/*/paragraphStyle/alignment")),
    )
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("3 differences outside the preview filter"));
    assert!(!output.contains("... 3 more differences matching the preview filter"));
}

#[test]
fn filtered_json_comparison_treats_an_absent_scope_pattern_as_outside() {
    let mut source = searchable_document();
    let second_paragraph = source["body"]["content"][0].clone();
    source["body"]["content"] =
        serde_json::json!([source["body"]["content"][0].clone(), second_paragraph]);
    let mut target = searchable_document();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["paragraphStyle"]["alignment"] =
        serde_json::json!("CENTER");
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::All,
        false,
        comparison_preview(1, Some("/entries/*/paragraphStyle/alignment")),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let inventory = output["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|scope| scope["scope"] == "inventory")
        .unwrap();
    assert_eq!(
        output["totalDifferenceCount"],
        output["scopes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|scope| scope["differenceCount"].as_u64().unwrap())
            .sum::<u64>()
    );
    assert_eq!(output["totalPreviewDifferenceCount"], 1);
    assert_eq!(
        output["totalDifferenceCountOutsidePreview"],
        output["totalDifferenceCount"].as_u64().unwrap() - 1
    );
    assert_eq!(inventory["previewDifferenceCount"], 0);
    assert_eq!(inventory["displayedDifferenceCount"], 0);
    assert_eq!(inventory["differenceCountHiddenByLimit"], 0);
    assert_eq!(
        inventory["differenceCountOutsidePreview"],
        inventory["differenceCount"]
    );
}

#[test]
fn comparison_rejects_an_unreported_difference_pattern() {
    let source = searchable_document();
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["paragraphStyle"]["alignment"] =
        serde_json::json!("CENTER");
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    let error = write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        false,
        DocsCompareScope::Formatting,
        false,
        comparison_preview(20, Some("/entries/*/paragraphStyle/alignmnt")),
    )
    .unwrap_err();

    assert_eq!(
        error.to_string(),
        "difference pattern `/entries/*/paragraphStyle/alignmnt` was not found in the selected comparison scope. Closest reported pattern: `/entries/*/paragraphStyle`. Run without --difference-pattern to list all reported patterns"
    );
    assert!(out.is_empty());
}

#[test]
fn closest_difference_patterns_rank_typos_before_other_reported_patterns() {
    let mut source = searchable_document();
    source["body"]["content"][0]["paragraph"]["paragraphStyle"] = serde_json::json!({
        "alignment": "START",
        "direction": "LEFT_TO_RIGHT",
        "lineSpacing": 100
    });
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["paragraphStyle"] = serde_json::json!({
        "alignment": "CENTER",
        "direction": "RIGHT_TO_LEFT",
        "lineSpacing": 115
    });
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    let error = write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        false,
        DocsCompareScope::Formatting,
        false,
        comparison_preview(20, Some("/entries/*/paragraphStyle/lineSpcing")),
    )
    .unwrap_err();

    let message = error.to_string();
    assert!(message.contains("Closest reported patterns: `/entries/*/paragraphStyle/lineSpacing`,"));
    assert!(message.contains("`/entries/*/paragraphStyle/alignment`"));
    assert!(message.contains("`/entries/*/paragraphStyle/direction`"));
    assert!(out.is_empty());
}

#[test]
fn formatting_comparison_ignores_font_redundant_with_inherited_named_style() {
    let mut source = searchable_document();
    source["namedStyles"] = serde_json::json!({
        "styles": [
            {
                "namedStyleType": "NORMAL_TEXT",
                "textStyle": {
                    "weightedFontFamily": { "fontFamily": "Bai Jamjuree", "weight": 400 }
                }
            },
            { "namedStyleType": "TITLE", "textStyle": { "fontSize": { "magnitude": 26, "unit": "PT" } } }
        ]
    });
    source["body"]["content"][0]["paragraph"]["elements"][0]["textRun"]["textStyle"] = serde_json::json!({
        "bold": true,
        "weightedFontFamily": { "fontFamily": "Bai Jamjuree", "weight": 400 }
    });
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["body"]["content"][0]["paragraph"]["elements"] = serde_json::json!([
        {
            "startIndex": 1,
            "endIndex": 13,
            "textRun": { "content": "Project Plan", "textStyle": { "bold": true } }
        },
        {
            "startIndex": 13,
            "endIndex": 14,
            "textRun": {
                "content": "\n",
                "textStyle": {
                    "bold": true,
                    "weightedFontFamily": { "fontFamily": "Bai Jamjuree", "weight": 400 }
                }
            }
        }
    ]);
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::Formatting,
        true,
        comparison_preview(20, None),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["matches"], true);
    assert_eq!(output["scopes"][0]["differenceCount"], 0);
}

#[test]
fn visual_system_comparison_ignores_google_materialized_defaults() {
    let mut source = searchable_document();
    source["tabs"] = serde_json::json!([{
        "tabProperties": { "tabId": "tab-1" },
        "documentTab": {
            "documentStyle": { "marginTop": { "magnitude": 72, "unit": "PT" } },
            "namedStyles": { "styles": [{
                "namedStyleType": "HEADING_1",
                "paragraphStyle": {
                    "namedStyleType": "HEADING_1",
                    "keepWithNext": true
                },
                "textStyle": { "fontSize": { "magnitude": 20, "unit": "PT" } }
            }] }
        }
    }]);
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["tabs"][0]["documentTab"]["documentStyle"]["pageNumberStart"] = serde_json::json!(1);
    target["tabs"][0]["documentTab"]["namedStyles"]["styles"][0]["paragraphStyle"]
        ["namedStyleType"] = serde_json::json!("NORMAL_TEXT");
    target["tabs"][0]["documentTab"]["namedStyles"]["styles"][0]["paragraphStyle"]
        ["pageBreakBefore"] = serde_json::json!(false);
    target["tabs"][0]["documentTab"]["namedStyles"]["styles"][0]["textStyle"]["bold"] =
        serde_json::json!(false);
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::VisualSystem,
        true,
        comparison_preview(20, None),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["matches"], true);
    assert_eq!(output["scopes"][0]["differenceCount"], 0);
}

#[test]
fn visual_system_comparison_preserves_non_default_page_number_start() {
    let mut source = searchable_document();
    source["tabs"] = serde_json::json!([{
        "tabProperties": { "tabId": "tab-1" },
        "documentTab": {
            "documentStyle": { "marginTop": { "magnitude": 72, "unit": "PT" } },
            "namedStyles": { "styles": [] }
        }
    }]);
    let mut target = source.clone();
    target["documentId"] = serde_json::json!("target-456");
    target["tabs"][0]["documentTab"]["documentStyle"]["pageNumberStart"] = serde_json::json!(2);
    let source_map = build_document_map(&source);
    let target_map = build_document_map(&target);
    let mut out = Vec::new();

    write_document_comparison(
        &mut out,
        &source_map,
        &target_map,
        true,
        DocsCompareScope::VisualSystem,
        false,
        comparison_preview(20, None),
    )
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["matches"], false);
    assert_eq!(
        output["scopes"][0]["differences"][0]["path"],
        "/documentStyles/0/documentStyle/pageNumberStart"
    );
}

#[test]
fn comparison_differences_report_paths_missing_values_and_a_bounded_preview() {
    let source = serde_json::json!({
        "a/b": [0, 1, 2],
        "removed": true,
        "same": "value",
    });
    let target = serde_json::json!({
        "a/b": [10, 11],
        "added": false,
        "same": "value",
    });
    let mut differences = Vec::new();

    let count = collect_json_differences("", Some(&source), Some(&target), &mut differences, 4);

    assert_eq!(count, 5);
    assert_eq!(differences.len(), 4);
    assert_eq!(differences[0].path, "/a~1b/0");
    assert_eq!(differences[0].source, "0");
    assert_eq!(differences[0].target, "10");
    assert_eq!(differences[2].path, "/a~1b/2");
    assert_eq!(differences[2].target, "<missing>");
    assert_eq!(differences[3].path, "/added");
    assert_eq!(differences[3].source, "<missing>");
}

#[tokio::test]
async fn run_map_filters_page_and_section_breaks() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(document_with_initial_section_and_page_breaks()),
        )
        .expect(2)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut json_out = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Breaks,
        true,
        &mut json_out,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let breaks: serde_json::Value = serde_json::from_slice(&json_out).unwrap();
    assert_eq!(breaks.as_array().unwrap().len(), 3);
    assert_eq!(breaks[0]["kind"], "section-break");
    assert_eq!(breaks[0]["location"]["index"], 0);
    assert_eq!(breaks[0]["sectionType"], "CONTINUOUS");
    assert_eq!(
        breaks[0]["sectionStyle"]["contentDirection"],
        "LEFT_TO_RIGHT"
    );
    assert_eq!(breaks[1]["kind"], "page-break");
    assert_eq!(breaks[1]["location"]["index"], 14);
    assert_eq!(breaks[1]["preview"], "[page break]");
    assert_eq!(breaks[2]["kind"], "section-break");
    assert_eq!(breaks[2]["location"]["index"], 27);
    assert_eq!(breaks[2]["sectionType"], "NEXT_PAGE");
    assert_eq!(breaks[2]["sectionStyle"]["defaultHeaderId"], "header-2");
    assert_eq!(breaks[2]["sectionStyle"]["defaultFooterId"], "footer-2");
    assert_eq!(breaks[2]["preview"], "[section break: next page]");

    let mut human_out = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Breaks,
        false,
        &mut human_out,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let human = String::from_utf8(human_out).unwrap();
    assert!(human.contains("PageBreak"));
    assert!(human.contains("header:header-2,footer:footer-2"));
    assert!(human.contains("SectionBreak"));
    assert!(human.contains("NEXT_PAGE"));
}

#[tokio::test]
async fn run_map_json_emits_structured_locations_for_long_document_shape() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(long_document_with_toc_and_objects()),
        )
        .expect(5)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::All,
        true,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["revisionId"], "rev-long");
    assert_eq!(output["documentStyles"].as_array().unwrap().len(), 1);
    assert_eq!(output["documentStyles"][0]["tabId"], "tab-1");
    assert_eq!(
        output["documentStyles"][0]["documentStyle"]["pageSize"]["width"]["magnitude"],
        612
    );
    assert_eq!(
        output["documentStyles"][0]["documentStyle"]["marginTop"]["magnitude"],
        72
    );
    assert_eq!(output["namedStyles"].as_array().unwrap().len(), 1);
    assert_eq!(output["namedStyles"][0]["tabId"], "tab-1");
    assert_eq!(
        output["namedStyles"][0]["namedStyles"]["styles"][0]["namedStyleType"],
        "NORMAL_TEXT"
    );
    assert_eq!(
        output["namedStyles"][0]["namedStyles"]["styles"][0]["textStyle"]["weightedFontFamily"]
            ["fontFamily"],
        "Bai Jamjuree"
    );
    assert_eq!(output["documentLocations"].as_array().unwrap().len(), 9);
    assert_eq!(output["breaks"].as_array().unwrap().len(), 1);
    assert_eq!(output["entries"][0]["kind"], "table-of-contents");
    assert_eq!(
        output["entries"][0]["preview"],
        "[table of contents: 1 entry]"
    );
    assert_eq!(output["entries"][0]["location"]["index"], 1);
    assert_eq!(output["entries"][0]["textRuns"][0]["startIndex"], 2);
    assert_eq!(output["entries"][0]["textRuns"][0]["endIndex"], 23);
    assert_eq!(
        output["entries"][0]["textRuns"][0]["content"],
        "วิธีใช้งาน\t3\n"
    );
    assert_eq!(
        output["entries"][0]["textRuns"][0]["textStyle"]["link"]["headingId"],
        "h.how-to"
    );
    assert_eq!(output["entries"][0]["paragraphs"][0]["startIndex"], 2);
    assert_eq!(output["entries"][0]["paragraphs"][0]["endIndex"], 23);
    assert_eq!(
        output["entries"][0]["paragraphs"][0]["content"],
        "วิธีใช้งาน\t3\n"
    );
    assert_eq!(
        output["entries"][0]["paragraphs"][0]["paragraphStyle"]["namedStyleType"],
        "NORMAL_TEXT"
    );
    assert_eq!(
        output["entries"][0]["paragraphs"][0]["paragraphStyle"]["indentStart"]["magnitude"],
        18
    );
    assert_eq!(
        output["entries"][1]["location"]["confidence"],
        "table-of-contents"
    );
    assert_eq!(output["entries"][1]["location"]["page"], 3);
    assert_eq!(output["entries"][1]["preview"], "วิธีใช้งาน");
    assert_eq!(output["entries"][1]["headingId"], "h.how-to");
    assert_eq!(
        output["entries"][1]["paragraphStyle"]["alignment"],
        "CENTER"
    );
    assert_eq!(
        output["entries"][1]["paragraphStyle"]["spaceBelow"]["magnitude"],
        10
    );
    assert_eq!(output["entries"][1]["paragraphStyle"]["keepWithNext"], true);
    assert_eq!(output["entries"][1]["textRuns"][0]["startIndex"], 24);
    assert_eq!(output["entries"][1]["textRuns"][0]["endIndex"], 28);
    assert_eq!(
        output["entries"][1]["textRuns"][0]["textStyle"]["weightedFontFamily"]["fontFamily"],
        "Bai Jamjuree"
    );
    assert_eq!(output["entries"][1]["textRuns"][1]["startIndex"], 28);
    assert_eq!(output["entries"][1]["textRuns"][1]["endIndex"], 35);
    assert_eq!(output["entries"][1]["textRuns"][1]["content"], "ใช้งาน\n");
    assert_eq!(
        output["entries"][1]["textRuns"][1]["textStyle"]["underline"],
        true
    );
    assert_eq!(output["entries"][2]["location"]["confidence"], "unknown");
    assert!(output["entries"][2]["location"]["page"].is_null());
    assert_eq!(output["entries"][3]["kind"], "table");
    assert_eq!(output["entries"][3]["preview"], "หัวข้อ | สถานะ");
    assert_eq!(output["entries"][4]["kind"], "inline-image");
    assert_eq!(output["entries"][5]["kind"], "positioned-image");
    assert_eq!(
        output["entries"][6]["location"]["confidence"],
        "explicit-page-break"
    );
    assert_eq!(output["entries"][6]["location"]["page"], 2);
    assert_eq!(output["entries"][6]["location"]["contentLine"], 1);
    assert_eq!(output["entries"][7]["preview"], "[non-body inline image]");
    assert!(output["entries"][7]["location"]["index"].is_null());
    assert_eq!(
        output["entries"][8]["preview"],
        "[non-body positioned image]"
    );
    assert!(output["entries"][8]["location"]["index"].is_null());
    assert_eq!(output["segments"].as_array().unwrap().len(), 3);
    assert_eq!(output["segments"][0]["kind"], "header");
    assert_eq!(output["segments"][0]["segmentId"], "header-123");
    assert_eq!(output["segments"][0]["startIndex"], 0);
    assert_eq!(output["segments"][0]["endIndex"], 17);
    assert_eq!(
        output["segments"][0]["preview"],
        "Customer contact [page number]"
    );
    assert_eq!(output["segments"][0]["autoTextTypes"][0], "PAGE_NUMBER");
    assert_eq!(output["segments"][0]["autoTexts"][0]["startIndex"], 16);
    assert_eq!(output["segments"][0]["autoTexts"][0]["endIndex"], 17);
    assert_eq!(output["segments"][0]["autoTexts"][0]["type"], "PAGE_NUMBER");
    assert_eq!(
        output["segments"][0]["autoTexts"][0]["textStyle"]["fontSize"]["magnitude"],
        10
    );
    assert_eq!(output["segments"][0]["textRuns"][0]["startIndex"], 0);
    assert_eq!(output["segments"][0]["textRuns"][0]["endIndex"], 16);
    assert_eq!(
        output["segments"][0]["textRuns"][0]["content"],
        "Customer contact"
    );
    assert_eq!(
        output["segments"][0]["textRuns"][0]["textStyle"]["weightedFontFamily"]["fontFamily"],
        "Bai Jamjuree"
    );
    assert_eq!(output["segments"][0]["paragraphs"][0]["startIndex"], 0);
    assert_eq!(output["segments"][0]["paragraphs"][0]["endIndex"], 17);
    assert_eq!(
        output["segments"][0]["paragraphs"][0]["content"],
        "Customer contact"
    );
    assert_eq!(
        output["segments"][0]["paragraphs"][0]["paragraphStyle"]["alignment"],
        "CENTER"
    );
    assert_eq!(
        output["segments"][0]["paragraphs"][0]["paragraphStyle"]["lineSpacing"],
        100
    );
    assert_eq!(output["segments"][1]["segmentId"], "legacy-header");
    assert_eq!(output["segments"][1]["preview"], "Legacy header");
    assert_eq!(
        output["segments"][1]["textRuns"][0]["content"],
        "Legacy header\n"
    );
    assert_eq!(
        output["segments"][1]["paragraphs"][0]["paragraphStyle"]["alignment"],
        "END"
    );
    assert_eq!(
        output["segments"][1]["paragraphs"][0]["paragraphStyle"]["spaceBelow"]["magnitude"],
        4
    );
    assert_eq!(output["segments"][2]["kind"], "footer");
    assert_eq!(output["segments"][2]["preview"], "[empty footer]");
    assert_eq!(output["lists"].as_array().unwrap().len(), 1);
    assert_eq!(output["lists"][0]["listId"], "list-abc");
    assert_eq!(output["lists"][0]["itemCount"], 1);
    assert_eq!(output["lists"][0]["nestingLevels"][0], 1);
    assert_eq!(output["lists"][0]["glyphs"][0]["nestingLevel"], 1);
    assert_eq!(output["lists"][0]["glyphs"][0]["glyphSymbol"], "○");
    assert_eq!(
        output["lists"][0]["preview"],
        "เอกสารนี้มีข้อความภาษาไทยสำหรับทดสอบ"
    );

    let mut lists = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Lists,
        true,
        &mut lists,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let lists: serde_json::Value = serde_json::from_slice(&lists).unwrap();
    assert_eq!(lists.as_array().unwrap().len(), 1);
    assert_eq!(lists[0]["startIndex"], 35);
    assert_eq!(lists[0]["endIndex"], 74);

    let mut human_lists = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Lists,
        false,
        &mut human_lists,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let human_lists = String::from_utf8(human_lists).unwrap();
    assert!(human_lists.contains("List"));
    assert!(human_lists.contains("list-abc"));
    assert!(human_lists.contains("○"));

    let mut segments = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Segments,
        true,
        &mut segments,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let segments: serde_json::Value = serde_json::from_slice(&segments).unwrap();
    assert_eq!(segments.as_array().unwrap().len(), 3);
    assert_eq!(segments[0]["segmentId"], "header-123");

    let mut human_segments = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Segments,
        false,
        &mut human_segments,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let human_segments = String::from_utf8(human_segments).unwrap();
    assert!(human_segments.contains("Segment"));
    assert!(human_segments.contains("header-123"));
    assert!(human_segments.contains("PAGE_NUMBER"));
}

#[tokio::test]
async fn run_map_json_emits_each_inline_image_in_a_paragraph() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(document_with_multiple_inline_images()),
        )
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::All,
        true,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["entries"].as_array().unwrap().len(), 2);
    assert_eq!(output["entries"][0]["kind"], "inline-image");
    assert_eq!(output["entries"][0]["location"]["index"], 1);
    assert_eq!(output["entries"][0]["preview"], "[inline image 1]");
    assert_eq!(output["entries"][1]["kind"], "inline-image");
    assert_eq!(output["entries"][1]["location"]["index"], 2);
    assert_eq!(output["entries"][1]["preview"], "[inline image 2]");
    assert_eq!(output["entries"][0]["location"]["contentLine"], 1);
    assert_eq!(output["entries"][1]["location"]["contentLine"], 1);
}

#[tokio::test]
async fn run_search_text_prints_human_matches_from_document_map() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_search_text_to(
        &client,
        "document-123".into(),
        "Plan".into(),
        false,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("Match Page"));
    assert!(output.contains("1     -     1    9     Unknown"));
    assert!(output.contains("Project Plan"));
    assert!(output.contains("2     2     1    49    ExplicitPageBreak"));
    assert!(output.contains("Second Page Plan"));
}

#[tokio::test]
async fn run_search_text_json_emits_document_ranges_and_locations() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_search_text_to(
        &client,
        "document-123".into(),
        "Plan".into(),
        true,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output.as_array().unwrap().len(), 2);
    assert_eq!(output[0]["startIndex"], 9);
    assert_eq!(output[0]["endIndex"], 13);
    assert_eq!(output[0]["location"]["index"], 9);
    assert_eq!(output[0]["location"]["confidence"], "unknown");
    assert_eq!(output[1]["location"]["page"], 2);
    assert_eq!(output[1]["preview"], "Second Page Plan");
}

#[tokio::test]
async fn run_get_content_keeps_index_and_entry_distinct() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(2)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut by_index = Vec::new();
    run_get_content_to(
        &client,
        "document-123".into(),
        ContentSelector::Index(44),
        false,
        &mut by_index,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let by_index = String::from_utf8(by_index).unwrap();
    assert!(by_index.contains("Entry Index"));
    assert!(by_index.contains("3     37"));
    assert!(by_index.contains("Second Page Plan"));

    let mut by_entry = Vec::new();
    run_get_content_to(
        &client,
        "document-123".into(),
        ContentSelector::Entry(2),
        false,
        &mut by_entry,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let by_entry = String::from_utf8(by_entry).unwrap();
    assert!(by_entry.contains("2     14"));
    assert!(by_entry.contains("No matching text here"));
}

#[test]
fn get_content_selector_rejects_mixed_or_partial_selectors() {
    fn assert_selector_error(
        selector: anyhow::Result<ContentSelector>,
        expected_message: &str,
        failure_message: &str,
    ) {
        let error = selector.expect_err(failure_message);
        assert!(
            error.to_string().contains(expected_message),
            "expected {error} to contain {expected_message}"
        );
    }

    assert_selector_error(
        content_selector(Some(44), Some(2), None, None, None),
        "provide exactly one content selector",
        "index and entry selectors must be mutually exclusive",
    );
    assert_selector_error(
        content_selector(None, None, Some(2), None, None),
        "--page and --line must be provided together",
        "page selectors require a matching line",
    );
    assert_selector_error(
        content_selector(None, None, None, Some(1), None),
        "--page and --line must be provided together",
        "line selectors require a matching page",
    );
}

#[test]
fn insert_text_selector_accepts_at_selector() {
    let selector = insert_text_selector("page:2,line:5".into()).unwrap();
    assert_eq!(selector, InsertTextSelector::PageLine { page: 2, line: 5 });

    let selector = insert_text_selector("before-text:\"quarterly plan\"".into()).unwrap();
    assert_eq!(
        selector,
        InsertTextSelector::BeforeText("quarterly plan".into())
    );
}

#[tokio::test]
async fn run_get_content_json_resolves_page_line_and_heading() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(2)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut by_page_line = Vec::new();
    run_get_content_to(
        &client,
        "document-123".into(),
        ContentSelector::PageLine { page: 2, line: 1 },
        true,
        &mut by_page_line,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let by_page_line: serde_json::Value = serde_json::from_slice(&by_page_line).unwrap();
    assert_eq!(by_page_line["entry"], 3);
    assert_eq!(by_page_line["location"]["page"], 2);

    let mut by_heading = Vec::new();
    run_get_content_to(
        &client,
        "document-123".into(),
        ContentSelector::Heading("Second Page Plan".into()),
        true,
        &mut by_heading,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let by_heading: serde_json::Value = serde_json::from_slice(&by_heading).unwrap();
    assert_eq!(by_heading["entry"], 3);
    assert_eq!(by_heading["kind"], "heading");
}

#[tokio::test]
async fn run_get_content_ambiguous_heading_returns_candidates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(ambiguous_heading_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let result = run_get_content_to(
        &client,
        "document-123".into(),
        ContentSelector::Heading("Overview".into()),
        false,
        &mut out,
        Some(&documents_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("ambiguous heading selector"));
    assert!(message.contains("entry 1"));
    assert!(message.contains("entry 2"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_insert_text_dry_run_json_emits_request_without_mutating() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_insert_text_to(
        &client,
        InsertTextCommand {
            document_id: "document-123".into(),
            text: "Hello ".into(),
            selector: InsertTextSelector::PageLine { page: 2, line: 1 },
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["revisionId"], "rev-search");
    assert_eq!(output["location"]["index"], 37);
    assert_eq!(
        output["requestBody"]["requests"][0]["insertText"]["location"]["index"],
        37
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["insertText"]["text"],
        "Hello "
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
    assert_eq!(output["preview"]["before"], "Second Page Plan");
    assert_eq!(output["preview"]["after"], "Hello Second Page Plan");

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
}

#[tokio::test]
async fn run_insert_text_dry_run_json_resolves_exact_heading_and_text_selectors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(5)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let cases = [
        (InsertTextSelector::Index(44), 44, "Second Hello Page Plan"),
        (
            InsertTextSelector::BeforeHeading("Second Page Plan".into()),
            37,
            "Hello Second Page Plan",
        ),
        (
            InsertTextSelector::AfterHeading("Second Page Plan".into()),
            54,
            "Second Page PlanHello ",
        ),
        (
            InsertTextSelector::BeforeText("matching text".into()),
            17,
            "No Hello matching text here",
        ),
        (
            InsertTextSelector::AfterText("matching text".into()),
            30,
            "No matching textHello  here",
        ),
    ];

    for (selector, expected_index, expected_preview) in cases {
        let mut out = Vec::new();
        run_insert_text_to(
            &client,
            InsertTextCommand {
                document_id: "document-123".into(),
                text: "Hello ".into(),
                selector,
                dry_run: true,
                json: true,
                required_revision_id: None,
            },
            &mut out,
            Some(&documents_url),
        )
        .await
        .unwrap();

        let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(output["location"]["index"], expected_index);
        assert_eq!(
            output["requestBody"]["requests"][0]["insertText"]["location"]["index"],
            expected_index
        );
        assert_eq!(output["preview"]["after"], expected_preview);
    }

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 5);
    assert!(requests
        .iter()
        .all(|request| request.method.as_str() == "GET"));
}

#[tokio::test]
async fn run_replace_text_dry_run_json_emits_request_without_mutating() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_replace_text_to(
        &client,
        ReplaceTextCommand {
            document_id: "document-123".into(),
            old_text: "matching text".into(),
            new_text: "updated copy".into(),
            match_number: None,
            all: false,
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["revisionId"], "rev-search");
    assert_eq!(output["ranges"][0]["startIndex"], 17);
    assert_eq!(output["ranges"][0]["endIndex"], 30);
    assert_eq!(
        output["requestBody"]["requests"][0]["deleteContentRange"]["range"]["startIndex"],
        17
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["deleteContentRange"]["range"]["endIndex"],
        30
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["insertText"]["location"]["index"],
        17
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["insertText"]["text"],
        "updated copy"
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
    assert_eq!(
        output["preview"]["changes"][0]["before"],
        "No matching text here"
    );
    assert_eq!(
        output["preview"]["changes"][0]["after"],
        "No updated copy here"
    );

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
}

#[tokio::test]
async fn run_replace_text_all_dry_run_orders_requests_from_document_end() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_replace_text_to(
        &client,
        ReplaceTextCommand {
            document_id: "document-123".into(),
            old_text: "Plan".into(),
            new_text: "Strategy".into(),
            match_number: None,
            all: true,
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["ranges"].as_array().unwrap().len(), 2);
    assert_eq!(output["ranges"][0]["startIndex"], 9);
    assert_eq!(output["ranges"][1]["startIndex"], 49);
    assert_eq!(
        output["requestBody"]["requests"][0]["deleteContentRange"]["range"]["startIndex"],
        49
    );
    assert_eq!(
        output["requestBody"]["requests"][2]["deleteContentRange"]["range"]["startIndex"],
        9
    );
    assert_eq!(output["preview"]["changes"][0]["after"], "Project Strategy");
    assert_eq!(
        output["preview"]["changes"][1]["after"],
        "Second Page Strategy"
    );

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
}

#[tokio::test]
async fn run_insert_text_posts_resolved_batch_update_request() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "insertText": {
                    "location": { "index": 37 },
                    "text": "Hello "
                }
            }
        ],
        "writeControl": {
            "requiredRevisionId": "rev-search"
        }
    });
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_insert_text_to(
        &client,
        InsertTextCommand {
            document_id: "document-123".into(),
            text: "Hello ".into(),
            selector: InsertTextSelector::PageLine { page: 2, line: 1 },
            dry_run: false,
            json: false,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"replies\":[{}]}\n"
    );
}

#[tokio::test]
async fn run_replace_text_posts_selected_match_batch_update_request() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteContentRange": {
                    "range": {
                        "startIndex": 49,
                        "endIndex": 53
                    }
                }
            },
            {
                "insertText": {
                    "location": { "index": 49 },
                    "text": "Strategy"
                }
            }
        ],
        "writeControl": {
            "requiredRevisionId": "rev-search"
        }
    });
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}, {}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_replace_text_to(
        &client,
        ReplaceTextCommand {
            document_id: "document-123".into(),
            old_text: "Plan".into(),
            new_text: "Strategy".into(),
            match_number: Some(2),
            all: false,
            dry_run: false,
            json: false,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"replies\":[{},{}]}\n"
    );
}

#[tokio::test]
async fn run_insert_text_rejects_ambiguous_text_anchor_with_candidates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let result = run_insert_text_to(
        &client,
        InsertTextCommand {
            document_id: "document-123".into(),
            text: "Hello ".into(),
            selector: InsertTextSelector::BeforeText("Plan".into()),
            dry_run: false,
            json: false,
            required_revision_id: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("ambiguous text selector"));
    assert!(message.contains("match 1 index 9"));
    assert!(message.contains("match 2 index 49"));
    assert!(out.is_empty());

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
}

#[tokio::test]
async fn run_replace_text_rejects_ambiguous_match_with_candidates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let result = run_replace_text_to(
        &client,
        ReplaceTextCommand {
            document_id: "document-123".into(),
            old_text: "Plan".into(),
            new_text: "Strategy".into(),
            match_number: None,
            all: false,
            dry_run: false,
            json: false,
            required_revision_id: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("ambiguous text replace match"));
    assert!(message.contains("match 1 index 9"));
    assert!(message.contains("match 2 index 49"));
    assert!(out.is_empty());

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
}

#[tokio::test]
async fn run_map_filters_images_and_tables_with_document_map_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(long_document_with_toc_and_objects()),
        )
        .expect(3)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut images = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Images,
        true,
        &mut images,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let images: serde_json::Value = serde_json::from_slice(&images).unwrap();
    assert_eq!(images.as_array().unwrap().len(), 4);
    assert_eq!(images[0]["kind"], "inline-image");
    assert_eq!(images[0]["imageHandle"], "image-1");
    assert_eq!(images[0]["objectId"], "inline-image-1");
    assert_eq!(images[0]["imageAltText"]["title"], "Process overview");
    assert_eq!(images[0]["paragraphStyle"]["alignment"], "CENTER");
    assert_eq!(
        images[0]["imageAltText"]["description"],
        "Workflow from intake to delivery"
    );
    assert_eq!(
        images[0]["layoutMetadata"]["size"]["width"]["magnitude"],
        144
    );
    assert_eq!(images[0]["layoutMetadata"]["marginLeft"]["magnitude"], 9);
    assert_eq!(
        images[0]["layoutMetadata"]["cropProperties"]["cropLeft"],
        0.1
    );
    assert_eq!(images[1]["kind"], "positioned-image");
    assert_eq!(images[1]["imageHandle"], "image-2");
    assert_eq!(images[1]["objectId"], "positioned-image-1");
    assert_eq!(images[1]["imageAltText"]["title"], "Page decoration");
    assert!(images[1]["location"]["index"].is_number());
    assert_eq!(
        images[1]["layoutMetadata"]["positioning"]["layout"],
        "WRAP_TEXT"
    );
    assert_eq!(
        images[1]["layoutMetadata"]["positioning"]["leftOffset"]["magnitude"],
        12
    );
    assert_eq!(
        images[1]["layoutMetadata"]["size"]["height"]["magnitude"],
        72
    );
    assert_eq!(images[2]["kind"], "inline-image");
    assert_eq!(images[2]["objectId"], "header-inline-image");
    assert!(images[2]["location"]["index"].is_null());
    assert_eq!(images[2]["preview"], "[non-body inline image]");
    assert_eq!(
        images[2]["imageAltText"]["description"],
        "Customer header logo"
    );
    assert_eq!(
        images[2]["layoutMetadata"]["size"]["height"]["magnitude"],
        24
    );
    assert_eq!(images[3]["kind"], "positioned-image");
    assert_eq!(images[3]["objectId"], "footer-positioned-image");
    assert!(images[3]["location"]["index"].is_null());
    assert_eq!(images[3]["preview"], "[non-body positioned image]");
    assert_eq!(images[3]["imageAltText"]["title"], "Footer decoration");
    assert_eq!(
        images[3]["layoutMetadata"]["positioning"]["layout"],
        "BEHIND_TEXT"
    );

    let mut human_images = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Images,
        false,
        &mut human_images,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let human_images = String::from_utf8(human_images).unwrap();
    assert!(human_images.contains("Handle"));
    assert!(human_images.contains("image-1"));
    assert!(human_images.contains("inline-image-1"));
    assert!(human_images.contains("image-2"));
    assert!(human_images.contains("positioned-image-1"));
    assert!(human_images.contains("Image alt text"));
    assert!(human_images.contains("Process overview"));
    assert!(human_images.contains("Page decoration"));

    let mut tables = Vec::new();
    run_map_to(
        &client,
        "document-123".into(),
        DocsMapType::Tables,
        true,
        &mut tables,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let tables: serde_json::Value = serde_json::from_slice(&tables).unwrap();
    assert_eq!(tables.as_array().unwrap().len(), 1);
    assert_eq!(tables[0]["kind"], "table");
    assert_eq!(tables[0]["tableHandle"], "table-1");
    assert_eq!(tables[0]["rows"], 1);
    assert_eq!(tables[0]["columns"], 2);
    assert_eq!(tables[0]["preview"], "หัวข้อ | สถานะ");
    assert_eq!(
        tables[0]["layoutMetadata"]["tableColumnProperties"][0]["width"]["magnitude"],
        144
    );
    assert_eq!(
        tables[0]["layoutMetadata"]["tableColumnProperties"][1]["widthType"],
        "FIXED_WIDTH"
    );
    assert_eq!(tables[0]["layoutMetadata"]["pinnedHeaderRowsCount"], 1);
    assert_eq!(
        tables[0]["layoutMetadata"]["tableCellStyles"][0][0]["contentAlignment"],
        "MIDDLE"
    );
    assert_eq!(
        tables[0]["layoutMetadata"]["tableCellStyles"][0][0]["backgroundColor"]["color"]
            ["rgbColor"]["blue"],
        0.9
    );
    assert_eq!(
        tables[0]["layoutMetadata"]["tableCellStyles"][0][0]["borderBottom"]["width"]["magnitude"],
        1
    );
    assert_eq!(
        tables[0]["layoutMetadata"]["tableCellStyles"][0][1],
        serde_json::json!({})
    );
    assert_eq!(tables[0]["tableCellTextRuns"][0][0][0]["startIndex"], 77);
    assert_eq!(tables[0]["tableCellTextRuns"][0][0][0]["endIndex"], 84);
    assert_eq!(
        tables[0]["tableCellTextRuns"][0][0][0]["textStyle"]["bold"],
        true
    );
    assert_eq!(
        tables[0]["tableCellTextRuns"][0][1][0]["content"],
        "สถานะ\n"
    );
    assert_eq!(
        tables[0]["tableCellTextRuns"][0][1][0]["textStyle"],
        serde_json::json!({"italic": true})
    );
    assert_eq!(tables[0]["tableCellParagraphs"][0][0][0]["startIndex"], 77);
    assert_eq!(tables[0]["tableCellParagraphs"][0][0][0]["endIndex"], 84);
    assert_eq!(
        tables[0]["tableCellParagraphs"][0][0][0]["content"],
        "หัวข้อ\n"
    );
    assert_eq!(
        tables[0]["tableCellParagraphs"][0][0][0]["paragraphStyle"]["alignment"],
        "CENTER"
    );
}

#[tokio::test]
async fn run_insert_image_and_table_dry_run_emit_native_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(2)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut image = Vec::new();
    run_insert_image_to(
        &client,
        InsertImageCommand {
            document_id: "document-123".into(),
            image_uri: "https://example.test/image.png".into(),
            selector: Some(InsertTextSelector::PageLine { page: 2, line: 1 }),
            segment_id: None,
            width: Some(468.0),
            height: Some(240.0),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut image,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let image: serde_json::Value = serde_json::from_slice(&image).unwrap();
    assert_eq!(image["location"]["index"], 37);
    assert_eq!(
        image["requestBody"]["requests"][0]["insertInlineImage"]["location"]["index"],
        37
    );
    assert_eq!(
        image["requestBody"]["requests"][0]["insertInlineImage"]["uri"],
        "https://example.test/image.png"
    );
    assert_eq!(
        image["requestBody"]["requests"][0]["insertInlineImage"]["objectSize"],
        serde_json::json!({
            "width": { "magnitude": 468.0, "unit": "PT" },
            "height": { "magnitude": 240.0, "unit": "PT" }
        })
    );
    assert_eq!(
        image["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );

    let mut table = Vec::new();
    run_insert_table_to(
        &client,
        InsertTableCommand {
            document_id: "document-123".into(),
            data: None,
            rows: Some(2),
            columns: Some(3),
            selector: InsertTextSelector::Index(44),
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        &mut table,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();
    let table: serde_json::Value = serde_json::from_slice(&table).unwrap();
    assert_eq!(
        table["requestBody"]["requests"][0]["insertTable"]["location"]["index"],
        44
    );
    assert_eq!(
        table["requestBody"]["requests"][0]["insertTable"]["rows"],
        2
    );
    assert_eq!(
        table["requestBody"]["requests"][0]["insertTable"]["columns"],
        3
    );
}

#[tokio::test]
async fn run_insert_page_break_dry_run_emits_native_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_insert_page_break_to(
        &client,
        InsertPageBreakCommand {
            document_id: "document-123".into(),
            selector: InsertTextSelector::Index(44),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["location"]["index"], 44);
    assert_eq!(
        output["requestBody"]["requests"][0]["insertPageBreak"]["location"]["index"],
        44
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
}

#[tokio::test]
async fn run_insert_section_break_dry_run_emits_native_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_insert_section_break_to(
        &client,
        InsertSectionBreakCommand {
            document_id: "document-123".into(),
            section_type: DocsSectionBreakType::Continuous,
            selector: InsertTextSelector::Index(44),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["location"]["index"], 44);
    assert_eq!(
        output["requestBody"]["requests"][0]["insertSectionBreak"]["location"]["index"],
        44
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["insertSectionBreak"]["sectionType"],
        "CONTINUOUS"
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
}

#[tokio::test]
async fn run_create_header_dry_run_emits_native_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_create_header_to(
        &client,
        CreateHeaderCommand {
            document_id: "document-123".into(),
            text: None,
            section_break_index: Some(16),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["createHeader"]["type"],
        "DEFAULT"
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["createHeader"]["sectionBreakLocation"]["index"],
        16
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
}

#[tokio::test]
async fn run_create_footer_dry_run_emits_native_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_create_footer_to(
        &client,
        CreateFooterCommand {
            document_id: "document-123".into(),
            text: None,
            section_break_index: None,
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["createFooter"]["type"],
        "DEFAULT"
    );
    assert!(output["requestBody"]["requests"][0]["createFooter"]["sectionBreakLocation"].is_null());
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
}

#[tokio::test]
async fn run_create_header_populates_returned_segment_in_guarded_follow_up() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(body_json(serde_json::json!({
            "requests": [{ "createHeader": { "type": "DEFAULT" } }],
            "writeControl": { "requiredRevisionId": "rev-search" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{ "createHeader": { "headerId": "header-123" } }],
            "writeControl": { "requiredRevisionId": "rev-after-header" }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(body_json(serde_json::json!({
            "requests": [{
                "insertText": {
                    "endOfSegmentLocation": { "segmentId": "header-123" },
                    "text": "Confidential"
                }
            }],
            "writeControl": { "requiredRevisionId": "rev-after-header" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}],
            "writeControl": { "requiredRevisionId": "rev-after-text" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();
    run_create_header_to(
        &client,
        CreateHeaderCommand {
            document_id: "document-123".into(),
            text: Some("Confidential".into()),
            section_break_index: None,
            dry_run: false,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["replies"][0]["createHeader"]["headerId"],
        "header-123"
    );
    assert_eq!(output["replies"].as_array().unwrap().len(), 2);
    assert_eq!(
        output["writeControl"]["requiredRevisionId"],
        "rev-after-text"
    );
}

#[tokio::test]
async fn run_create_footer_populates_returned_segment_in_guarded_follow_up() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(body_json(serde_json::json!({
            "requests": [{ "createFooter": { "type": "DEFAULT" } }],
            "writeControl": { "requiredRevisionId": "rev-search" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{ "createFooter": { "footerId": "footer-123" } }],
            "writeControl": { "requiredRevisionId": "rev-after-footer" }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(body_json(serde_json::json!({
            "requests": [{
                "insertText": {
                    "endOfSegmentLocation": { "segmentId": "footer-123" },
                    "text": "Customer proposal"
                }
            }],
            "writeControl": { "requiredRevisionId": "rev-after-footer" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();
    run_create_footer_to(
        &client,
        CreateFooterCommand {
            document_id: "document-123".into(),
            text: Some("Customer proposal".into()),
            section_break_index: None,
            dry_run: false,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["replies"][0]["createFooter"]["footerId"],
        "footer-123"
    );
    assert_eq!(output["replies"].as_array().unwrap().len(), 2);
}

#[tokio::test]
async fn run_create_footnote_dry_run_emits_native_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_create_footnote_to(
        &client,
        CreateFootnoteCommand {
            document_id: "document-123".into(),
            selector: InsertTextSelector::Index(44),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["location"]["index"], 44);
    assert_eq!(
        output["requestBody"]["requests"][0]["createFootnote"]["location"]["index"],
        44
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
}

#[tokio::test]
async fn run_insert_table_dry_run_plans_structure_before_follow_up_population() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("wide-table.csv");
    std::fs::write(&data_path, "A1,B1,C1,D1\nA2,B2,C2,D2\n").unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_insert_table_to(
        &client,
        InsertTableCommand {
            document_id: "document-123".into(),
            data: Some(data_path.to_string_lossy().into_owned()),
            rows: None,
            columns: None,
            selector: InsertTextSelector::Index(44),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
            no_auto_style: false,
        },
        &mut out,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["insertTable"]["rows"],
        2
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["insertTable"]["columns"],
        4
    );
    assert_eq!(
        output["requestBody"]["requests"].as_array().unwrap().len(),
        1
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
    assert!(output["preview"]["summary"]
        .as_str()
        .unwrap()
        .contains("A1 | B1 | C1 / A2 | B2 | C2"));
}

#[test]
fn inserted_table_handle_uses_the_post_insert_table_index() {
    let document_map = build_document_map(&editable_table_document());

    assert_eq!(inserted_table_handle(&document_map, 0).unwrap(), "table-1");
    assert!(inserted_table_handle(&document_map, 1)
        .unwrap_err()
        .to_string()
        .contains("inserted table was not found at Google Docs index 2"));
}

#[tokio::test]
async fn run_insert_table_dry_run_accepts_tsv_data() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("table.tsv");
    std::fs::write(&data_path, "Left\tRight\nBottom left\tBottom right\n").unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_insert_table_to(
        &client,
        InsertTableCommand {
            document_id: "document-123".into(),
            data: Some(data_path.to_string_lossy().into_owned()),
            rows: None,
            columns: None,
            selector: InsertTextSelector::Index(44),
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        &mut out,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["insertTable"]["columns"],
        2
    );
    assert!(output["preview"]["summary"]
        .as_str()
        .unwrap()
        .contains("Bottom right"));
}

#[tokio::test]
async fn run_insert_image_dry_run_human_shows_placeholder_in_context() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_insert_image_to(
        &client,
        InsertImageCommand {
            document_id: "document-123".into(),
            image_uri: "https://example.test/image.png".into(),
            selector: Some(InsertTextSelector::PageLine { page: 2, line: 1 }),
            segment_id: None,
            width: None,
            height: None,
            dry_run: true,
            json: false,
            required_revision_id: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("image insert: Insert inline image at index 37"));
    assert!(output.contains("Before: Second Page Plan"));
    assert!(output.contains("After: [inline image]Second Page Plan"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
}

#[tokio::test]
async fn run_insert_image_dry_run_targets_header_or_footer_segment_end() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_insert_image_to(
        &client,
        InsertImageCommand {
            document_id: "document-123".into(),
            image_uri: "https://example.test/logo.png".into(),
            selector: None,
            segment_id: Some("header-123".into()),
            width: Some(72.0),
            height: Some(24.0),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["location"], serde_json::Value::Null);
    assert_eq!(
        output["requestBody"]["requests"][0]["insertInlineImage"]["endOfSegmentLocation"]
            ["segmentId"],
        "header-123"
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
}

#[tokio::test]
async fn run_edit_table_dry_run_replaces_cells_from_document_end() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(editable_table_document()))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("table.csv");
    std::fs::write(&data_path, "New A,New B\nNew C,New D\n").unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_edit_table_to(
        &client,
        EditTableCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            data: data_path.to_string_lossy().into_owned(),
            resize: false,
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-table".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["location"]["index"], 1);
    assert_eq!(
        output["requestBody"]["requests"][0]["deleteContentRange"]["range"]["startIndex"],
        31
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["insertText"]["location"]["index"],
        31
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["insertText"]["text"],
        "New D"
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-table"
    );
}

#[tokio::test]
async fn run_edit_table_rejects_dimension_changes_without_supported_resize() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(editable_table_document()))
        .expect(2)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("mismatch.csv");
    std::fs::write(&data_path, "Only,Two\n").unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mismatch = run_edit_table_to(
        &client,
        EditTableCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            data: data_path.to_string_lossy().into_owned(),
            resize: false,
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
        &mut Vec::new(),
        Some(&documents_url),
    )
    .await;

    let message = format!("{:#}", mismatch.unwrap_err());
    assert!(message.contains("table edit data dimensions are 1x2"));
    assert!(message.contains("table-1 is 2x2"));
    assert!(message.contains("pass --resize"));

    let resize = run_edit_table_to(
        &client,
        EditTableCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            data: data_path.to_string_lossy().into_owned(),
            resize: true,
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
        &mut Vec::new(),
        Some(&documents_url),
    )
    .await;

    let message = format!("{:#}", resize.unwrap_err());
    assert!(message.contains("table edit --resize is not supported yet"));
}

#[tokio::test]
async fn run_style_table_row_dry_run_targets_the_selected_native_row() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(editable_table_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_style_table_row_to(
        &client,
        StyleTableRowCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            row: 2,
            column: Some(1),
            background_color: None,
            content_alignment: Some(crate::cli::DocsTableCellAlignment::Middle),
            border_color: Some("#FFFFFF".into()),
            border_width: Some(1.0),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-table".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let update = &output["requestBody"]["requests"][0]["updateTableCellStyle"];
    assert_eq!(update["tableRange"]["tableCellLocation"]["rowIndex"], 1);
    assert_eq!(update["tableRange"]["tableCellLocation"]["columnIndex"], 0);
    assert_eq!(update["tableRange"]["columnSpan"], 1);
    assert_eq!(update["tableCellStyle"]["contentAlignment"], "MIDDLE");
    assert_eq!(
        update["tableCellStyle"]["borderTop"]["width"]["magnitude"],
        1.0
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-table"
    );
}

#[tokio::test]
async fn run_set_table_column_widths_dry_run_targets_each_native_column() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(editable_table_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_set_table_column_widths_to(
        &client,
        SetTableColumnWidthsCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            widths: vec![104.25, 363.75],
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-table".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    let requests = output["requestBody"]["requests"].as_array().unwrap();
    assert_eq!(requests.len(), 2);
    assert_eq!(
        requests[0]["updateTableColumnProperties"]["columnIndices"],
        serde_json::json!([0])
    );
    assert_eq!(
        requests[1]["updateTableColumnProperties"]["tableColumnProperties"]["width"],
        serde_json::json!({ "magnitude": 363.75, "unit": "PT" })
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-table"
    );
}

#[tokio::test]
async fn run_pin_table_header_rows_dry_run_targets_the_native_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(editable_table_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_pin_table_header_rows_to(
        &client,
        PinTableHeaderRowsCommand {
            document_id: "document-123".into(),
            table_id: "table-1".into(),
            rows: 1,
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-table".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["pinTableHeaderRows"]["pinnedHeaderRowsCount"],
        1
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-table"
    );
}

#[tokio::test]
async fn run_apply_styles_and_list_dry_run_emit_native_requests() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(2)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut styles = Vec::new();
    run_apply_styles_to(
        &client,
        ApplyStylesCommand {
            document_id: "document-123".into(),
            segment_id: None,
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: true,
            italic: true,
            underline: true,
            font_size: Some(14.0),
            font_family: Some("Bai Jamjuree".into()),
            foreground_color: Some("#336699".into()),
            link_heading_id: Some("h.target-heading".into()),
            alignment: Some(crate::cli::DocsParagraphAlignment::Justified),
            direction: Some(crate::cli::DocsParagraphDirection::LeftToRight),
            space_above: Some(4.0),
            space_below: Some(10.0),
            line_spacing: Some(115.0),
            spacing_mode: Some(crate::cli::DocsParagraphSpacingMode::NeverCollapse),
            indent_start: Some(36.0),
            indent_end: Some(12.0),
            indent_first_line: Some(18.0),
            keep_with_next: true,
            keep_lines_together: true,
            avoid_widow_and_orphan: true,
            page_break_before: true,
            heading: Some("HEADING_2".into()),
            style_json: None,
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
            no_auto_style: false,
        },
        &mut styles,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();
    let styles: serde_json::Value = serde_json::from_slice(&styles).unwrap();
    assert_eq!(styles["range"]["startIndex"], 17);
    assert_eq!(
        styles["requestBody"]["requests"][1]["updateTextStyle"]["fields"],
        "bold,italic,underline,fontSize,weightedFontFamily,foregroundColor,link"
    );
    assert_eq!(
        styles["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["link"]["headingId"],
        "h.target-heading"
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
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["namedStyleType"],
        "HEADING_2"
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]["alignment"],
        "JUSTIFIED"
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]["direction"],
        "LEFT_TO_RIGHT"
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["fields"],
        "namedStyleType,alignment,direction,spaceAbove,spaceBelow,lineSpacing,spacingMode,indentStart,indentEnd,indentFirstLine,keepWithNext,keepLinesTogether,avoidWidowAndOrphan,pageBreakBefore"
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["lineSpacing"],
        115.0
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["spacingMode"],
        "NEVER_COLLAPSE"
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["indentStart"],
        serde_json::json!({ "magnitude": 36.0, "unit": "PT" })
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]["indentEnd"],
        serde_json::json!({ "magnitude": 12.0, "unit": "PT" })
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["indentFirstLine"],
        serde_json::json!({ "magnitude": 18.0, "unit": "PT" })
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
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["pageBreakBefore"],
        true
    );
    assert_eq!(
        styles["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );

    let mut list = Vec::new();
    run_apply_list_to(
        &client,
        ApplyListCommand {
            document_id: "document-123".into(),
            selector: RangeSelector::Entry(2),
            list_type: Some(crate::cli::DocsListType::Checkbox),
            preset: None,
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        &mut list,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();
    let list: serde_json::Value = serde_json::from_slice(&list).unwrap();
    assert_eq!(
        list["requestBody"]["requests"][0]["createParagraphBullets"]["bulletPreset"],
        "BULLET_CHECKBOX"
    );
    assert_eq!(
        list["requestBody"]["requests"][0]["createParagraphBullets"]["range"]["startIndex"],
        14
    );
}

#[tokio::test]
async fn run_create_named_range_dry_run_emits_native_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_create_named_range_to(
        &client,
        CreateNamedRangeCommand {
            document_id: "document-123".into(),
            name: "my-range".into(),
            selector: RangeSelector::Entry(2),
            dry_run: true,
            json: true,
            required_revision_id: Some("rev-search".into()),
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["createNamedRange"]["name"],
        "my-range"
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["createNamedRange"]["range"]["startIndex"],
        14
    );
    assert_eq!(
        output["requestBody"]["writeControl"]["requiredRevisionId"],
        "rev-search"
    );
}

#[tokio::test]
async fn run_delete_named_range_dry_run_emits_native_request_by_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_delete_named_range_to(
        &client,
        DeleteNamedRangeCommand {
            document_id: "document-123".into(),
            named_range_id: Some("named-range-1".into()),
            name: None,
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["deleteNamedRange"]["namedRangeId"],
        "named-range-1"
    );
}

#[tokio::test]
async fn run_delete_named_range_rejects_conflicting_selectors() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    let error = run_delete_named_range_to(
        &client,
        DeleteNamedRangeCommand {
            document_id: "document-123".into(),
            named_range_id: Some("named-range-1".into()),
            name: Some("my-range".into()),
            dry_run: true,
            json: true,
            required_revision_id: None,
        },
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap_err();

    assert!(error
        .to_string()
        .contains("exactly one of --named-range-id or --name"));
}

#[tokio::test]
async fn run_apply_list_dry_run_maps_cli_types_and_preserves_raw_preset() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(5)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    for (list_type, expected_preset) in [
        (DocsListType::Bullet, "BULLET_DISC_CIRCLE_SQUARE"),
        (DocsListType::Numbered, "NUMBERED_DECIMAL_ALPHA_ROMAN"),
        (DocsListType::Dash, "BULLET_DIAMONDX_ARROW3D_SQUARE"),
        (DocsListType::Checkbox, "BULLET_CHECKBOX"),
    ] {
        let mut out = Vec::new();
        run_apply_list_to(
            &client,
            dry_run_apply_list_command(
                RangeSelector::IndexRange {
                    start_index: 4,
                    end_index: 12,
                },
                list_type,
            ),
            &mut out,
            Some(&documents_url),
            None,
        )
        .await
        .unwrap();

        let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
        let bullets = &output["requestBody"]["requests"][0]["createParagraphBullets"];
        assert_eq!(bullets["bulletPreset"], expected_preset);
        assert_eq!(bullets["range"]["startIndex"], 4);
        assert_eq!(bullets["range"]["endIndex"], 12);
        assert_eq!(output["range"]["startIndex"], 4);
        assert_eq!(output["range"]["endIndex"], 12);
        assert_eq!(output["revisionId"], "rev-search");
    }

    let mut raw = Vec::new();
    run_apply_list_to(
        &client,
        ApplyListCommand {
            list_type: None,
            preset: Some("BULLET_STAR_CIRCLE_SQUARE".into()),
            required_revision_id: Some("rev-required".into()),
            ..dry_run_apply_list_command(
                RangeSelector::IndexRange {
                    start_index: 6,
                    end_index: 18,
                },
                DocsListType::Bullet,
            )
        },
        &mut raw,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let request_body = &output["requestBody"];
    assert_eq!(
        request_body["requests"][0]["createParagraphBullets"]["bulletPreset"],
        "BULLET_STAR_CIRCLE_SQUARE"
    );
    assert_eq!(
        request_body["writeControl"]["requiredRevisionId"],
        "rev-required"
    );
}

#[tokio::test]
async fn run_apply_list_targets_whole_blocks_and_rejects_ambiguous_text_ranges() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(2)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut page_line = Vec::new();
    run_apply_list_to(
        &client,
        dry_run_apply_list_command(
            RangeSelector::PageLine { page: 2, line: 1 },
            DocsListType::Bullet,
        ),
        &mut page_line,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&page_line).unwrap();
    assert_eq!(output["range"]["startIndex"], 37);
    assert_eq!(output["range"]["endIndex"], 54);
    assert_eq!(output["range"]["preview"], "Second Page Plan");
    assert_eq!(
        output["requestBody"]["requests"][0]["createParagraphBullets"]["range"]["startIndex"],
        37
    );

    let result = run_apply_list_to(
        &client,
        dry_run_apply_list_command(
            RangeSelector::Text {
                text: "Plan".into(),
                match_number: None,
            },
            DocsListType::Numbered,
        ),
        &mut Vec::new(),
        Some(&documents_url),
        None,
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("ambiguous text replace match"));
    assert!(message.contains("index 9"));
    assert!(message.contains("index 49"));
}

#[tokio::test]
async fn run_apply_list_posts_mutation_request_body() {
    let server = MockServer::start().await;
    let expected_request = serde_json::json!({
        "requests": [
            {
                "createParagraphBullets": {
                    "range": {
                        "startIndex": 1,
                        "endIndex": 14
                    },
                    "bulletPreset": "BULLET_DISC_CIRCLE_SQUARE"
                }
            }
        ],
        "writeControl": {
            "requiredRevisionId": "rev-search"
        }
    });
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(&expected_request))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_apply_list_to(
        &client,
        ApplyListCommand {
            dry_run: false,
            json: false,
            required_revision_id: Some("rev-search".into()),
            ..dry_run_apply_list_command(RangeSelector::Entry(1), DocsListType::Bullet)
        },
        &mut out,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"replies\":[{}]}\n"
    );
}

#[tokio::test]
async fn run_apply_styles_dry_run_preserves_raw_style_payload() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_apply_styles_to(
        &client,
        ApplyStylesCommand {
            document_id: "document-123".into(),
            segment_id: None,
            selector: RangeSelector::IndexRange {
                start_index: 17,
                end_index: 30,
            },
            bold: false,
            italic: false,
            underline: false,
            font_size: None,
            font_family: None,
            foreground_color: None,
            link_heading_id: None,
            alignment: None,
            direction: None,
            space_above: None,
            space_below: None,
            line_spacing: None,
            spacing_mode: None,
            indent_start: None,
            indent_end: None,
            indent_first_line: None,
            keep_with_next: false,
            keep_lines_together: false,
            avoid_widow_and_orphan: false,
            page_break_before: false,
            heading: None,
            style_json: Some(
                serde_json::json!({
                    "textStyle": {
                        "underline": true,
                        "weightedFontFamily": {
                            "fontFamily": "Roboto",
                            "weight": 700
                        }
                    },
                    "paragraphStyle": {
                        "alignment": "CENTER"
                    }
                })
                .to_string(),
            ),
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        &mut out,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["underline"],
        true
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["weightedFontFamily"]
            ["fontFamily"],
        "Roboto"
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["updateTextStyle"]["fields"],
        "underline,weightedFontFamily"
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]["alignment"],
        "CENTER"
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["updateParagraphStyle"]["fields"],
        "alignment"
    );
}

#[tokio::test]
async fn run_apply_styles_mutates_with_raw_and_shorthand_payload() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updateParagraphStyle": {
                        "range": {
                            "startIndex": 17,
                            "endIndex": 30
                        },
                        "paragraphStyle": {
                            "namedStyleType": "HEADING_1"
                        },
                        "fields": "namedStyleType"
                    }
                }
            ],
            "writeControl": {
                "requiredRevisionId": "rev-search"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}],
            "writeControl": {
                "requiredRevisionId": "rev-after-paragraph"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updateTextStyle": {
                        "range": {
                            "startIndex": 17,
                            "endIndex": 30
                        },
                        "textStyle": {
                            "strikethrough": true,
                            "bold": true
                        },
                        "fields": "strikethrough,bold"
                    }
                }
            ],
            "writeControl": {
                "requiredRevisionId": "rev-after-paragraph"
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_apply_styles_to(
        &client,
        ApplyStylesCommand {
            document_id: "document-123".into(),
            segment_id: None,
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: true,
            italic: false,
            underline: false,
            font_size: None,
            font_family: None,
            foreground_color: None,
            link_heading_id: None,
            alignment: None,
            direction: None,
            space_above: None,
            space_below: None,
            line_spacing: None,
            spacing_mode: None,
            indent_start: None,
            indent_end: None,
            indent_first_line: None,
            keep_with_next: false,
            keep_lines_together: false,
            avoid_widow_and_orphan: false,
            page_break_before: false,
            heading: Some("HEADING_1".into()),
            style_json: Some(r#"{"textStyle":{"strikethrough":true}}"#.into()),
            dry_run: false,
            json: false,
            required_revision_id: Some("rev-search".into()),
            no_auto_style: false,
        },
        &mut out,
        Some(&documents_url),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"replies\":[{}]}\n"
    );
}

#[tokio::test]
async fn run_apply_styles_uses_cached_heading_style_when_flags_are_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let cache_dir = tempfile::tempdir().unwrap();
    save_style_template_in(
        Some(cache_dir.path()),
        &StyleTemplate {
            document_id: "document-123".into(),
            source_revision_id: Some("rev-style".into()),
            named_styles: [(
                "HEADING_2".to_string(),
                NamedStyleTemplate {
                    text_style: TextStyleTemplate {
                        bold: Some(true),
                        italic: Some(true),
                        font_size_pt: Some(14.0),
                        foreground_color: Some("#336699".into()),
                    },
                    paragraph_style: Some(serde_json::json!({
                        "borderBottom": {
                            "dashStyle": "SOLID",
                            "padding": { "magnitude": 4.0, "unit": "PT" },
                            "width": { "magnitude": 1.5, "unit": "PT" }
                        },
                        "spaceAbove": { "magnitude": 14.0, "unit": "PT" },
                        "spaceBelow": { "magnitude": 7.0, "unit": "PT" },
                        "spacingMode": "NEVER_COLLAPSE"
                    })),
                },
            )]
            .into_iter()
            .collect(),
            table: None,
            list: None,
        },
    )
    .unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_apply_styles_to(
        &client,
        ApplyStylesCommand {
            document_id: "document-123".into(),
            segment_id: None,
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: false,
            italic: false,
            underline: false,
            font_size: None,
            font_family: None,
            foreground_color: None,
            link_heading_id: None,
            alignment: None,
            direction: None,
            space_above: None,
            space_below: None,
            line_spacing: None,
            spacing_mode: None,
            indent_start: None,
            indent_end: None,
            indent_first_line: None,
            keep_with_next: false,
            keep_lines_together: false,
            avoid_widow_and_orphan: false,
            page_break_before: false,
            heading: Some("HEADING_2".into()),
            style_json: None,
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        &mut out,
        Some(&documents_url),
        Some(cache_dir.path()),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["bold"],
        true
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["italic"],
        true
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["updateTextStyle"]["textStyle"]["fontSize"]
            ["magnitude"],
        14.0
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["updateTextStyle"]["fields"],
        "bold,italic,fontSize,foregroundColor"
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["namedStyleType"],
        "HEADING_2"
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["spacingMode"],
        "NEVER_COLLAPSE"
    );
    assert_eq!(
        output["requestBody"]["requests"][0]["updateParagraphStyle"]["fields"],
        "namedStyleType,borderBottom,spaceAbove,spaceBelow,spacingMode"
    );
}

#[tokio::test]
async fn run_apply_styles_posts_heading_and_text_updates_as_separate_batch_updates() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}],
            "writeControl": {
                "requiredRevisionId": "rev-after-paragraph"
            }
        })))
        .expect(2)
        .mount(&server)
        .await;

    let cache_dir = tempfile::tempdir().unwrap();
    save_style_template_in(
        Some(cache_dir.path()),
        &StyleTemplate {
            document_id: "document-123".into(),
            source_revision_id: Some("rev-style".into()),
            named_styles: [(
                "HEADING_1".to_string(),
                NamedStyleTemplate {
                    text_style: TextStyleTemplate {
                        bold: Some(true),
                        italic: None,
                        font_size_pt: Some(15.0),
                        foreground_color: Some("#00595B".into()),
                    },
                    paragraph_style: Some(serde_json::json!({
                        "borderBottom": {
                            "dashStyle": "SOLID",
                            "padding": { "magnitude": 4.0, "unit": "PT" },
                            "width": { "magnitude": 1.5, "unit": "PT" }
                        },
                        "spaceAbove": { "magnitude": 14.0, "unit": "PT" },
                        "spaceBelow": { "magnitude": 7.0, "unit": "PT" },
                        "spacingMode": "NEVER_COLLAPSE"
                    })),
                },
            )]
            .into_iter()
            .collect(),
            table: None,
            list: None,
        },
    )
    .unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_apply_styles_to(
        &client,
        ApplyStylesCommand {
            document_id: "document-123".into(),
            segment_id: None,
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: false,
            italic: false,
            underline: false,
            font_size: None,
            font_family: None,
            foreground_color: None,
            link_heading_id: None,
            alignment: None,
            direction: None,
            space_above: None,
            space_below: None,
            line_spacing: None,
            spacing_mode: None,
            indent_start: None,
            indent_end: None,
            indent_first_line: None,
            keep_with_next: false,
            keep_lines_together: false,
            avoid_widow_and_orphan: false,
            page_break_before: false,
            heading: Some("HEADING_1".into()),
            style_json: None,
            dry_run: false,
            json: true,
            required_revision_id: Some("rev-initial".into()),
            no_auto_style: false,
        },
        &mut out,
        Some(&documents_url),
        Some(cache_dir.path()),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["writeControl"]["requiredRevisionId"],
        "rev-after-paragraph"
    );

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 3);
    let first_post: serde_json::Value = serde_json::from_slice(&requests[1].body).unwrap();
    let second_post: serde_json::Value = serde_json::from_slice(&requests[2].body).unwrap();

    assert_eq!(
        first_post["writeControl"]["requiredRevisionId"],
        "rev-initial"
    );
    assert!(first_post["requests"][0]["updateParagraphStyle"].is_object());
    assert!(first_post["requests"][0]["updateTextStyle"].is_null());

    assert_eq!(
        second_post["writeControl"]["requiredRevisionId"],
        "rev-after-paragraph"
    );
    assert!(second_post["requests"][0]["updateTextStyle"].is_object());
    assert!(second_post["requests"][0]["updateParagraphStyle"].is_null());
}

#[tokio::test]
async fn run_apply_list_uses_cached_preset_when_flags_are_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let cache_dir = tempfile::tempdir().unwrap();
    save_style_template_in(
        Some(cache_dir.path()),
        &StyleTemplate {
            document_id: "document-123".into(),
            source_revision_id: Some("rev-style".into()),
            named_styles: Default::default(),
            table: None,
            list: Some(ListStyleTemplate {
                list_type: Some("Checkbox".into()),
                preset: "BULLET_CHECKBOX".into(),
            }),
        },
    )
    .unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let documents_url = format!("{}/docs/v1/documents", server.uri());
    let mut out = Vec::new();

    run_apply_list_to(
        &client,
        ApplyListCommand {
            document_id: "document-123".into(),
            selector: RangeSelector::Entry(2),
            list_type: None,
            preset: None,
            dry_run: true,
            json: true,
            required_revision_id: None,
            no_auto_style: false,
        },
        &mut out,
        Some(&documents_url),
        Some(cache_dir.path()),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(
        output["requestBody"]["requests"][0]["createParagraphBullets"]["bulletPreset"],
        "BULLET_CHECKBOX"
    );
}

#[test]
fn run_show_style_template_supports_json_and_missing_cache_message() {
    let cache_dir = tempfile::tempdir().unwrap();
    let template = StyleTemplate {
        document_id: "document-123".into(),
        source_revision_id: Some("rev-style".into()),
        named_styles: [(
            "HEADING_2".to_string(),
            NamedStyleTemplate {
                text_style: TextStyleTemplate {
                    bold: Some(true),
                    italic: None,
                    font_size_pt: Some(16.0),
                    foreground_color: Some("#336699".into()),
                },
                paragraph_style: None,
            },
        )]
        .into_iter()
        .collect(),
        table: None,
        list: Some(ListStyleTemplate {
            list_type: Some("Bullet".into()),
            preset: "BULLET_DISC_CIRCLE_SQUARE".into(),
        }),
    };
    save_style_template_in(Some(cache_dir.path()), &template).unwrap();

    let mut json_out = Vec::new();
    run_show_style_template("document-123", true, &mut json_out, Some(cache_dir.path())).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&json_out).unwrap();
    assert_eq!(json["document_id"], "document-123");
    assert_eq!(json["source_revision_id"], "rev-style");

    let mut missing_out = Vec::new();
    run_show_style_template(
        "missing-document",
        false,
        &mut missing_out,
        Some(cache_dir.path()),
    )
    .unwrap();
    assert_eq!(
        String::from_utf8(missing_out).unwrap(),
        "no cached style template for this document; run `docs get missing-document` first\n"
    );
}

#[tokio::test]
async fn run_create_prints_document_id_and_edit_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(
            serde_json::json!({ "title": "goog-e2e-scratch" }),
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-456",
            "title": "goog-e2e-scratch"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_create_to(
        &client,
        "goog-e2e-scratch".into(),
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "document-456\thttps://docs.google.com/document/d/document-456/edit\n"
    );
}

#[tokio::test]
async fn run_copy_preserves_template_through_drive_and_prints_edit_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/drive/v3/files/source-document-123/copy"))
        .and(wiremock::matchers::query_param(
            "fields",
            "id,name,mimeType,webViewLink",
        ))
        .and(wiremock::matchers::query_param("supportsAllDrives", "true"))
        .and(header("authorization", "Bearer drive-write-access"))
        .and(body_json(serde_json::json!({
            "name": "Customer proposal copy"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "copied-document-456",
            "name": "Customer proposal copy",
            "mimeType": "application/vnd.google-apps.document",
            "webViewLink": "https://docs.google.com/document/d/copied-document-456/edit"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token(
            "alice@example.com",
            &Token {
                access_token: "drive-write-access".into(),
                refresh_token: "refresh-123".into(),
                expiry: Utc::now() + Duration::hours(1),
                scopes: vec![DRIVE_SCOPE.into()],
            },
        )
        .unwrap();
    let client = AuthClient::from_config(test_config(), &store, None).unwrap();
    let mut out = Vec::new();
    let drive_files_url = format!("{}/drive/v3/files", server.uri());

    run_copy_to(
        &client,
        "source-document-123".into(),
        "Customer proposal copy".into(),
        &mut out,
        Some(&drive_files_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "copied-document-456\thttps://docs.google.com/document/d/copied-document-456/edit\n"
    );
}

#[tokio::test]
async fn run_export_pdf_writes_a_native_drive_export() {
    let server = MockServer::start().await;
    let pdf = b"%PDF-1.7\ndocument";
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/document-123/export"))
        .and(wiremock::matchers::query_param(
            "mimeType",
            "application/pdf",
        ))
        .and(header("authorization", "Bearer drive-read-access"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(pdf.to_vec()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token(
            "alice@example.com",
            &Token {
                access_token: "drive-read-access".into(),
                refresh_token: "refresh-123".into(),
                expiry: Utc::now() + Duration::hours(1),
                scopes: vec![DRIVE_SCOPE.into()],
            },
        )
        .unwrap();
    let client = AuthClient::from_config(test_config(), &store, None).unwrap();
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("document.pdf");
    let mut out = Vec::new();

    run_export_pdf_to(
        &client,
        "document-123".into(),
        output.clone(),
        &mut out,
        Some(&format!("{}/drive/v3/files", server.uri())),
    )
    .await
    .unwrap();

    assert_eq!(std::fs::read(&output).unwrap(), pdf);
    assert_eq!(
        String::from_utf8(out).unwrap(),
        format!("{}\t{}\n", output.display(), pdf.len())
    );
}

#[test]
fn pdf_export_access_failures_include_account_and_policy_guidance() {
    for error in [DriveError::NotFound, DriveError::PermissionDenied] {
        let message = format!("{:#}", with_pdf_export_context(error));

        assert!(message.contains("confirm the selected account can access the document"));
        assert!(message.contains("Workspace policies allow downloading, printing, and copying"));
        assert!(message.contains("use --account EMAIL"));
    }
}

#[tokio::test]
async fn run_batch_update_reads_requests_from_file_and_prints_response_json() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "insertText": {
                    "location": { "index": 1 },
                    "text": "Hello"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let request_path = temp_dir.path().join("requests.json");
    std::fs::write(&request_path, request_body.to_string()).unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_batch_update_to(
        &client,
        "document-123".into(),
        request_path.to_string_lossy().into_owned(),
        &mut input,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"replies\":[{}]}\n"
    );
}

#[tokio::test]
async fn run_batch_update_reads_requests_from_stdin() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteContentRange": {
                    "range": {
                        "startIndex": 1,
                        "endIndex": 2
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut input = std::io::Cursor::new(request_body.to_string());
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_batch_update_to(
        &client,
        "document-123".into(),
        "-".into(),
        &mut input,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"replies\":[{}]}\n"
    );
}

#[tokio::test]
async fn run_get_unified_tries_mapped_account_before_active_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "title": "Mapped"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut state = RuntimeState::default();
    state.set_resource_account(resource_key("docs", "document-123"), "bob@example.com");
    save_runtime_state_to_path(&state, &state_path).unwrap();
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_get_unified_to(
        &config,
        &store,
        None,
        "document-123".into(),
        None,
        false,
        &mut out,
        Some(&documents_url),
        Some(&state_path),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"title\":\"Mapped\"}\n"
    );
}

#[tokio::test]
async fn run_map_unified_falls_back_and_maps_successful_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(short_document_with_page_break()))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_map_unified_to(
        &config,
        &store,
        None,
        "document-123".into(),
        DocsMapType::All,
        false,
        &mut out,
        Some(&documents_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let output = String::from_utf8(out).unwrap();
    assert!(output.contains("Project Plan"));
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("docs", "document-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_search_text_unified_falls_back_without_changing_json_shape() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_search_text_unified_to(
        &config,
        &store,
        None,
        "document-123".into(),
        "Plan".into(),
        true,
        &mut out,
        Some(&documents_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output.as_array().unwrap().len(), 2);
    assert_eq!(output[0]["startIndex"], 9);
    assert_eq!(output[1]["preview"], "Second Page Plan");
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("docs", "document-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_get_content_unified_falls_back_without_changing_json_shape() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_get_content_unified_to(
        &config,
        &store,
        None,
        "document-123".into(),
        ContentSelector::Entry(3),
        true,
        &mut out,
        Some(&documents_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["entry"], 3);
    assert_eq!(output["preview"], "Second Page Plan");
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("docs", "document-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn high_level_docs_unified_commands_do_not_fallback_for_explicit_account() {
    let server = MockServer::start().await;
    for document_id in ["map-document", "search-document", "content-document"] {
        Mock::given(method("GET"))
            .and(path(format!("/docs/v1/documents/{document_id}")))
            .and(header("authorization", "Bearer alice-access"))
            .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path(format!("/docs/v1/documents/{document_id}")))
            .and(header("authorization", "Bearer bob-access"))
            .respond_with(ResponseTemplate::new(200).set_body_json(searchable_document()))
            .expect(0)
            .mount(&server)
            .await;
    }

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let map = run_map_unified_to(
        &config,
        &store,
        Some("alice@example.com"),
        "map-document".into(),
        DocsMapType::All,
        false,
        &mut Vec::new(),
        Some(&documents_url),
        Some(&state_path),
    )
    .await;
    let search = run_search_text_unified_to(
        &config,
        &store,
        Some("alice@example.com"),
        "search-document".into(),
        "Plan".into(),
        true,
        &mut Vec::new(),
        Some(&documents_url),
        Some(&state_path),
    )
    .await;
    let content = run_get_content_unified_to(
        &config,
        &store,
        Some("alice@example.com"),
        "content-document".into(),
        ContentSelector::Entry(1),
        true,
        &mut Vec::new(),
        Some(&documents_url),
        Some(&state_path),
    )
    .await;

    for result in [map, search, content] {
        let message = format!("{:#}", result.unwrap_err());
        assert!(message.contains("failed to read Google Docs Document"));
        assert!(message.contains("Google Docs Document was not found"));
    }
    assert!(load_runtime_state_from_path(&state_path)
        .unwrap()
        .resource_account_mappings
        .is_empty());
}

#[tokio::test]
async fn run_get_unified_falls_back_on_target_access_failure_and_repairs_stale_mapping() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for bob"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer carol-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "title": "Carol"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut state = RuntimeState::default();
    state.set_resource_account(resource_key("docs", "document-123"), "bob@example.com");
    save_runtime_state_to_path(&state, &state_path).unwrap();
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_get_unified_to(
        &config,
        &store,
        None,
        "document-123".into(),
        None,
        false,
        &mut out,
        Some(&documents_url),
        Some(&state_path),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("docs", "document-123")),
        Some("carol@example.com")
    );
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"title\":\"Carol\"}\n"
    );
}

#[tokio::test]
async fn run_get_unified_does_not_fallback_for_explicit_account_but_maps_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-456"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-456",
            "title": "Bob"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let mut denied_out = Vec::new();
    let denied = run_get_unified_to(
        &config,
        &store,
        Some("alice@example.com"),
        "document-123".into(),
        None,
        false,
        &mut denied_out,
        Some(&documents_url),
        Some(&state_path),
        None,
    )
    .await;

    let message = format!("{:#}", denied.unwrap_err());
    assert!(message.contains("failed to read Google Docs Document"));
    assert!(message.contains("Google Docs Document was not found"));
    assert!(denied_out.is_empty());

    let mut mapped_out = Vec::new();
    run_get_unified_to(
        &config,
        &store,
        Some("bob@example.com"),
        "document-456".into(),
        None,
        false,
        &mut mapped_out,
        Some(&documents_url),
        Some(&state_path),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("docs", "document-456")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_get_unified_does_not_fallback_on_non_target_api_error() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(500).set_body_string("server broke"))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let result = run_get_unified_to(
        &config,
        &store,
        None,
        "document-123".into(),
        None,
        false,
        &mut out,
        Some(&documents_url),
        Some(&state_path),
        None,
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to read Google Docs Document"));
    assert!(message.contains("Google Docs API error (500 Internal Server Error): server broke"));
    assert!(out.is_empty());
    assert!(load_runtime_state_from_path(&state_path)
        .unwrap()
        .resource_account_mappings
        .is_empty());
}

#[tokio::test]
async fn run_batch_update_unified_uses_same_fallback_and_mapping_behavior_for_writes() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "insertText": {
                    "location": { "index": 1 },
                    "text": "Hello"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer bob-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let request_path = temp_dir.path().join("requests.json");
    std::fs::write(&request_path, request_body.to_string()).unwrap();
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    run_batch_update_unified_to(
        &config,
        &store,
        None,
        "document-123".into(),
        request_path.to_string_lossy().into_owned(),
        &mut input,
        &mut out,
        Some(&documents_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"replies\":[{}]}\n"
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("docs", "document-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_batch_update_returns_clear_error_for_invalid_request_json() {
    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut input = std::io::Cursor::new("{not json");
    let mut out = Vec::new();

    let result = run_batch_update_to(
        &client,
        "document-123".into(),
        "-".into(),
        &mut input,
        &mut out,
        Some("https://example.test/docs/v1/documents"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Docs Batch Update request body"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_batch_update_invalid_file_json_names_request_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let request_path = temp_dir.path().join("requests.json");
    std::fs::write(&request_path, "{not json").unwrap();
    let request_path_arg = request_path.to_string_lossy().into_owned();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();

    let result = run_batch_update_to(
        &client,
        "document-123".into(),
        request_path_arg.clone(),
        &mut input,
        &mut out,
        Some("https://example.test/docs/v1/documents"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Docs Batch Update request body"));
    assert!(message.contains(&request_path_arg));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_get_returns_clear_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/missing-document"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let documents_url = format!("{}/docs/v1/documents", server.uri());

    let result = run_get_to(
        &client,
        "missing-document".into(),
        None,
        false,
        &mut out,
        Some(&documents_url),
        None,
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to read Google Docs Document"));
    assert!(message.contains("Google Docs Document was not found"));
    assert!(out.is_empty());
}

fn short_document_with_page_break() -> serde_json::Value {
    serde_json::json!({
        "documentId": "document-123",
        "title": "Roadmap",
        "revisionId": "rev-short",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 14,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "TITLE" },
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
                    "endIndex": 15,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 14,
                                "endIndex": 15,
                                "pageBreak": {}
                            }
                        ]
                    }
                },
                {
                    "startIndex": 15,
                    "endIndex": 27,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 15,
                                "endIndex": 27,
                                "textRun": { "content": "Second Page\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 27,
                    "endIndex": 28,
                    "sectionBreak": {
                        "sectionStyle": {
                            "sectionType": "NEXT_PAGE",
                            "defaultHeaderId": "header-2",
                            "defaultFooterId": "footer-2"
                        }
                    }
                }
            ]
        }
    })
}

fn document_with_initial_section_and_page_breaks() -> serde_json::Value {
    let mut document = short_document_with_page_break();
    let content = document["body"]["content"].as_array_mut().unwrap();
    content.insert(
        0,
        serde_json::json!({
            "endIndex": 1,
            "sectionBreak": {
                "sectionStyle": {
                    "sectionType": "CONTINUOUS",
                    "contentDirection": "LEFT_TO_RIGHT"
                }
            }
        }),
    );
    document
}

fn searchable_document() -> serde_json::Value {
    serde_json::json!({
        "documentId": "document-123",
        "title": "Searchable",
        "revisionId": "rev-search",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 14,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "TITLE" },
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
                    "endIndex": 36,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 14,
                                "endIndex": 36,
                                "textRun": { "content": "No matching text here\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 36,
                    "endIndex": 37,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 36,
                                "endIndex": 37,
                                "pageBreak": {}
                            }
                        ]
                    }
                },
                {
                    "startIndex": 37,
                    "endIndex": 54,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 37,
                                "endIndex": 54,
                                "textRun": { "content": "Second Page Plan\n" }
                            }
                        ]
                    }
                }
            ]
        }
    })
}

fn ambiguous_heading_document() -> serde_json::Value {
    serde_json::json!({
        "documentId": "document-123",
        "title": "Ambiguous",
        "revisionId": "rev-ambiguous",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 10,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 1,
                                "endIndex": 10,
                                "textRun": { "content": "Overview\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 10,
                    "endIndex": 11,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 10,
                                "endIndex": 11,
                                "pageBreak": {}
                            }
                        ]
                    }
                },
                {
                    "startIndex": 11,
                    "endIndex": 20,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 11,
                                "endIndex": 20,
                                "textRun": { "content": "Overview\n" }
                            }
                        ]
                    }
                }
            ]
        }
    })
}

fn editable_table_document() -> serde_json::Value {
    serde_json::json!({
        "documentId": "document-123",
        "title": "Editable table",
        "revisionId": "rev-table",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 40,
                    "table": {
                        "tableRows": [
                            {
                                "tableCells": [
                                    {
                                        "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        {
                                                            "startIndex": 5,
                                                            "endIndex": 11,
                                                            "textRun": { "content": "Old A\n" }
                                                        }
                                                    ]
                                                }
                                            }
                                        ]
                                    },
                                    {
                                        "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        {
                                                            "startIndex": 13,
                                                            "endIndex": 19,
                                                            "textRun": { "content": "Old B\n" }
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
                                                            "startIndex": 23,
                                                            "endIndex": 29,
                                                            "textRun": { "content": "Old C\n" }
                                                        }
                                                    ]
                                                }
                                            }
                                        ]
                                    },
                                    {
                                        "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        {
                                                            "startIndex": 31,
                                                            "endIndex": 37,
                                                            "textRun": { "content": "Old D\n" }
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

fn long_document_with_toc_and_objects() -> serde_json::Value {
    let mut document = serde_json::json!({
        "documentId": "document-123",
        "title": "คู่มือ Sandcastle",
        "revisionId": "rev-long",
        "lists": {
            "list-abc": {
                "listProperties": {
                    "nestingLevels": [
                        { "glyphSymbol": "●", "glyphFormat": "%0", "startNumber": 1 },
                        { "glyphSymbol": "○", "glyphFormat": "%1", "startNumber": 1 }
                    ]
                }
            }
        },
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 24,
                    "tableOfContents": {
                        "content": [
                            {
                                "startIndex": 2,
                                "endIndex": 23,
                                "paragraph": {
                                    "paragraphStyle": {
                                        "namedStyleType": "NORMAL_TEXT",
                                        "indentStart": { "magnitude": 18, "unit": "PT" }
                                    },
                                    "elements": [
                                        {
                                            "startIndex": 2,
                                            "endIndex": 23,
                                            "textRun": {
                                                "content": "วิธีใช้งาน\t3\n",
                                                "textStyle": {
                                                    "link": { "headingId": "h.how-to" },
                                                    "weightedFontFamily": {
                                                        "fontFamily": "Bai Jamjuree"
                                                    }
                                                }
                                            }
                                        }
                                    ]
                                }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 24,
                    "endIndex": 35,
                    "paragraph": {
                        "paragraphStyle": {
                            "namedStyleType": "HEADING_1",
                            "headingId": "h.how-to",
                            "alignment": "CENTER",
                            "spaceBelow": { "magnitude": 10, "unit": "PT" },
                            "keepWithNext": true
                        },
                        "elements": [
                            {
                                "startIndex": 24,
                                "endIndex": 28,
                                "textRun": {
                                    "content": "วิธี",
                                    "textStyle": {
                                        "bold": true,
                                        "weightedFontFamily": { "fontFamily": "Bai Jamjuree" }
                                    }
                                }
                            },
                            {
                                "startIndex": 28,
                                "endIndex": 35,
                                "textRun": {
                                    "content": "ใช้งาน\n",
                                    "textStyle": {
                                        "underline": true,
                                        "foregroundColor": {
                                            "color": { "rgbColor": { "red": 0.2 } }
                                        }
                                    }
                                }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 35,
                    "endIndex": 74,
                    "paragraph": {
                        "bullet": { "listId": "list-abc", "nestingLevel": 1 },
                        "paragraphStyle": { "namedStyleType": "NORMAL_TEXT" },
                        "elements": [
                            {
                                "startIndex": 35,
                                "endIndex": 74,
                                "textRun": { "content": "เอกสารนี้มีข้อความภาษาไทยสำหรับทดสอบ\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 74,
                    "endIndex": 103,
                    "table": {
                        "tableStyle": {
                            "tableColumnProperties": [
                                {
                                    "width": { "magnitude": 144, "unit": "PT" },
                                    "widthType": "FIXED_WIDTH"
                                },
                                {
                                    "width": { "magnitude": 324, "unit": "PT" },
                                    "widthType": "FIXED_WIDTH"
                                }
                            ]
                        },
                        "tableRows": [
                            {
                                "tableRowStyle": { "tableHeader": true },
                                "tableCells": [
                                    {
                                        "tableCellStyle": {
                                            "contentAlignment": "MIDDLE",
                                            "backgroundColor": {
                                                "color": {
                                                    "rgbColor": {
                                                        "red": 0.7,
                                                        "green": 0.8,
                                                        "blue": 0.9
                                                    }
                                                }
                                            },
                                            "borderBottom": {
                                                "width": { "magnitude": 1, "unit": "PT" },
                                                "dashStyle": "SOLID",
                                                "color": {
                                                    "color": {
                                                        "rgbColor": {
                                                            "red": 1,
                                                            "green": 1,
                                                            "blue": 1
                                                        }
                                                    }
                                                }
                                            }
                                        },
                                                "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        {
                                                            "startIndex": 77,
                                                            "endIndex": 84,
                                                            "textRun": {
                                                                "content": "หัวข้อ\n",
                                                                "textStyle": { "bold": true }
                                                            }
                                                        }
                                                    ]
                                                }
                                            }
                                        ]
                                    },
                                    {
                                        "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        {
                                                            "startIndex": 85,
                                                            "endIndex": 91,
                                                            "textRun": {
                                                                "content": "สถานะ\n",
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
                },
                {
                    "startIndex": 103,
                    "endIndex": 104,
                    "paragraph": {
                        "paragraphStyle": { "alignment": "CENTER" },
                        "elements": [
                            {
                                "startIndex": 103,
                                "endIndex": 104,
                                "inlineObjectElement": {
                                    "inlineObjectId": "inline-image-1"
                                }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 104,
                    "endIndex": 105,
                    "paragraph": {
                        "positionedObjectIds": ["positioned-image-1"],
                        "elements": [
                            {
                                "startIndex": 104,
                                "endIndex": 105,
                                "textRun": { "content": "\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 105,
                    "endIndex": 106,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 105,
                                "endIndex": 106,
                                "pageBreak": {}
                            }
                        ]
                    }
                },
                {
                    "startIndex": 106,
                    "endIndex": 119,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_2" },
                        "elements": [
                            {
                                "startIndex": 106,
                                "endIndex": 119,
                                "textRun": { "content": "Appendix\n" }
                            }
                        ]
                    }
                }
            ]
        },
        "headers": {
            "legacy-header": {
                "headerId": "legacy-header",
                "content": [{
                    "endIndex": 14,
                    "paragraph": {
                        "paragraphStyle": {
                            "alignment": "END",
                            "spaceBelow": { "magnitude": 4, "unit": "PT" }
                        },
                        "elements": [{
                            "endIndex": 14,
                            "textRun": { "content": "Legacy header\n" }
                        }]
                    }
                }]
            }
        },
        "inlineObjects": {
            "inline-image-1": {
                "inlineObjectProperties": {
                    "embeddedObject": {
                        "title": "Process overview",
                        "description": "Workflow from intake to delivery",
                        "size": {
                            "height": { "magnitude": 81, "unit": "PT" },
                            "width": { "magnitude": 144, "unit": "PT" }
                        },
                        "marginLeft": { "magnitude": 9, "unit": "PT" },
                        "imageProperties": {
                            "cropProperties": { "cropLeft": 0.1 }
                        }
                    }
                }
            }
        },
        "positionedObjects": {
            "positioned-image-1": {
                "positionedObjectProperties": {
                    "positioning": {
                        "layout": "WRAP_TEXT",
                        "leftOffset": {
                            "magnitude": 12,
                            "unit": "PT"
                        },
                        "topOffset": {
                            "magnitude": 24,
                            "unit": "PT"
                        }
                    },
                    "embeddedObject": {
                        "title": "Page decoration",
                        "size": {
                            "height": {
                                "magnitude": 72,
                                "unit": "PT"
                            },
                            "width": {
                                "magnitude": 96,
                                "unit": "PT"
                            }
                        },
                        "imageProperties": {}
                    }
                }
            }
        },
        "tabs": [{
            "tabProperties": { "tabId": "tab-1" },
            "documentTab": {
                "documentStyle": {
                    "pageSize": {
                        "width": { "magnitude": 612, "unit": "PT" },
                        "height": { "magnitude": 792, "unit": "PT" }
                    },
                    "marginTop": { "magnitude": 72, "unit": "PT" }
                },
                "namedStyles": {
                    "styles": [{
                        "namedStyleType": "NORMAL_TEXT",
                        "textStyle": {
                            "weightedFontFamily": {
                                "fontFamily": "Bai Jamjuree",
                                "weight": 400
                            }
                        }
                    }]
                },
                "headers": {
                    "header-123": {
                        "headerId": "header-123",
                        "content": [{
                            "endIndex": 17,
                            "paragraph": {
                                "paragraphStyle": {
                                    "alignment": "CENTER",
                                    "lineSpacing": 100
                                },
                                "elements": [
                                    {
                                        "startIndex": 0,
                                        "endIndex": 16,
                                        "textRun": {
                                            "content": "Customer contact",
                                            "textStyle": {
                                                "weightedFontFamily": {
                                                    "fontFamily": "Bai Jamjuree",
                                                    "weight": 400
                                                }
                                            }
                                        }
                                    },
                                    {
                                        "startIndex": 16,
                                        "endIndex": 17,
                                        "autoText": {
                                            "type": "PAGE_NUMBER",
                                            "textStyle": {
                                                "fontSize": { "magnitude": 10, "unit": "PT" }
                                            }
                                        }
                                    }
                                ]
                            }
                        }]
                    }
                },
                "footers": {
                    "footer-123": {
                        "footerId": "footer-123",
                        "content": [{
                            "endIndex": 1,
                            "paragraph": {
                                "elements": [{
                                    "endIndex": 1,
                                    "textRun": { "content": "\n" }
                                }]
                            }
                        }]
                    }
                },
                "inlineObjects": {
                    "header-inline-image": {
                        "inlineObjectProperties": {
                            "embeddedObject": {
                                "description": "Customer header logo",
                                "size": {
                                    "height": { "magnitude": 24, "unit": "PT" },
                                    "width": { "magnitude": 48, "unit": "PT" }
                                },
                                "imageProperties": {}
                            }
                        }
                    }
                },
                "positionedObjects": {
                    "footer-positioned-image": {
                        "positionedObjectProperties": {
                            "positioning": {
                                "layout": "BEHIND_TEXT"
                            },
                            "embeddedObject": {
                                "title": "Footer decoration",
                                "size": {
                                    "height": {
                                        "magnitude": 24,
                                        "unit": "PT"
                                    },
                                    "width": {
                                        "magnitude": 48,
                                        "unit": "PT"
                                    }
                                },
                                "imageProperties": {}
                            }
                        }
                    }
                }
            }
        }]
    });
    let table_paragraph = document
        .pointer_mut("/body/content/3/table/tableRows/0/tableCells/0/content/0")
        .unwrap();
    table_paragraph["startIndex"] = serde_json::json!(77);
    table_paragraph["endIndex"] = serde_json::json!(84);
    table_paragraph["paragraph"]["paragraphStyle"] = serde_json::json!({
        "namedStyleType": "NORMAL_TEXT",
        "alignment": "CENTER"
    });
    document
}

fn document_with_multiple_inline_images() -> serde_json::Value {
    serde_json::json!({
        "documentId": "document-123",
        "title": "Images",
        "revisionId": "rev-images",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 3,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 1,
                                "endIndex": 2,
                                "inlineObjectElement": {
                                    "inlineObjectId": "inline-image-1"
                                }
                            },
                            {
                                "startIndex": 2,
                                "endIndex": 3,
                                "inlineObjectElement": {
                                    "inlineObjectId": "inline-image-2"
                                }
                            }
                        ]
                    }
                }
            ]
        }
    })
}
