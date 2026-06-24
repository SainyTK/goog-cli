use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use url::Url;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::{AuthClient, AuthorizationCode, AuthorizationCodeFlow};
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::error::AuthError;
use crate::auth::testing::MemoryStore;
use crate::sheets::*;

const SPREADSHEET_RESPONSE: &str = r#"{
  "spreadsheetId": "spreadsheet-123",
  "properties": {
    "title": "Roadmap"
  },
  "sheets": [
    {
      "properties": {
        "sheetId": 0,
        "title": "Sheet1"
      }
    }
  ]
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

fn sheets_token() -> Token {
    Token {
        access_token: "sheets-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![SHEETS_READONLY_SCOPE.into()],
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
        .save_token("alice@example.com", &sheets_token())
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

fn spreadsheets_url(server: &MockServer) -> String {
    format!("{}/sheets/v4/spreadsheets", server.uri())
}

fn spreadsheet_options(server: &MockServer, spreadsheet_id: &str) -> GetSpreadsheetOptions {
    GetSpreadsheetOptions::new(spreadsheet_id).with_spreadsheets_url(spreadsheets_url(server))
}

async fn received_url(server: &MockServer) -> Url {
    server
        .received_requests()
        .await
        .unwrap()
        .into_iter()
        .next()
        .unwrap()
        .url
}

fn query_value(url: &Url, name: &str) -> Option<String> {
    url.query_pairs()
        .find(|(query_name, _)| query_name == name)
        .map(|(_, value)| value.into_owned())
}

fn query_values(url: &Url, name: &str) -> Vec<String> {
    url.query_pairs()
        .filter(|(query_name, _)| query_name == name)
        .map(|(_, value)| value.into_owned())
        .collect()
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
            code: "sheets-code".into(),
        })
    }
}

#[tokio::test]
async fn get_spreadsheet_fetches_raw_google_sheets_spreadsheet() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer sheets-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SPREADSHEET_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = spreadsheet_options(&server, "spreadsheet-123");

    let spreadsheet = get_spreadsheet(&client, &options).await.unwrap();

    assert_eq!(spreadsheet["spreadsheetId"], "spreadsheet-123");
    assert_eq!(spreadsheet["properties"]["title"], "Roadmap");
    assert!(spreadsheet["sheets"].is_array());
}

#[tokio::test]
async fn get_spreadsheet_passes_google_query_options() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer sheets-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SPREADSHEET_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = spreadsheet_options(&server, "spreadsheet-123")
        .with_fields("spreadsheetId,properties.title")
        .with_include_grid_data(true)
        .with_ranges(vec!["Sheet1!A1:B2".into(), "Summary!A:A".into()]);

    get_spreadsheet(&client, &options).await.unwrap();

    let url = received_url(&server).await;
    assert_eq!(
        query_value(&url, "fields").as_deref(),
        Some("spreadsheetId,properties.title")
    );
    assert_eq!(
        query_value(&url, "includeGridData").as_deref(),
        Some("true")
    );
    assert_eq!(
        query_values(&url, "ranges"),
        vec!["Sheet1!A1:B2".to_string(), "Summary!A:A".to_string()]
    );
}

#[tokio::test]
async fn get_spreadsheet_excludes_grid_data_by_default() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer sheets-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SPREADSHEET_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = spreadsheet_options(&server, "spreadsheet-123");

    get_spreadsheet(&client, &options).await.unwrap();

    let url = received_url(&server).await;
    assert_eq!(query_value(&url, "includeGridData"), None);
}

#[tokio::test]
async fn get_spreadsheet_requests_only_readonly_sheets_scope_when_missing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("code=sheets-code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "sheets-access",
            "expires_in": 3600,
            "scope": SHEETS_READONLY_SCOPE,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer sheets-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SPREADSHEET_RESPONSE))
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
    let options = spreadsheet_options(&server, "spreadsheet-123");

    get_spreadsheet(&client, &options).await.unwrap();

    assert_eq!(
        scopes_seen.lock().unwrap().clone(),
        vec![SHEETS_READONLY_SCOPE.to_string()]
    );
    let saved = store.load_token("alice@example.com").unwrap().unwrap();
    assert_eq!(
        saved.scopes,
        vec!["openid".to_string(), SHEETS_READONLY_SCOPE.to_string()]
    );
}

#[tokio::test]
async fn get_spreadsheet_returns_sheets_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/missing-spreadsheet"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = spreadsheet_options(&server, "missing-spreadsheet");

    let err = get_spreadsheet(&client, &options).await.unwrap_err();

    assert!(matches!(err, SheetsError::NotFound));
}

#[tokio::test]
async fn get_spreadsheet_returns_sheets_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/private-spreadsheet"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = spreadsheet_options(&server, "private-spreadsheet");

    let err = get_spreadsheet(&client, &options).await.unwrap_err();

    assert!(matches!(err, SheetsError::PermissionDenied));
}

#[tokio::test]
async fn get_spreadsheet_returns_invalid_url_error_for_malformed_api_url() {
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetSpreadsheetOptions::new("spreadsheet-123").with_spreadsheets_url("://bad-url");

    let err = get_spreadsheet(&client, &options).await.unwrap_err();

    assert!(matches!(err, SheetsError::InvalidUrl(_)));
}

#[tokio::test]
async fn get_spreadsheet_returns_invalid_response_error_for_malformed_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{not json"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = spreadsheet_options(&server, "spreadsheet-123");

    let err = get_spreadsheet(&client, &options).await.unwrap_err();

    assert!(matches!(err, SheetsError::InvalidResponse(_)));
}
