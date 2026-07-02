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
use crate::docs::{DOCS_READONLY_SCOPE, DOCS_SCOPE};

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
        scopes: vec![DOCS_READONLY_SCOPE.into(), DOCS_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &docs_token())
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
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
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"title\":\"Roadmap\"}\n"
    );
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
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"documentId\":\"document-123\",\"title\":\"Mapped\"}\n"
    );
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
                    "embeddedObject": {
                        "imageProperties": {}
                    }
                }
            }
        }
    })
}
