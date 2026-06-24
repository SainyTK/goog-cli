use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, SettingsConfig};
use crate::auth::testing::MemoryStore;
use crate::docs::DOCS_READONLY_SCOPE;

use super::docs::*;

fn test_config() -> Config {
    Config {
        oauth_app: Some(OAuthAppConfig {
            client_id: "client-123".into(),
            client_secret: "secret-456".into(),
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

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store.save_token("alice@example.com", &docs_token()).unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

#[tokio::test]
async fn run_get_prints_document_json_to_stdout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/docs/v1/documents/document-123"))
        .and(header("authorization", "Bearer docs-access"))
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
        &mut out,
        Some(&documents_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to fetch Google Docs Document"));
    assert!(message.contains("Google Docs Document was not found"));
    assert!(out.is_empty());
}
