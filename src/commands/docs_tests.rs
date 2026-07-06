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
use crate::cli::DocsListType;
use crate::docs::map::{ContentSelector, InsertTextSelector, RangeSelector};
use crate::docs::style_template::{
    load_style_template_in, save_style_template_in, ListStyleTemplate, NamedStyleTemplate,
    StyleTemplate, TextStyleTemplate,
};
use crate::docs::DOCS_SCOPE;

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
async fn run_map_json_emits_structured_locations_for_long_document_shape() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(long_document_with_toc_and_objects()),
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
        true,
        &mut out,
        Some(&documents_url),
    )
    .await
    .unwrap();

    let output: serde_json::Value = serde_json::from_slice(&out).unwrap();
    assert_eq!(output["revisionId"], "rev-long");
    assert_eq!(output["documentLocations"].as_array().unwrap().len(), 6);
    assert_eq!(
        output["entries"][0]["location"]["confidence"],
        "table-of-contents"
    );
    assert_eq!(output["entries"][0]["location"]["page"], 3);
    assert_eq!(output["entries"][0]["preview"], "วิธีใช้งาน");
    assert_eq!(output["entries"][1]["location"]["confidence"], "unknown");
    assert!(output["entries"][1]["location"]["page"].is_null());
    assert_eq!(output["entries"][2]["kind"], "table");
    assert_eq!(output["entries"][2]["preview"], "หัวข้อ | สถานะ");
    assert_eq!(output["entries"][3]["kind"], "inline-image");
    assert_eq!(output["entries"][4]["kind"], "positioned-image");
    assert_eq!(
        output["entries"][5]["location"]["confidence"],
        "explicit-page-break"
    );
    assert_eq!(output["entries"][5]["location"]["page"], 2);
    assert_eq!(output["entries"][5]["location"]["contentLine"], 1);
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
    assert!(message.contains("ambiguous replace-text match"));
    assert!(message.contains("match 1 index 9"));
    assert!(message.contains("match 2 index 49"));
    assert!(out.is_empty());

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
}

#[tokio::test]
async fn run_list_images_and_tables_emit_document_map_metadata() {
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
    run_list_images_to(
        &client,
        "document-123".into(),
        true,
        &mut images,
        Some(&documents_url),
    )
    .await
    .unwrap();
    let images: serde_json::Value = serde_json::from_slice(&images).unwrap();
    assert_eq!(images.as_array().unwrap().len(), 2);
    assert_eq!(images[0]["kind"], "inline-image");
    assert_eq!(images[0]["imageHandle"], "image-1");
    assert_eq!(images[0]["objectId"], "inline-image-1");
    assert!(images[0].get("layoutMetadata").is_none());
    assert_eq!(images[1]["kind"], "positioned-image");
    assert_eq!(images[1]["imageHandle"], "image-2");
    assert_eq!(images[1]["objectId"], "positioned-image-1");
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

    let mut human_images = Vec::new();
    run_list_images_to(
        &client,
        "document-123".into(),
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

    let mut tables = Vec::new();
    run_list_tables_to(
        &client,
        "document-123".into(),
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
            selector: InsertTextSelector::PageLine { page: 2, line: 1 },
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
async fn run_insert_table_dry_run_populates_csv_data_from_document_end() {
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
        output["requestBody"]["requests"][1]["insertText"]["location"]["index"],
        59
    );
    assert_eq!(
        output["requestBody"]["requests"][1]["insertText"]["text"],
        "D2"
    );
    assert_eq!(
        output["requestBody"]["requests"][8]["insertText"]["location"]["index"],
        48
    );
    assert_eq!(
        output["requestBody"]["requests"][8]["insertText"]["text"],
        "A1"
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
    assert_eq!(
        output["requestBody"]["requests"][1]["insertText"]["text"],
        "Bottom right"
    );
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
            selector: InsertTextSelector::PageLine { page: 2, line: 1 },
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
    assert!(output.contains("insert-image: Insert inline image at index 37"));
    assert!(output.contains("Before: Second Page Plan"));
    assert!(output.contains("After: [inline image]Second Page Plan"));

    let requests = server.received_requests().await.unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].method.as_str(), "GET");
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
    assert!(message.contains("edit-table data dimensions are 1x2"));
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
    assert!(message.contains("edit-table --resize is not supported yet"));
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
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: true,
            italic: true,
            font_size: Some(14.0),
            foreground_color: Some("#336699".into()),
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
        "bold,italic,fontSize,foregroundColor"
    );
    assert_eq!(
        styles["requestBody"]["requests"][0]["updateParagraphStyle"]["paragraphStyle"]
            ["namedStyleType"],
        "HEADING_2"
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
    assert!(message.contains("ambiguous replace-text match"));
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
            selector: RangeSelector::IndexRange {
                start_index: 17,
                end_index: 30,
            },
            bold: false,
            italic: false,
            font_size: None,
            foreground_color: None,
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
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: true,
            italic: false,
            font_size: None,
            foreground_color: None,
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
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: false,
            italic: false,
            font_size: None,
            foreground_color: None,
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
            selector: RangeSelector::Text {
                text: "matching text".into(),
                match_number: None,
            },
            bold: false,
            italic: false,
            font_size: None,
            foreground_color: None,
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
        assert!(message.contains("failed to fetch Google Docs Document"));
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
    assert!(message.contains("failed to fetch Google Docs Document"));
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
    assert!(message.contains("failed to fetch Google Docs Document"));
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
    assert!(message.contains("failed to fetch Google Docs Document"));
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
                }
            ]
        }
    })
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
    serde_json::json!({
        "documentId": "document-123",
        "title": "คู่มือ Sandcastle",
        "revisionId": "rev-long",
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
                                    "elements": [
                                        {
                                            "startIndex": 2,
                                            "endIndex": 23,
                                            "textRun": { "content": "วิธีใช้งาน\t3\n" }
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
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 24,
                                "endIndex": 35,
                                "textRun": { "content": "วิธีใช้งาน\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 35,
                    "endIndex": 74,
                    "paragraph": {
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
                        "tableRows": [
                            {
                                "tableCells": [
                                    {
                                        "content": [
                                            {
                                                "paragraph": {
                                                    "elements": [
                                                        { "textRun": { "content": "หัวข้อ\n" } }
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
                                                        { "textRun": { "content": "สถานะ\n" } }
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
        "inlineObjects": {
            "inline-image-1": {
                "inlineObjectProperties": {
                    "embeddedObject": {
                        "imageProperties": {}
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
        }
    })
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
