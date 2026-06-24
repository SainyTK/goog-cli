use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use wiremock::matchers::{body_json, body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::{AuthClient, AuthorizationCode, AuthorizationCodeFlow};
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::error::AuthError;
use crate::auth::testing::MemoryStore;
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
        scopes: vec![DOCS_READONLY_SCOPE.into()],
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

fn profile_token() -> Token {
    Token {
        access_token: "profile-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec!["openid".into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store.save_token("alice@example.com", &docs_token()).unwrap();
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
    store.save_token("alice@example.com", &docs_write_token()).unwrap();
    let client = AuthClient::from_config(test_config(), &store, None).unwrap();
    let options = BatchUpdateDocumentOptions::new("document-123", request_body)
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    let response = batch_update_document(&client, &options).await.unwrap();

    assert_eq!(response["documentId"], "document-123");
    assert_eq!(response["replies"].as_array().unwrap().len(), 1);
}

#[tokio::test]
async fn get_document_requests_only_readonly_docs_scope_when_missing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("code=docs-code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "docs-access",
            "expires_in": 3600,
            "scope": DOCS_READONLY_SCOPE,
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
        .with_auth_urls_for_tests("https://example.test/auth", format!("{}/token", server.uri()))
        .with_authorization_code_flow_for_tests(Box::new(StaticAuthorizationCodeFlow {
            scopes_seen: scopes_seen.clone(),
        }));
    let options = GetDocumentOptions::new("document-123")
        .with_documents_url(format!("{}/docs/v1/documents", server.uri()));

    get_document(&client, &options).await.unwrap();

    assert_eq!(
        scopes_seen.lock().unwrap().clone(),
        vec![DOCS_READONLY_SCOPE.to_string()]
    );
    let saved = store.load_token("alice@example.com").unwrap().unwrap();
    assert_eq!(
        saved.scopes,
        vec!["openid".to_string(), DOCS_READONLY_SCOPE.to_string()]
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
    store.save_token("alice@example.com", &profile_token()).unwrap();
    let scopes_seen = Arc::new(Mutex::new(Vec::new()));
    let client = AuthClient::from_config(test_config(), &store, None)
        .unwrap()
        .with_auth_urls_for_tests("https://example.test/auth", format!("{}/token", server.uri()))
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
