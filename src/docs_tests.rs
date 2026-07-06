use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use wiremock::matchers::{body_json, body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::{AuthClient, AuthorizationCode, AuthorizationCodeFlow};
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::error::AuthError;
use crate::auth::testing::MemoryStore;
use crate::docs::map::{
    build_document_map, resolve_content_entry, resolve_insert_text_location,
    resolve_range_selector, ContentSelector, InsertTextSelector, RangeSelector,
};
use crate::docs::*;

const DOCUMENT_RESPONSE: &str = r#"{
  "documentId": "document-123",
  "title": "Roadmap",
  "body": {
    "content": [
      {
        "startIndex": 1,
        "endIndex": 2,
        "paragraph": {
          "elements": [
            {
              "startIndex": 1,
              "endIndex": 2,
              "textRun": {
                "content": "\n"
              }
            }
          ]
        }
      }
    ]
  }
}"#;

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
    Token {
        access_token: "docs-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![DOCS_SCOPE.into()],
    }
}

fn docs_write_token() -> Token {
    Token {
        access_token: "docs-write-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![DOCS_SCOPE.into()],
    }
}

fn docs_and_drive_token() -> Token {
    Token {
        scopes: vec![DOCS_SCOPE.into(), crate::drive::DRIVE_SCOPE.into()],
        ..docs_token()
    }
}

fn profile_token() -> Token {
    Token {
        access_token: "profile-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec!["openid".into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &docs_token())
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

struct StaticAuthorizationCodeFlow {
    scopes_seen: Arc<Mutex<Vec<String>>>,
}

impl AuthorizationCodeFlow for StaticAuthorizationCodeFlow {
    fn authorize(
        &self,
        auth_url: &str,
        client_id: &str,
        _state: &str,
        scopes: &[&str],
    ) -> Result<AuthorizationCode, AuthError> {
        assert_eq!(auth_url, "https://example.test/auth");
        assert_eq!(client_id, "client-123");
        *self.scopes_seen.lock().unwrap() = scopes.iter().map(|scope| scope.to_string()).collect();

        Ok(AuthorizationCode {
            redirect_uri: "http://127.0.0.1:54321/".into(),
            code: "docs-code".into(),
        })
    }
}

#[tokio::test]
async fn get_document_fetches_raw_google_docs_document() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DOCUMENT_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetDocumentOptions::new("document-123")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let document = get_document(&client, &options).await.unwrap();

    assert_eq!(document["documentId"], "document-123");
    assert_eq!(document["title"], "Roadmap");
    assert!(document["body"]["content"].is_array());
}

#[tokio::test]
async fn get_document_passes_google_query_options() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(query_param("fields", "documentId,title"))
        .and(query_param("includeTabsContent", "true"))
        .and(header("authorization", "Bearer docs-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DOCUMENT_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetDocumentOptions::new("document-123")
        .with_fields("documentId,title")
        .with_include_tabs_content(true)
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let document = get_document(&client, &options).await.unwrap();

    assert_eq!(document["documentId"], "document-123");
}

#[tokio::test]
async fn get_document_omits_include_tabs_content_by_default() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DOCUMENT_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetDocumentOptions::new("document-123")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    get_document(&client, &options).await.unwrap();

    let requests = server.received_requests().await.unwrap();
    let request = requests.first().unwrap();
    assert!(!request
        .url
        .query_pairs()
        .any(|(name, _)| name == "includeTabsContent"));
}

#[tokio::test]
async fn batch_update_posts_full_google_request_body() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "insertText": {
                    "location": { "index": 1 },
                    "text": "Hello"
                }
            }
        ],
        "writeControl": {
            "requiredRevisionId": "rev-123"
        }
    });
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": [{}],
            "writeControl": {
                "requiredRevisionId": "rev-456"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &docs_write_token())
        .unwrap();
    let client = AuthClient::from_config(test_config(), &store, None).unwrap();
    let options = BatchUpdateDocumentOptions::new("document-123", request_body)
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let response = batch_update_document(&client, &options).await.unwrap();

    assert_eq!(response["documentId"], "document-123");
    assert_eq!(response["replies"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn get_document_requests_full_docs_scope_when_missing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("code=docs-code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "docs-access",
            "expires_in": 3600,
            "scope": DOCS_SCOPE,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DOCUMENT_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &profile_token())
        .unwrap();
    let scopes_seen = Arc::new(Mutex::new(Vec::new()));
    let client = AuthClient::from_config(test_config(), &store, None)
        .unwrap()
        .with_auth_urls_for_tests(
            "https://example.test/auth",
            format!("{}/token", server.uri()),
        )
        .with_authorization_code_flow_for_tests(Box::new(StaticAuthorizationCodeFlow {
            scopes_seen: scopes_seen.clone(),
        }));
    let options = GetDocumentOptions::new("document-123")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    get_document(&client, &options).await.unwrap();

    assert_eq!(
        scopes_seen.lock().unwrap().clone(),
        vec![DOCS_SCOPE.to_string()]
    );
    let saved = store.load_token("alice@example.com").unwrap().unwrap();
    assert_eq!(
        saved.scopes,
        vec!["openid".to_string(), DOCS_SCOPE.to_string()]
    );
}

#[tokio::test]
async fn batch_update_requests_write_docs_scope_when_missing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("code=docs-code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "docs-write-access",
            "expires_in": 3600,
            "scope": DOCS_SCOPE,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/document-123:batchUpdate"))
        .and(header("authorization", "Bearer docs-write-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "document-123",
            "replies": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &profile_token())
        .unwrap();
    let scopes_seen = Arc::new(Mutex::new(Vec::new()));
    let client = AuthClient::from_config(test_config(), &store, None)
        .unwrap()
        .with_auth_urls_for_tests(
            "https://example.test/auth",
            format!("{}/token", server.uri()),
        )
        .with_authorization_code_flow_for_tests(Box::new(StaticAuthorizationCodeFlow {
            scopes_seen: scopes_seen.clone(),
        }));
    let options = BatchUpdateDocumentOptions::new(
        "document-123",
        serde_json::json!({
            "requests": []
        }),
    )
    .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    batch_update_document(&client, &options).await.unwrap();

    assert_eq!(
        scopes_seen.lock().unwrap().clone(),
        vec![DOCS_SCOPE.to_string()]
    );
    let saved = store.load_token("alice@example.com").unwrap().unwrap();
    assert_eq!(
        saved.scopes,
        vec!["openid".to_string(), DOCS_SCOPE.to_string()]
    );
}

#[tokio::test]
async fn get_document_returns_docs_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/private-document"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetDocumentOptions::new("private-document")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let err = get_document(&client, &options).await.unwrap_err();

    assert!(matches!(err, DocsError::PermissionDenied));
}

#[tokio::test]
async fn get_document_returns_api_error_with_response_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream failure"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetDocumentOptions::new("document-123")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let err = get_document(&client, &options).await.unwrap_err();

    match err {
        DocsError::Api { status, body } => {
            assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "upstream failure");
        }
        _ => panic!("unexpected error: {err}"),
    }
}

#[tokio::test]
async fn get_document_returns_invalid_url_error_for_malformed_api_url() {
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetDocumentOptions::new("document-123").with_documents_url("://bad-url");

    let err = get_document(&client, &options).await.unwrap_err();

    assert!(matches!(err, DocsError::InvalidUrl(_)));
}

#[tokio::test]
async fn get_document_returns_invalid_response_error_for_malformed_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not json"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetDocumentOptions::new("document-123")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let err = get_document(&client, &options).await.unwrap_err();

    assert!(matches!(err, DocsError::InvalidResponse(_)));
}

fn office_file_precondition_response() -> ResponseTemplate {
    ResponseTemplate::new(400).set_body_json(serde_json::json!({
        "error": {
            "code": 400,
            "status": "FAILED_PRECONDITION",
            "message": "This operation is not supported for this document. The document must not be an Office file."
        }
    }))
}

#[tokio::test]
async fn get_document_converts_office_file_then_reads_temporary_document() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/office-file-123"))
        .respond_with(office_file_precondition_response())
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/drive/v3/files/office-file-123/copy"))
        .and(query_param("fields", "id"))
        .and(query_param("supportsAllDrives", "true"))
        .and(body_json(&serde_json::json!({
            "mimeType": "application/vnd.google-apps.document",
            "name": "goog temporary Docs conversion"
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "converted-document-456"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/converted-document-456"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "documentId": "converted-document-456",
            "title": "Converted from office-file-123"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("DELETE"))
        .and(path("/drive/v3/files/converted-document-456"))
        .and(query_param("supportsAllDrives", "true"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &docs_and_drive_token())
        .unwrap();
    let client = AuthClient::from_config(test_config(), &store, None).unwrap();
    let options = GetDocumentOptions::new("office-file-123")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()))
        .with_drive_files_url(format!("{}/drive/v3/files", server.uri()));

    let document = get_document(&client, &options).await.unwrap();

    assert_eq!(document["documentId"], "converted-document-456");
    assert_eq!(document["title"], "Converted from office-file-123");
}

#[tokio::test]
async fn batch_update_reports_office_file_precondition_clearly() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/docs/v1/documents/office-file-123:batchUpdate"))
        .respond_with(office_file_precondition_response())
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &docs_write_token())
        .unwrap();
    let client = AuthClient::from_config(test_config(), &store, None).unwrap();
    let options =
        BatchUpdateDocumentOptions::new("office-file-123", serde_json::json!({ "requests": [] }))
            .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let err = batch_update_document(&client, &options).await.unwrap_err();

    assert!(matches!(err, DocsError::UnsupportedOfficeFile));
}

#[test]
fn extract_document_id_passes_through_a_bare_document_id() {
    assert_eq!(
        extract_document_id("placeholder-document-id"),
        "placeholder-document-id"
    );
}

#[test]
fn extract_document_id_extracts_id_from_docs_edit_url() {
    assert_eq!(
        extract_document_id("https://docs.google.com/document/d/placeholder-document-id/edit"),
        "placeholder-document-id"
    );
}

#[test]
fn extract_document_id_extracts_id_from_docs_edit_url_with_query_and_fragment() {
    assert_eq!(
        extract_document_id(
            "https://docs.google.com/document/d/placeholder-document-id/edit?tab=t.0#heading=h.abc"
        ),
        "placeholder-document-id"
    );
}

#[test]
fn extract_document_id_extracts_id_from_drive_file_url() {
    assert_eq!(
        extract_document_id("https://drive.google.com/file/d/placeholder-document-id/view"),
        "placeholder-document-id"
    );
}

#[test]
fn extract_document_id_extracts_id_from_drive_open_query_param() {
    assert_eq!(
        extract_document_id("https://drive.google.com/open?id=placeholder-document-id"),
        "placeholder-document-id"
    );
}

#[test]
fn extract_document_id_trims_surrounding_whitespace() {
    assert_eq!(
        extract_document_id("  placeholder-document-id  "),
        "placeholder-document-id"
    );
}

#[test]
fn document_map_resolves_content_selectors_at_the_map_boundary() {
    let document_map = build_document_map(&selector_document());

    let by_index = resolve_content_entry(&document_map, &ContentSelector::Index(44)).unwrap();
    assert_eq!(by_index.preview, "Second page plan");

    let by_entry = resolve_content_entry(&document_map, &ContentSelector::Entry(2)).unwrap();
    assert_eq!(by_entry.preview, "Second page plan");

    let by_page_line = resolve_content_entry(
        &document_map,
        &ContentSelector::PageLine { page: 2, line: 1 },
    )
    .unwrap();
    assert_eq!(by_page_line.preview, "Second page plan");

    let by_heading =
        resolve_content_entry(&document_map, &ContentSelector::Heading("Overview".into())).unwrap();
    assert_eq!(by_heading.preview, "Overview");
}

#[test]
fn document_map_resolves_insert_selectors_at_the_map_boundary() {
    let document_map = build_document_map(&selector_document());

    let by_index =
        resolve_insert_text_location(&document_map, &InsertTextSelector::Index(51)).unwrap();
    assert_eq!(by_index.location.index, Some(51));
    assert_eq!(by_index.preview_before, "Second page plan");

    let after_heading = resolve_insert_text_location(
        &document_map,
        &InsertTextSelector::AfterHeading("Overview".into()),
    )
    .unwrap();
    assert_eq!(after_heading.location.index, Some(10));

    let before_text = resolve_insert_text_location(
        &document_map,
        &InsertTextSelector::BeforeText("unique anchor".into()),
    )
    .unwrap();
    assert_eq!(before_text.location.index, Some(61));
}

#[test]
fn document_map_resolves_range_selectors_at_the_map_boundary() {
    let document_map = build_document_map(&selector_document());

    let by_entry = resolve_range_selector(&document_map, &RangeSelector::Entry(2)).unwrap();
    assert_eq!(by_entry.start_index, 41);
    assert_eq!(by_entry.end_index, 58);

    let by_text = resolve_range_selector(
        &document_map,
        &RangeSelector::Text {
            text: "unique anchor".into(),
            match_number: Some(1),
        },
    )
    .unwrap();
    assert_eq!(by_text.start_index, 61);
    assert_eq!(by_text.end_index, 74);
}

#[test]
fn document_map_rejects_ambiguous_heading_and_text_selectors_with_candidates() {
    let document_map = build_document_map(&ambiguous_selector_document());

    let heading_error =
        resolve_content_entry(&document_map, &ContentSelector::Heading("Duplicate".into()))
            .unwrap_err()
            .to_string();
    assert!(heading_error.contains("ambiguous heading selector \"Duplicate\"; candidates:"));
    assert!(heading_error.contains("entry 1 index 1 page - line 1 preview Duplicate"));
    assert!(heading_error.contains("entry 2 index 11 page - line 2 preview Duplicate"));

    let text_error = resolve_insert_text_location(
        &document_map,
        &InsertTextSelector::BeforeText("Duplicate".into()),
    )
    .unwrap_err()
    .to_string();
    assert!(text_error.contains("ambiguous text selector \"Duplicate\"; candidates:"));
    assert!(text_error.contains("match 1 index 1 page - line 1 preview Duplicate"));
    assert!(text_error.contains("match 2 index 11 page - line 2 preview Duplicate"));
}

#[test]
fn document_map_resolves_index_insertion_in_an_empty_document() {
    let document_map = build_document_map(&serde_json::json!({
        "documentId": "document-123",
        "title": "Empty",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 2,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 1,
                                "endIndex": 2,
                                "textRun": { "content": "\n" }
                            }
                        ]
                    }
                }
            ]
        }
    }));

    assert!(document_map.entries.is_empty());
    let resolved =
        resolve_insert_text_location(&document_map, &InsertTextSelector::Index(1)).unwrap();
    assert_eq!(resolved.location.index, Some(1));
    assert_eq!(resolved.preview_before, "");
    assert_eq!(resolved.preview_offset, 0);
}

fn selector_document() -> serde_json::Value {
    serde_json::json!({
        "documentId": "document-123",
        "title": "Selectors",
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
                    "startIndex": 40,
                    "endIndex": 41,
                    "paragraph": {
                        "elements": [
                            { "pageBreak": {} }
                        ]
                    }
                },
                {
                    "startIndex": 41,
                    "endIndex": 58,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 41,
                                "endIndex": 58,
                                "textRun": { "content": "Second page plan\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 61,
                    "endIndex": 88,
                    "paragraph": {
                        "elements": [
                            {
                                "startIndex": 61,
                                "endIndex": 88,
                                "textRun": { "content": "unique anchor paragraph\n" }
                            }
                        ]
                    }
                }
            ]
        }
    })
}

fn ambiguous_selector_document() -> serde_json::Value {
    serde_json::json!({
        "documentId": "document-123",
        "title": "Ambiguous",
        "body": {
            "content": [
                {
                    "startIndex": 1,
                    "endIndex": 11,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 1,
                                "endIndex": 11,
                                "textRun": { "content": "Duplicate\n" }
                            }
                        ]
                    }
                },
                {
                    "startIndex": 11,
                    "endIndex": 21,
                    "paragraph": {
                        "paragraphStyle": { "namedStyleType": "HEADING_1" },
                        "elements": [
                            {
                                "startIndex": 11,
                                "endIndex": 21,
                                "textRun": { "content": "Duplicate\n" }
                            }
                        ]
                    }
                }
            ]
        }
    })
}
