use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::testing::MemoryStore;
use crate::sheets::SHEETS_READONLY_SCOPE;

use super::sheets::*;

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

fn sheets_token() -> Token {
    Token {
        access_token: "sheets-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![SHEETS_READONLY_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &sheets_token())
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

#[tokio::test]
async fn run_get_prints_spreadsheet_json_to_stdout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer sheets-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "properties": {
                "title": "Roadmap"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = format!("{}/sheets/v4/spreadsheets", server.uri());

    run_get_to(
        &client,
        "spreadsheet-123".into(),
        None,
        false,
        Vec::new(),
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"properties\":{\"title\":\"Roadmap\"},\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
}

#[tokio::test]
async fn run_get_returns_clear_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/missing-spreadsheet"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = format!("{}/sheets/v4/spreadsheets", server.uri());

    let result = run_get_to(
        &client,
        "missing-spreadsheet".into(),
        None,
        false,
        Vec::new(),
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to fetch Google Sheets Spreadsheet"));
    assert!(message.contains("Google Sheets Spreadsheet was not found"));
    assert!(out.is_empty());
}
