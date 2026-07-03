use chrono::{Duration, Utc};
use url::Url;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::state::{
    load_runtime_state_from_path, resource_key, save_runtime_state_to_path, RuntimeState,
};
use crate::auth::testing::MemoryStore;
use crate::cli::{
    SheetsInsertDataOption, SheetsValueInputOption, SheetsValueRenderOption, SheetsValuesCommand,
};
use crate::sheets::{SHEETS_READONLY_SCOPE, SHEETS_SCOPE};

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

fn sheets_write_token() -> Token {
    scoped_sheets_token("sheets-write-access")
}

fn scoped_sheets_token(access_token: &str) -> Token {
    Token {
        access_token: access_token.into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![SHEETS_READONLY_SCOPE.into(), SHEETS_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &sheets_token())
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

fn write_test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &sheets_write_token())
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
        .save_token("alice@example.com", &scoped_sheets_token("alice-access"))
        .unwrap();
    store
        .save_token("bob@example.com", &scoped_sheets_token("bob-access"))
        .unwrap();
    store
        .save_token("carol@example.com", &scoped_sheets_token("carol-access"))
        .unwrap();
    store
}

fn spreadsheets_url(server: &MockServer) -> String {
    format!("{}/sheets/v4/spreadsheets", server.uri())
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

fn assert_query_value(url: &Url, name: &str, expected: &str) {
    assert_eq!(query_value(url, name).as_deref(), Some(expected));
}

fn update_values_command(
    values: impl Into<String>,
    value_input_option: SheetsValueInputOption,
) -> SheetsValuesCommand {
    SheetsValuesCommand::Update {
        spreadsheet_id: "spreadsheet-123".into(),
        range: "Sheet1!A1:B2".into(),
        values: values.into(),
        value_input_option,
    }
}

fn append_values_command(
    values: impl Into<String>,
    value_input_option: SheetsValueInputOption,
    insert_data_option: SheetsInsertDataOption,
) -> SheetsValuesCommand {
    SheetsValuesCommand::Append {
        spreadsheet_id: "spreadsheet-123".into(),
        range: "Sheet1!A:B".into(),
        values: values.into(),
        value_input_option,
        insert_data_option,
    }
}

fn batch_update_values_command(values: impl Into<String>) -> SheetsValuesCommand {
    SheetsValuesCommand::BatchUpdate {
        spreadsheet_id: "spreadsheet-123".into(),
        values: values.into(),
    }
}

fn clear_values_command() -> SheetsValuesCommand {
    SheetsValuesCommand::Clear {
        spreadsheet_id: "spreadsheet-123".into(),
        range: "Sheet1!A1:B2".into(),
    }
}

fn batch_clear_values_command() -> SheetsValuesCommand {
    SheetsValuesCommand::BatchClear {
        spreadsheet_id: "spreadsheet-123".into(),
        ranges: vec!["Sheet1!A1:B2".into(), "Summary!A:A".into()],
    }
}

fn assert_api_failure(message: &str, operation: &str, api_body: &str) {
    assert!(message.contains(operation));
    assert!(message.contains("Google Sheets API error (400 Bad Request)"));
    assert!(message.contains(api_body));
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
    let spreadsheets_url = spreadsheets_url(&server);

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
    let spreadsheets_url = spreadsheets_url(&server);

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

#[tokio::test]
async fn run_get_unified_falls_back_on_target_access_failure_and_repairs_stale_mapping() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for bob"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer carol-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "properties": { "title": "Carol" }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut state = RuntimeState::default();
    state.set_resource_account(resource_key("sheets", "spreadsheet-123"), "bob@example.com");
    save_runtime_state_to_path(&state, &state_path).unwrap();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_get_unified_to(
        &config,
        &store,
        None,
        "spreadsheet-123".into(),
        None,
        false,
        Vec::new(),
        &mut out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"properties\":{\"title\":\"Carol\"},\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("sheets", "spreadsheet-123")),
        Some("carol@example.com")
    );
}

#[tokio::test]
async fn run_get_unified_does_not_fallback_for_explicit_account_but_maps_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-456"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-456"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let spreadsheets_url = spreadsheets_url(&server);

    let mut denied_out = Vec::new();
    let denied = run_get_unified_to(
        &config,
        &store,
        Some("alice@example.com"),
        "spreadsheet-123".into(),
        None,
        false,
        Vec::new(),
        &mut denied_out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await;

    let message = format!("{:#}", denied.unwrap_err());
    assert!(message.contains("failed to fetch Google Sheets Spreadsheet"));
    assert!(message.contains("Google Sheets Spreadsheet was not found"));
    assert!(denied_out.is_empty());

    let mut mapped_out = Vec::new();
    run_get_unified_to(
        &config,
        &store,
        Some("bob@example.com"),
        "spreadsheet-456".into(),
        None,
        false,
        Vec::new(),
        &mut mapped_out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("sheets", "spreadsheet-456")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_values_get_prints_value_range_json_to_stdout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("authorization", "Bearer sheets-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "range": "Sheet1!A1:B2",
            "values": [["Name", "Score"], ["Ada", "42"]]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        SheetsValuesCommand::Get {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B2".into(),
            value_render_option: SheetsValueRenderOption::FormattedValue,
        },
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"range\":\"Sheet1!A1:B2\",\"values\":[[\"Name\",\"Score\"],[\"Ada\",\"42\"]]}\n"
    );
}

#[tokio::test]
async fn run_values_batch_get_prints_batch_response_json_to_stdout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(header("authorization", "Bearer sheets-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "valueRanges": [
                {
                    "range": "Sheet1!A1:B2",
                    "values": [["Name", "Score"], ["Ada", "=40+2"]]
                },
                {
                    "range": "Summary!A:A",
                    "values": [["Total"], ["=SUM(Sheet1!B:B)"]]
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        SheetsValuesCommand::BatchGet {
            spreadsheet_id: "spreadsheet-123".into(),
            ranges: vec!["Sheet1!A1:B2".into(), "Summary!A:A".into()],
            value_render_option: SheetsValueRenderOption::Formula,
        },
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            "{\"spreadsheetId\":\"spreadsheet-123\",",
            "\"valueRanges\":[",
            "{\"range\":\"Sheet1!A1:B2\",\"values\":[[\"Name\",\"Score\"],[\"Ada\",\"=40+2\"]]},",
            "{\"range\":\"Summary!A:A\",\"values\":[[\"Total\"],[\"=SUM(Sheet1!B:B)\"]]}",
            "]}\n"
        )
    );

    let url = received_url(&server).await;
    assert!(url
        .path()
        .ends_with("/spreadsheets/spreadsheet-123/values/:batchGet"));
    assert_eq!(
        query_values(&url, "ranges"),
        vec!["Sheet1!A1:B2".to_string(), "Summary!A:A".to_string()]
    );
    assert_eq!(
        query_value(&url, "valueRenderOption").as_deref(),
        Some("FORMULA")
    );
}

#[tokio::test]
async fn run_values_get_unified_uses_fallback_and_updates_mapping() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B2",
        ))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B2",
        ))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "range": "Sheet1!A1:B2",
            "values": [["Name", "Score"]]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_unified_to(
        &config,
        &store,
        None,
        SheetsValuesCommand::Get {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B2".into(),
            value_render_option: SheetsValueRenderOption::FormattedValue,
        },
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"range\":\"Sheet1!A1:B2\",\"values\":[[\"Name\",\"Score\"]]}\n"
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("sheets", "spreadsheet-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_values_write_commands_use_unified_mapping() {
    let server = MockServer::start().await;
    let update_body = serde_json::json!({ "values": [["Ada", 42]] });
    let batch_update_body = serde_json::json!({
        "valueInputOption": "RAW",
        "data": [{ "range": "Sheet1!A2", "values": [["Grace"]] }]
    });
    let append_body = serde_json::json!({ "values": [["Linus"]] });

    Mock::given(method("PUT"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B1",
        ))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B1",
        ))
        .and(header("authorization", "Bearer bob-access"))
        .and(body_json(&update_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "updatedCells": 2
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/:batchUpdate",
        ))
        .and(header("authorization", "Bearer bob-access"))
        .and(body_json(&batch_update_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "totalUpdatedCells": 1
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A:A:append",
        ))
        .and(header("authorization", "Bearer bob-access"))
        .and(body_json(&append_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "updates": { "updatedRows": 1 }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B1:clear",
        ))
        .and(header("authorization", "Bearer bob-access"))
        .and(body_json(&serde_json::json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "clearedRange": "Sheet1!A1:B1"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/:batchClear",
        ))
        .and(header("authorization", "Bearer bob-access"))
        .and(body_json(&serde_json::json!({
            "ranges": ["Sheet1!A2:A2", "Sheet1!B2:B2"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "clearedRanges": ["Sheet1!A2:A2", "Sheet1!B2:B2"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let spreadsheets_url = spreadsheets_url(&server);

    let mut update_input = std::io::Cursor::new(update_body.to_string());
    let mut update_out = Vec::new();
    run_values_unified_to(
        &config,
        &store,
        None,
        SheetsValuesCommand::Update {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B1".into(),
            values: "-".into(),
            value_input_option: SheetsValueInputOption::Raw,
        },
        &mut update_input,
        &mut update_out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let mut batch_update_input = std::io::Cursor::new(batch_update_body.to_string());
    let mut batch_update_out = Vec::new();
    run_values_unified_to(
        &config,
        &store,
        None,
        batch_update_values_command("-"),
        &mut batch_update_input,
        &mut batch_update_out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let mut append_input = std::io::Cursor::new(append_body.to_string());
    let mut append_out = Vec::new();
    run_values_unified_to(
        &config,
        &store,
        None,
        SheetsValuesCommand::Append {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A:A".into(),
            values: "-".into(),
            value_input_option: SheetsValueInputOption::UserEntered,
            insert_data_option: SheetsInsertDataOption::InsertRows,
        },
        &mut append_input,
        &mut append_out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let mut clear_input = std::io::empty();
    let mut clear_out = Vec::new();
    run_values_unified_to(
        &config,
        &store,
        None,
        SheetsValuesCommand::Clear {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B1".into(),
        },
        &mut clear_input,
        &mut clear_out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let mut batch_clear_input = std::io::empty();
    let mut batch_clear_out = Vec::new();
    run_values_unified_to(
        &config,
        &store,
        None,
        SheetsValuesCommand::BatchClear {
            spreadsheet_id: "spreadsheet-123".into(),
            ranges: vec!["Sheet1!A2:A2".into(), "Sheet1!B2:B2".into()],
        },
        &mut batch_clear_input,
        &mut batch_clear_out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(update_out).unwrap(),
        "{\"updatedCells\":2}\n"
    );
    assert_eq!(
        String::from_utf8(batch_update_out).unwrap(),
        "{\"totalUpdatedCells\":1}\n"
    );
    assert_eq!(
        String::from_utf8(append_out).unwrap(),
        "{\"updates\":{\"updatedRows\":1}}\n"
    );
    assert_eq!(
        String::from_utf8(clear_out).unwrap(),
        "{\"clearedRange\":\"Sheet1!A1:B1\"}\n"
    );
    assert_eq!(
        String::from_utf8(batch_clear_out).unwrap(),
        concat!(
            "{\"clearedRanges\":[\"Sheet1!A2:A2\",\"Sheet1!B2:B2\"],",
            "\"spreadsheetId\":\"spreadsheet-123\"}\n"
        )
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("sheets", "spreadsheet-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_values_update_reads_values_from_file_and_prints_response_json() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "range": "DoesNotNeedToMatch!A1:B2",
        "values": [["Ada", 42]]
    });
    Mock::given(method("PUT"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "updatedRange": "Sheet1!A1:B2"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let values_path = temp_dir.path().join("values.json");
    std::fs::write(&values_path, request_body.to_string()).unwrap();
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        update_values_command(
            values_path.to_string_lossy().into_owned(),
            SheetsValueInputOption::UserEntered,
        ),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"spreadsheetId\":\"spreadsheet-123\",\"updatedRange\":\"Sheet1!A1:B2\"}\n"
    );

    let url = received_url(&server).await;
    assert!(url
        .path()
        .ends_with("/spreadsheets/spreadsheet-123/values/Sheet1!A1:B2"));
    assert_query_value(&url, "valueInputOption", "USER_ENTERED");
}

#[tokio::test]
async fn run_values_update_reads_values_from_stdin() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "values": [["Ada", 42]]
    });
    Mock::given(method("PUT"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "updatedCells": 2
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(request_body.to_string());
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        update_values_command("-", SheetsValueInputOption::Raw),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(String::from_utf8(out).unwrap(), "{\"updatedCells\":2}\n");
}

#[tokio::test]
async fn run_values_batch_update_reads_values_from_file_and_passes_full_body_through() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "valueInputOption": "RAW",
        "data": [
            {
                "range": "Sheet1!A1:B2",
                "majorDimension": "ROWS",
                "values": [["Ada", 42]]
            },
            {
                "range": "Summary!A1",
                "values": [["done"]]
            }
        ],
        "includeValuesInResponse": true,
        "responseValueRenderOption": "UNFORMATTED_VALUE"
    });
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/:batchUpdate",
        ))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "totalUpdatedCells": 3,
            "responses": [
                {
                    "updatedRange": "Sheet1!A1:B2"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let values_path = temp_dir.path().join("batch-values.json");
    std::fs::write(&values_path, request_body.to_string()).unwrap();
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        batch_update_values_command(values_path.to_string_lossy().into_owned()),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            "{\"responses\":[{\"updatedRange\":\"Sheet1!A1:B2\"}],",
            "\"spreadsheetId\":\"spreadsheet-123\",\"totalUpdatedCells\":3}\n"
        )
    );
}

#[tokio::test]
async fn run_values_batch_update_reads_values_from_stdin() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "valueInputOption": "USER_ENTERED",
        "data": [
            {
                "range": "Sheet1!A1",
                "values": [["=40+2"]]
            }
        ]
    });
    Mock::given(method("POST"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "totalUpdatedCells": 1
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(request_body.to_string());
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        batch_update_values_command("-"),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"totalUpdatedCells\":1}\n"
    );
}

#[tokio::test]
async fn run_values_append_reads_values_from_file_and_prints_response_json() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "range": "DoesNotNeedToMatch!A:B",
        "values": [["Grace", 99]]
    });
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "tableRange": "Sheet1!A1:B2",
            "updates": {
                "updatedRange": "Sheet1!A3:B3"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let values_path = temp_dir.path().join("append-values.json");
    std::fs::write(&values_path, request_body.to_string()).unwrap();
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        append_values_command(
            values_path.to_string_lossy().into_owned(),
            SheetsValueInputOption::UserEntered,
            SheetsInsertDataOption::InsertRows,
        ),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            "{\"spreadsheetId\":\"spreadsheet-123\",",
            "\"tableRange\":\"Sheet1!A1:B2\",",
            "\"updates\":{\"updatedRange\":\"Sheet1!A3:B3\"}}\n"
        )
    );

    let url = received_url(&server).await;
    assert!(url
        .path()
        .ends_with("/spreadsheets/spreadsheet-123/values/Sheet1!A:B:append"));
    assert_query_value(&url, "valueInputOption", "USER_ENTERED");
    assert_query_value(&url, "insertDataOption", "INSERT_ROWS");
}

#[tokio::test]
async fn run_values_append_reads_values_from_stdin() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "values": [["Grace", 99]]
    });
    Mock::given(method("POST"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "updates": {
                "updatedRows": 1
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(request_body.to_string());
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        append_values_command(
            "-",
            SheetsValueInputOption::Raw,
            SheetsInsertDataOption::Overwrite,
        ),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"updates\":{\"updatedRows\":1}}\n"
    );

    let url = received_url(&server).await;
    assert_query_value(&url, "valueInputOption", "RAW");
    assert_query_value(&url, "insertDataOption", "OVERWRITE");
}

#[tokio::test]
async fn run_values_batch_clear_prints_response_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/:batchClear",
        ))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&serde_json::json!({
            "ranges": ["Sheet1!A1:B2", "Summary!A:A"]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "clearedRanges": ["Sheet1!A1:B2", "Summary!A:A"]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        batch_clear_values_command(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"clearedRanges\":[\"Sheet1!A1:B2\",\"Summary!A:A\"]}\n"
    );
}

#[tokio::test]
async fn run_values_clear_prints_response_json() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B2:clear",
        ))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&serde_json::json!({})))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "clearedRange": "Sheet1!A1:B2"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        clear_values_command(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"clearedRange\":\"Sheet1!A1:B2\",\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
}

#[tokio::test]
async fn run_batch_update_reads_requests_from_stdin() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addSheet": {
                    "properties": {
                        "title": "New sheet"
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(request_body.to_string());
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_batch_update_to(
        &client,
        "spreadsheet-123".into(),
        "-".into(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"replies\":[{}],\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
}

#[tokio::test]
async fn run_batch_update_reads_requests_from_file_and_passes_full_body_through() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 0,
                        "title": "Renamed"
                    },
                    "fields": "title"
                }
            }
        ],
        "includeSpreadsheetInResponse": true,
        "responseRanges": ["Renamed!A1:B2"],
        "responseIncludeGridData": false
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [{}],
            "updatedSpreadsheet": {
                "spreadsheetId": "spreadsheet-123"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let requests_path = temp_dir.path().join("batch-update.json");
    std::fs::write(&requests_path, request_body.to_string()).unwrap();

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_batch_update_to(
        &client,
        "spreadsheet-123".into(),
        requests_path.to_string_lossy().into_owned(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            "{\"replies\":[{}],\"spreadsheetId\":\"spreadsheet-123\",",
            "\"updatedSpreadsheet\":{\"spreadsheetId\":\"spreadsheet-123\"}}\n"
        )
    );
}

#[tokio::test]
async fn run_batch_update_unified_uses_fallback_and_mapping_for_structural_writes() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addSheet": {
                    "properties": {
                        "title": "Issue54"
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer bob-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let requests_path = temp_dir.path().join("batch-update.json");
    std::fs::write(&requests_path, request_body.to_string()).unwrap();
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_batch_update_unified_to(
        &config,
        &store,
        None,
        "spreadsheet-123".into(),
        requests_path.to_string_lossy().into_owned(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"replies\":[{}],\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("sheets", "spreadsheet-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_batch_update_returns_clear_error_for_invalid_request_json() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new("{not json");
    let mut out = Vec::new();

    let result = run_batch_update_to(
        &client,
        "spreadsheet-123".into(),
        "-".into(),
        &mut input,
        &mut out,
        Some("https://example.test/sheets/v4/spreadsheets"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Sheets Batch Update request body from stdin"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_batch_update_returns_clear_error_for_invalid_request_json_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let requests_path = temp_dir.path().join("invalid-batch-update.json");
    std::fs::write(&requests_path, "{not json").unwrap();
    let requests_path_arg = requests_path.to_string_lossy().into_owned();

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();

    let result = run_batch_update_to(
        &client,
        "spreadsheet-123".into(),
        requests_path_arg.clone(),
        &mut input,
        &mut out,
        Some("https://example.test/sheets/v4/spreadsheets"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Sheets Batch Update request body from"));
    assert!(message.contains(&requests_path_arg));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_batch_update_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad batch update request"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(serde_json::json!({ "requests": [] }).to_string());
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    let result = run_batch_update_to(
        &client,
        "spreadsheet-123".into(),
        "-".into(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to apply Google Sheets Batch Update"));
    assert!(message.contains("Google Sheets API error (400 Bad Request)"));
    assert!(message.contains("bad batch update request"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_update_returns_clear_error_for_invalid_request_json() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new("{not json");
    let mut out = Vec::new();

    let result = run_values_to(
        &client,
        update_values_command("-", SheetsValueInputOption::UserEntered),
        &mut input,
        &mut out,
        Some("https://example.test/sheets/v4/spreadsheets"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Sheets Values request body from stdin"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_append_returns_clear_error_for_invalid_request_json() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new("{not json");
    let mut out = Vec::new();

    let result = run_values_to(
        &client,
        append_values_command(
            "-",
            SheetsValueInputOption::UserEntered,
            SheetsInsertDataOption::InsertRows,
        ),
        &mut input,
        &mut out,
        Some("https://example.test/sheets/v4/spreadsheets"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Sheets Values request body from stdin"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_batch_update_returns_clear_error_for_invalid_request_json() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new("{not json");
    let mut out = Vec::new();

    let result = run_values_to(
        &client,
        batch_update_values_command("-"),
        &mut input,
        &mut out,
        Some("https://example.test/sheets/v4/spreadsheets"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Sheets Values request body from stdin"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_batch_update_returns_clear_error_for_invalid_request_json_file() {
    let temp_dir = tempfile::tempdir().unwrap();
    let values_path = temp_dir.path().join("batch-values.json");
    std::fs::write(&values_path, "{not json").unwrap();
    let values_path_arg = values_path.to_string_lossy().into_owned();
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();

    let result = run_values_to(
        &client,
        batch_update_values_command(values_path_arg.clone()),
        &mut input,
        &mut out,
        Some("https://example.test/sheets/v4/spreadsheets"),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to parse Google Sheets Values request body from"));
    assert!(message.contains(&values_path_arg));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_batch_update_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/:batchUpdate",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad batch value update request"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(
        serde_json::json!({
            "valueInputOption": "RAW",
            "data": []
        })
        .to_string(),
    );
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    let result = run_values_to(
        &client,
        batch_update_values_command("-"),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert_api_failure(
        &message,
        "failed to batch update Google Sheets values",
        "bad batch value update request",
    );
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_update_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad update request"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(serde_json::json!({ "values": [["Ada"]] }).to_string());
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    let result = run_values_to(
        &client,
        update_values_command("-", SheetsValueInputOption::UserEntered),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to update Google Sheets values"));
    assert!(message.contains("Google Sheets API error (400 Bad Request)"));
    assert!(message.contains("bad update request"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_append_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad append request"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::Cursor::new(serde_json::json!({ "values": [["Ada"]] }).to_string());
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    let result = run_values_to(
        &client,
        append_values_command(
            "-",
            SheetsValueInputOption::UserEntered,
            SheetsInsertDataOption::InsertRows,
        ),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to append Google Sheets values"));
    assert!(message.contains("Google Sheets API error (400 Bad Request)"));
    assert!(message.contains("bad append request"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_clear_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B2:clear",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad clear request"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    let result = run_values_to(
        &client,
        clear_values_command(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert_api_failure(
        &message,
        "failed to clear Google Sheets values",
        "bad clear request",
    );
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_batch_clear_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/:batchClear",
        ))
        .respond_with(ResponseTemplate::new(400).set_body_string("bad batch clear request"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    let result = run_values_to(
        &client,
        batch_clear_values_command(),
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert_api_failure(
        &message,
        "failed to batch clear Google Sheets values",
        "bad batch clear request",
    );
    assert!(out.is_empty());
}
