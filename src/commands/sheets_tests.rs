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
    SheetsBorderEdge, SheetsBorderStyle, SheetsConditionalFormatCondition, SheetsDimension,
    SheetsHorizontalAlignment, SheetsInsertDataOption, SheetsMergeType, SheetsNumberFormatType,
    SheetsPasteOrientation, SheetsPasteType, SheetsSheetCommand, SheetsSortOrder,
    SheetsTextDirection, SheetsValueInputOption, SheetsValueRenderOption, SheetsValuesCommand,
    SheetsVerticalAlignment, SheetsWrapStrategy,
};
use crate::sheets::SHEETS_SCOPE;

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
        scopes: vec![SHEETS_SCOPE.into()],
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
        scopes: vec![SHEETS_SCOPE.into()],
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
async fn run_create_prints_spreadsheet_id_and_edit_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets"))
        .and(header("authorization", "Bearer sheets-access"))
        .and(body_json(serde_json::json!({
            "properties": { "title": "goog-e2e-scratch" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-456",
            "properties": {
                "title": "goog-e2e-scratch"
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_create_to(
        &client,
        "goog-e2e-scratch".into(),
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "spreadsheet-456\thttps://docs.google.com/spreadsheets/d/spreadsheet-456/edit\n"
    );
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
async fn run_values_update_table_builds_value_range_from_tsv_file() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "majorDimension": "ROWS",
        "values": [
            ["Name", "Score"],
            ["Grace", "99"],
            ["Ada", "100"]
        ]
    });
    Mock::given(method("PUT"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A1:B3",
        ))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "updatedRange": "Sheet1!A1:B3",
            "updatedRows": 3
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("rows.tsv");
    std::fs::write(&data_path, "Name\tScore\nGrace\t99\nAda\t100\n").unwrap();

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        SheetsValuesCommand::UpdateTable {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B3".into(),
            data: data_path.to_string_lossy().into_owned(),
            value_input_option: SheetsValueInputOption::Raw,
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
            "\"updatedRange\":\"Sheet1!A1:B3\",\"updatedRows\":3}\n"
        )
    );

    let url = received_url(&server).await;
    assert_query_value(&url, "valueInputOption", "RAW");
}

#[tokio::test]
async fn run_values_update_table_rejects_ragged_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("rows.csv");
    std::fs::write(&data_path, "Name,Score\nGrace\n").unwrap();

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();

    let err = run_values_to(
        &client,
        SheetsValuesCommand::UpdateTable {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B3".into(),
            data: data_path.to_string_lossy().into_owned(),
            value_input_option: SheetsValueInputOption::Raw,
        },
        &mut input,
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("Google Sheets table data must be rectangular"));
}

#[tokio::test]
async fn run_values_update_row_builds_value_range_from_flags() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "majorDimension": "ROWS",
        "values": [["Grace", "99", "=SUM(B2:B4)"]]
    });
    Mock::given(method("PUT"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A2:C2",
        ))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "updatedRange": "Sheet1!A2:C2",
            "updatedRows": 1
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
        SheetsValuesCommand::UpdateRow {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A2:C2".into(),
            values: vec!["Grace".into(), "99".into(), "=SUM(B2:B4)".into()],
            value_input_option: SheetsValueInputOption::Raw,
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
            "\"updatedRange\":\"Sheet1!A2:C2\",\"updatedRows\":1}\n"
        )
    );

    let url = received_url(&server).await;
    assert_query_value(&url, "valueInputOption", "RAW");
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
async fn run_values_append_row_builds_value_range_from_flags() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "majorDimension": "ROWS",
        "values": [["Grace", "99", "=SUM(B2:B4)"]]
    });
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A:C:append",
        ))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "updates": {
                "updatedRange": "Sheet1!A5:C5",
                "updatedRows": 1
            }
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
        SheetsValuesCommand::AppendRow {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A:C".into(),
            values: vec!["Grace".into(), "99".into(), "=SUM(B2:B4)".into()],
            value_input_option: SheetsValueInputOption::UserEntered,
            insert_data_option: SheetsInsertDataOption::InsertRows,
        },
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"updates\":{\"updatedRange\":\"Sheet1!A5:C5\",\"updatedRows\":1}}\n"
    );

    let url = received_url(&server).await;
    assert_query_value(&url, "valueInputOption", "USER_ENTERED");
    assert_query_value(&url, "insertDataOption", "INSERT_ROWS");
}

#[tokio::test]
async fn run_values_append_table_builds_value_range_from_csv_file() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "majorDimension": "ROWS",
        "values": [
            ["Name", "Score"],
            ["Grace", "99"],
            ["Ada", "100"]
        ]
    });
    Mock::given(method("POST"))
        .and(path(
            "/sheets/v4/spreadsheets/spreadsheet-123/values/Sheet1!A:B:append",
        ))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "updates": {
                "updatedRange": "Sheet1!A1:B3",
                "updatedRows": 3
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("rows.csv");
    std::fs::write(&data_path, "Name,Score\nGrace,99\nAda,100\n").unwrap();

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_values_to(
        &client,
        SheetsValuesCommand::AppendTable {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A:B".into(),
            data: data_path.to_string_lossy().into_owned(),
            value_input_option: SheetsValueInputOption::Raw,
            insert_data_option: SheetsInsertDataOption::Overwrite,
        },
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"updates\":{\"updatedRange\":\"Sheet1!A1:B3\",\"updatedRows\":3}}\n"
    );

    let url = received_url(&server).await;
    assert_query_value(&url, "valueInputOption", "RAW");
    assert_query_value(&url, "insertDataOption", "OVERWRITE");
}

#[tokio::test]
async fn run_values_append_table_rejects_ragged_data() {
    let temp_dir = tempfile::tempdir().unwrap();
    let data_path = temp_dir.path().join("rows.csv");
    std::fs::write(&data_path, "Name,Score\nGrace\n").unwrap();

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut input = std::io::empty();
    let mut out = Vec::new();

    let err = run_values_to(
        &client,
        SheetsValuesCommand::AppendTable {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A:B".into(),
            data: data_path.to_string_lossy().into_owned(),
            value_input_option: SheetsValueInputOption::Raw,
            insert_data_option: SheetsInsertDataOption::Overwrite,
        },
        &mut input,
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("Google Sheets table data must be rectangular"));
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
async fn run_sheet_add_builds_add_sheet_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addSheet": {
                    "properties": {
                        "title": "Planning",
                        "sheetId": 42,
                        "index": 1
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [
                {
                    "addSheet": {
                        "properties": {
                            "sheetId": 42,
                            "title": "Planning"
                        }
                    }
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Add {
            spreadsheet_id: "spreadsheet-123".into(),
            title: "Planning".into(),
            sheet_id: Some(42),
            index: Some(1),
        },
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            "{\"replies\":[{\"addSheet\":{\"properties\":{\"sheetId\":42,",
            "\"title\":\"Planning\"}}}],\"spreadsheetId\":\"spreadsheet-123\"}\n"
        )
    );
}

#[tokio::test]
async fn run_sheet_delete_builds_delete_sheet_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteSheet": {
                    "sheetId": 42
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Delete {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
        },
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
async fn run_sheet_rename_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42,
                        "title": "Archive"
                    },
                    "fields": "title"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Rename {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            title: "Archive".into(),
        },
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
async fn run_sheet_move_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42,
                        "index": 3
                    },
                    "fields": "index"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Move {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            index: 3,
        },
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
async fn run_sheet_duplicate_builds_duplicate_sheet_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "duplicateSheet": {
                    "sourceSheetId": 42,
                    "newSheetName": "Planning Copy",
                    "newSheetId": 43,
                    "insertSheetIndex": 2
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [
                {
                    "duplicateSheet": {
                        "properties": {
                            "sheetId": 43,
                            "title": "Planning Copy"
                        }
                    }
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Duplicate {
            spreadsheet_id: "spreadsheet-123".into(),
            source_sheet_id: 42,
            title: "Planning Copy".into(),
            sheet_id: Some(43),
            index: Some(2),
        },
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        concat!(
            "{\"replies\":[{\"duplicateSheet\":{\"properties\":{\"sheetId\":43,",
            "\"title\":\"Planning Copy\"}}}],\"spreadsheetId\":\"spreadsheet-123\"}\n"
        )
    );
}

#[tokio::test]
async fn run_sheet_freeze_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42,
                        "gridProperties": {
                            "frozenRowCount": 1,
                            "frozenColumnCount": 2
                        }
                    },
                    "fields": "gridProperties.frozenRowCount,gridProperties.frozenColumnCount"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Freeze {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            rows: Some(1),
            columns: Some(2),
        },
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
async fn run_sheet_resize_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42,
                        "gridProperties": {
                            "rowCount": 200,
                            "columnCount": 12
                        }
                    },
                    "fields": "gridProperties.rowCount,gridProperties.columnCount"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Resize {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            rows: Some(200),
            columns: Some(12),
        },
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
async fn run_sheet_auto_resize_builds_auto_resize_dimensions_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "autoResizeDimensions": {
                    "dimensions": {
                        "sheetId": 42,
                        "dimension": "COLUMNS",
                        "startIndex": 0,
                        "endIndex": 5
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::AutoResize {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Columns,
            start_index: 0,
            end_index: 5,
        },
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
async fn run_sheet_auto_resize_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::AutoResize {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 5,
            end_index: 5,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_sheet_set_dimension_size_builds_update_dimension_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateDimensionProperties": {
                    "range": {
                        "sheetId": 42,
                        "dimension": "ROWS",
                        "startIndex": 1,
                        "endIndex": 3
                    },
                    "properties": {
                        "pixelSize": 28
                    },
                    "fields": "pixelSize"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::SetDimensionSize {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 1,
            end_index: 3,
            pixel_size: 28,
        },
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
async fn run_sheet_set_dimension_size_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::SetDimensionSize {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Columns,
            start_index: 5,
            end_index: 5,
            pixel_size: 80,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_sheet_hide_dimension_builds_update_dimension_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateDimensionProperties": {
                    "range": {
                        "sheetId": 42,
                        "dimension": "COLUMNS",
                        "startIndex": 1,
                        "endIndex": 3
                    },
                    "properties": {
                        "hiddenByUser": true
                    },
                    "fields": "hiddenByUser"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::HideDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Columns,
            start_index: 1,
            end_index: 3,
        },
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
async fn run_sheet_unhide_dimension_builds_update_dimension_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateDimensionProperties": {
                    "range": {
                        "sheetId": 42,
                        "dimension": "ROWS",
                        "startIndex": 4,
                        "endIndex": 8
                    },
                    "properties": {
                        "hiddenByUser": false
                    },
                    "fields": "hiddenByUser"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::UnhideDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 4,
            end_index: 8,
        },
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
async fn run_sheet_hide_dimension_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::HideDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 5,
            end_index: 5,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_sheet_group_dimension_builds_add_dimension_group_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addDimensionGroup": {
                    "range": {
                        "sheetId": 42,
                        "dimension": "ROWS",
                        "startIndex": 1,
                        "endIndex": 5
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::GroupDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 1,
            end_index: 5,
        },
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
async fn run_sheet_ungroup_dimension_builds_delete_dimension_group_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteDimensionGroup": {
                    "range": {
                        "sheetId": 42,
                        "dimension": "COLUMNS",
                        "startIndex": 2,
                        "endIndex": 6
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::UngroupDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Columns,
            start_index: 2,
            end_index: 6,
        },
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
async fn run_sheet_group_dimension_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::GroupDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 5,
            end_index: 5,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_sheet_collapse_dimension_group_builds_update_dimension_group_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateDimensionGroup": {
                    "dimensionGroup": {
                        "range": {
                            "sheetId": 42,
                            "dimension": "ROWS",
                            "startIndex": 1,
                            "endIndex": 5
                        },
                        "collapsed": true
                    },
                    "fields": "collapsed"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::CollapseDimensionGroup {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 1,
            end_index: 5,
        },
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
async fn run_sheet_expand_dimension_group_builds_update_dimension_group_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateDimensionGroup": {
                    "dimensionGroup": {
                        "range": {
                            "sheetId": 42,
                            "dimension": "COLUMNS",
                            "startIndex": 2,
                            "endIndex": 6
                        },
                        "collapsed": false
                    },
                    "fields": "collapsed"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ExpandDimensionGroup {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Columns,
            start_index: 2,
            end_index: 6,
        },
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
async fn run_sheet_collapse_dimension_group_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::CollapseDimensionGroup {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 5,
            end_index: 5,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_sheet_insert_dimension_builds_insert_dimension_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "insertDimension": {
                    "range": {
                        "sheetId": 42,
                        "dimension": "ROWS",
                        "startIndex": 2,
                        "endIndex": 4
                    },
                    "inheritFromBefore": true
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::InsertDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 2,
            end_index: 4,
            inherit_from_before: true,
        },
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
async fn run_sheet_insert_dimension_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::InsertDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Columns,
            start_index: 5,
            end_index: 5,
            inherit_from_before: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_sheet_insert_dimension_rejects_inherit_from_before_at_zero() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::InsertDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 0,
            end_index: 1,
            inherit_from_before: true,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--inherit-from-before requires --start-index greater than 0"));
}

#[tokio::test]
async fn run_sheet_delete_dimension_builds_delete_dimension_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteDimension": {
                    "range": {
                        "sheetId": 42,
                        "dimension": "COLUMNS",
                        "startIndex": 3,
                        "endIndex": 6
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Columns,
            start_index: 3,
            end_index: 6,
        },
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
async fn run_sheet_delete_dimension_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteDimension {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            dimension: SheetsDimension::Rows,
            start_index: 5,
            end_index: 5,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_sheet_basic_filter_builds_set_basic_filter_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "setBasicFilter": {
                    "filter": {
                        "range": {
                            "sheetId": 42,
                            "startRowIndex": 0,
                            "endRowIndex": 100,
                            "startColumnIndex": 0,
                            "endColumnIndex": 5
                        }
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::BasicFilter {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 100,
            start_column: 0,
            end_column: 5,
        },
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
async fn run_sheet_basic_filter_rejects_empty_row_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::BasicFilter {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 5,
            end_row: 5,
            start_column: 0,
            end_column: 5,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_basic_filter_rejects_empty_column_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::BasicFilter {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 100,
            start_column: 5,
            end_column: 5,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_clear_basic_filter_builds_clear_basic_filter_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "clearBasicFilter": {
                    "sheetId": 42
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ClearBasicFilter {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
        },
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
async fn run_sheet_merge_builds_merge_cells_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "mergeCells": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 2,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "mergeType": "MERGE_ROWS"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Merge {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 2,
            start_column: 1,
            end_column: 4,
            merge_type: SheetsMergeType::Rows,
        },
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
async fn run_sheet_merge_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Merge {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 2,
            end_row: 2,
            start_column: 1,
            end_column: 4,
            merge_type: SheetsMergeType::All,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_unmerge_builds_unmerge_cells_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "unmergeCells": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 2,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Unmerge {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 2,
            start_column: 1,
            end_column: 4,
        },
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
async fn run_sheet_sort_range_builds_sort_range_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "sortRange": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 100,
                        "startColumnIndex": 0,
                        "endColumnIndex": 5
                    },
                    "sortSpecs": [
                        {
                            "dimensionIndex": 3,
                            "sortOrder": "DESCENDING"
                        }
                    ]
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::SortRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 100,
            start_column: 0,
            end_column: 5,
            sort_column: 3,
            order: SheetsSortOrder::Descending,
        },
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
async fn run_sheet_sort_range_rejects_sort_column_outside_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::SortRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 100,
            start_column: 1,
            end_column: 5,
            sort_column: 0,
            order: SheetsSortOrder::Ascending,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--sort-column must be inside the selected column range"));
}

#[tokio::test]
async fn run_sheet_delete_duplicates_builds_delete_duplicates_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteDuplicates": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 100,
                        "startColumnIndex": 0,
                        "endColumnIndex": 5
                    },
                    "comparisonColumns": [
                        {
                            "sheetId": 42,
                            "dimension": "COLUMNS",
                            "startIndex": 1,
                            "endIndex": 2
                        },
                        {
                            "sheetId": 42,
                            "dimension": "COLUMNS",
                            "startIndex": 3,
                            "endIndex": 4
                        }
                    ]
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteDuplicates {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 100,
            start_column: 0,
            end_column: 5,
            comparison_columns: vec![1, 3],
        },
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
async fn run_sheet_delete_duplicates_omits_comparison_columns_by_default() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteDuplicates": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 100,
                        "startColumnIndex": 0,
                        "endColumnIndex": 5
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteDuplicates {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 100,
            start_column: 0,
            end_column: 5,
            comparison_columns: vec![],
        },
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
async fn run_sheet_delete_duplicates_rejects_comparison_column_outside_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteDuplicates {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 100,
            start_column: 1,
            end_column: 5,
            comparison_columns: vec![0],
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--comparison-column must be inside the selected column range"));
}

#[tokio::test]
async fn run_sheet_delete_duplicates_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteDuplicates {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 1,
            start_column: 0,
            end_column: 5,
            comparison_columns: vec![],
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_find_replace_builds_all_sheets_find_replace_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "findReplace": {
                    "find": "draft",
                    "replacement": "final",
                    "matchCase": true,
                    "matchEntireCell": false,
                    "searchByRegex": false,
                    "includeFormulas": true,
                    "allSheets": true
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [
                {
                    "findReplace": {
                        "occurrencesChanged": 3
                    }
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::FindReplace {
            spreadsheet_id: "spreadsheet-123".into(),
            find: "draft".into(),
            replacement: "final".into(),
            sheet_id: None,
            match_case: true,
            match_entire_cell: false,
            search_by_regex: false,
            include_formulas: true,
        },
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"replies\":[{\"findReplace\":{\"occurrencesChanged\":3}}],\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
}

#[tokio::test]
async fn run_sheet_find_replace_builds_sheet_scoped_regex_request() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "findReplace": {
                    "find": "^Q([0-9])$",
                    "replacement": "Quarter $1",
                    "matchCase": false,
                    "matchEntireCell": true,
                    "searchByRegex": true,
                    "includeFormulas": false,
                    "sheetId": 42
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::FindReplace {
            spreadsheet_id: "spreadsheet-123".into(),
            find: "^Q([0-9])$".into(),
            replacement: "Quarter $1".into(),
            sheet_id: Some(42),
            match_case: false,
            match_entire_cell: true,
            search_by_regex: true,
            include_formulas: false,
        },
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
async fn run_sheet_find_replace_rejects_empty_find_text() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::FindReplace {
            spreadsheet_id: "spreadsheet-123".into(),
            find: "".into(),
            replacement: "new".into(),
            sheet_id: None,
            match_case: false,
            match_entire_cell: false,
            search_by_regex: false,
            include_formulas: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("find text must not be empty"));
}

#[tokio::test]
async fn run_sheet_copy_paste_builds_copy_paste_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "copyPaste": {
                    "source": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 4,
                        "startColumnIndex": 0,
                        "endColumnIndex": 3
                    },
                    "destination": {
                        "sheetId": 99,
                        "startRowIndex": 10,
                        "endRowIndex": 13,
                        "startColumnIndex": 5,
                        "endColumnIndex": 8
                    },
                    "pasteType": "PASTE_VALUES",
                    "pasteOrientation": "TRANSPOSE"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::CopyPaste {
            spreadsheet_id: "spreadsheet-123".into(),
            source_sheet_id: 42,
            source_start_row: 1,
            source_end_row: 4,
            source_start_column: 0,
            source_end_column: 3,
            destination_sheet_id: 99,
            destination_start_row: 10,
            destination_end_row: 13,
            destination_start_column: 5,
            destination_end_column: 8,
            paste_type: SheetsPasteType::Values,
            paste_orientation: SheetsPasteOrientation::Transposed,
        },
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
async fn run_sheet_copy_paste_rejects_empty_destination_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::CopyPaste {
            spreadsheet_id: "spreadsheet-123".into(),
            source_sheet_id: 42,
            source_start_row: 1,
            source_end_row: 4,
            source_start_column: 0,
            source_end_column: 3,
            destination_sheet_id: 99,
            destination_start_row: 10,
            destination_end_row: 10,
            destination_start_column: 5,
            destination_end_column: 8,
            paste_type: SheetsPasteType::Normal,
            paste_orientation: SheetsPasteOrientation::Normal,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_cut_paste_builds_cut_paste_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "cutPaste": {
                    "source": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 4,
                        "startColumnIndex": 0,
                        "endColumnIndex": 3
                    },
                    "destination": {
                        "sheetId": 99,
                        "rowIndex": 10,
                        "columnIndex": 5
                    },
                    "pasteType": "PASTE_VALUES"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::CutPaste {
            spreadsheet_id: "spreadsheet-123".into(),
            source_sheet_id: 42,
            source_start_row: 1,
            source_end_row: 4,
            source_start_column: 0,
            source_end_column: 3,
            destination_sheet_id: 99,
            destination_row: 10,
            destination_column: 5,
            paste_type: SheetsPasteType::Values,
        },
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
async fn run_sheet_cut_paste_rejects_empty_source_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::CutPaste {
            spreadsheet_id: "spreadsheet-123".into(),
            source_sheet_id: 42,
            source_start_row: 1,
            source_end_row: 1,
            source_start_column: 0,
            source_end_column: 3,
            destination_sheet_id: 99,
            destination_row: 10,
            destination_column: 5,
            paste_type: SheetsPasteType::Normal,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_background_color_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "backgroundColor": {
                                "red": 1.0,
                                "green": 0.8,
                                "blue": 0.0
                            }
                        }
                    },
                    "fields": "userEnteredFormat.backgroundColor"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::BackgroundColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            color: "#ffcc00".into(),
        },
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
async fn run_sheet_background_color_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::BackgroundColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 0,
            start_column: 1,
            end_column: 4,
            color: "#ffcc00".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_background_color_rejects_non_hex_color() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::BackgroundColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            color: "yellow".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("color must be a hex RGB value like #3366cc or 3366cc"));
}

#[tokio::test]
async fn run_sheet_text_color_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "foregroundColor": {
                                    "red": 0.2,
                                    "green": 0.4,
                                    "blue": 0.8
                                }
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.foregroundColor"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::TextColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            color: "#3366cc".into(),
        },
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
async fn run_sheet_text_color_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::TextColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 4,
            end_column: 4,
            color: "#3366cc".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_text_color_rejects_non_hex_color() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::TextColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            color: "blue".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("color must be a hex RGB value like #3366cc or 3366cc"));
}

#[tokio::test]
async fn run_sheet_font_size_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "fontSize": 14
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.fontSize"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::FontSize {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            size: 14,
        },
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
async fn run_sheet_font_size_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::FontSize {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 4,
            end_column: 4,
            size: 14,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_font_family_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "fontFamily": "Roboto"
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.fontFamily"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::FontFamily {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            family: "Roboto".into(),
        },
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
async fn run_sheet_font_family_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::FontFamily {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 4,
            end_column: 4,
            family: "Roboto".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_font_family_rejects_empty_family() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::FontFamily {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            family: " ".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("--family must not be empty"));
}

#[tokio::test]
async fn run_sheet_number_format_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 10,
                        "startColumnIndex": 2,
                        "endColumnIndex": 3
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "numberFormat": {
                                "type": "CURRENCY",
                                "pattern": "$#,##0.00"
                            }
                        }
                    },
                    "fields": "userEnteredFormat.numberFormat"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::NumberFormat {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 10,
            start_column: 2,
            end_column: 3,
            format_type: SheetsNumberFormatType::Currency,
            pattern: Some("$#,##0.00".into()),
        },
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
async fn run_sheet_number_format_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::NumberFormat {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 10,
            start_column: 3,
            end_column: 3,
            format_type: SheetsNumberFormatType::Number,
            pattern: None,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_number_format_rejects_empty_pattern() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::NumberFormat {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 10,
            start_column: 2,
            end_column: 3,
            format_type: SheetsNumberFormatType::Number,
            pattern: Some(" ".into()),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("--pattern must not be empty"));
}

#[tokio::test]
async fn run_sheet_borders_builds_update_borders_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateBorders": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 5,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "top": {
                        "style": "SOLID_THICK",
                        "color": {
                            "red": 0.2,
                            "green": 0.4,
                            "blue": 0.8
                        }
                    },
                    "bottom": {
                        "style": "SOLID_THICK",
                        "color": {
                            "red": 0.2,
                            "green": 0.4,
                            "blue": 0.8
                        }
                    },
                    "left": {
                        "style": "SOLID_THICK",
                        "color": {
                            "red": 0.2,
                            "green": 0.4,
                            "blue": 0.8
                        }
                    },
                    "right": {
                        "style": "SOLID_THICK",
                        "color": {
                            "red": 0.2,
                            "green": 0.4,
                            "blue": 0.8
                        }
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Borders {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 5,
            start_column: 1,
            end_column: 4,
            edge: vec![SheetsBorderEdge::Outer],
            style: SheetsBorderStyle::SolidThick,
            color: Some("3366cc".into()),
        },
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
async fn run_sheet_borders_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Borders {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 0,
            start_column: 1,
            end_column: 4,
            edge: vec![SheetsBorderEdge::All],
            style: SheetsBorderStyle::Solid,
            color: None,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_borders_rejects_non_hex_color() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Borders {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 5,
            start_column: 1,
            end_column: 4,
            edge: vec![SheetsBorderEdge::All],
            style: SheetsBorderStyle::Solid,
            color: Some("blue".into()),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("color must be a hex RGB value like #3366cc or 3366cc"));
}

#[tokio::test]
async fn run_sheet_clear_format_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {}
                    },
                    "fields": "userEnteredFormat"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ClearFormat {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
        },
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
async fn run_sheet_clear_format_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::ClearFormat {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 0,
            start_column: 1,
            end_column: 4,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_bold_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "bold": true
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.bold"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Bold {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: false,
        },
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
async fn run_sheet_bold_off_builds_false_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "bold": false
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.bold"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Bold {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: true,
        },
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
async fn run_sheet_bold_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Bold {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 4,
            end_column: 4,
            off: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_italic_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "italic": true
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.italic"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Italic {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: false,
        },
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
async fn run_sheet_italic_off_builds_false_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "italic": false
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.italic"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Italic {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: true,
        },
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
async fn run_sheet_italic_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Italic {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 4,
            end_column: 4,
            off: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_underline_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "underline": true
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.underline"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Underline {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: false,
        },
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
async fn run_sheet_underline_off_builds_false_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "underline": false
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.underline"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Underline {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: true,
        },
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
async fn run_sheet_underline_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Underline {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 4,
            end_column: 4,
            off: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_strikethrough_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "strikethrough": true
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.strikethrough"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Strikethrough {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: false,
        },
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
async fn run_sheet_strikethrough_off_builds_false_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textFormat": {
                                "strikethrough": false
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textFormat.strikethrough"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Strikethrough {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            off: true,
        },
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
async fn run_sheet_strikethrough_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Strikethrough {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 4,
            end_column: 4,
            off: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_horizontal_align_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "horizontalAlignment": "CENTER"
                        }
                    },
                    "fields": "userEnteredFormat.horizontalAlignment"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::HorizontalAlign {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            alignment: SheetsHorizontalAlignment::Center,
        },
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
async fn run_sheet_horizontal_align_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::HorizontalAlign {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 0,
            start_column: 1,
            end_column: 4,
            alignment: SheetsHorizontalAlignment::Left,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_vertical_align_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "verticalAlignment": "MIDDLE"
                        }
                    },
                    "fields": "userEnteredFormat.verticalAlignment"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::VerticalAlign {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            alignment: SheetsVerticalAlignment::Middle,
        },
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
async fn run_sheet_vertical_align_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::VerticalAlign {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 2,
            end_column: 2,
            alignment: SheetsVerticalAlignment::Top,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-column must be greater than --start-column"));
}

#[tokio::test]
async fn run_sheet_text_wrap_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "wrapStrategy": "WRAP"
                        }
                    },
                    "fields": "userEnteredFormat.wrapStrategy"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::TextWrap {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            strategy: SheetsWrapStrategy::Wrap,
        },
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
async fn run_sheet_text_wrap_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::TextWrap {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 3,
            end_row: 3,
            start_column: 1,
            end_column: 4,
            strategy: SheetsWrapStrategy::Clip,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_text_rotation_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textRotation": {
                                "angle": 45
                            }
                        }
                    },
                    "fields": "userEnteredFormat.textRotation"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::TextRotation {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            angle: 45,
        },
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
async fn run_sheet_text_rotation_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::TextRotation {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 3,
            end_row: 3,
            start_column: 1,
            end_column: 4,
            angle: 45,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_text_direction_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "userEnteredFormat": {
                            "textDirection": "RIGHT_TO_LEFT"
                        }
                    },
                    "fields": "userEnteredFormat.textDirection"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::TextDirection {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            direction: SheetsTextDirection::RightToLeft,
        },
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
async fn run_sheet_text_direction_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::TextDirection {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 3,
            end_row: 3,
            start_column: 1,
            end_column: 4,
            direction: SheetsTextDirection::LeftToRight,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_note_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {
                        "note": "Review this input"
                    },
                    "fields": "note"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Note {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            note: Some("Review this input".into()),
            clear: false,
        },
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
async fn run_sheet_note_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Note {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 3,
            end_row: 3,
            start_column: 1,
            end_column: 4,
            note: Some("Review this input".into()),
            clear: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_note_rejects_empty_note() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Note {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            note: Some("   ".into()),
            clear: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("note text must not be empty"));
}

#[tokio::test]
async fn run_sheet_note_clear_builds_repeat_cell_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "repeatCell": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 0,
                        "endRowIndex": 10,
                        "startColumnIndex": 1,
                        "endColumnIndex": 4
                    },
                    "cell": {},
                    "fields": "note"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Note {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 10,
            start_column: 1,
            end_column: 4,
            note: None,
            clear: true,
        },
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
async fn run_sheet_note_clear_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::Note {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 3,
            end_row: 3,
            start_column: 1,
            end_column: 4,
            note: None,
            clear: true,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_data_validation_list_builds_set_data_validation_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "setDataValidation": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 20,
                        "startColumnIndex": 3,
                        "endColumnIndex": 4
                    },
                    "rule": {
                        "condition": {
                            "type": "ONE_OF_LIST",
                            "values": [
                                { "userEnteredValue": "Open" },
                                { "userEnteredValue": "Closed" }
                            ]
                        },
                        "strict": false,
                        "showCustomUi": false,
                        "inputMessage": "Pick a status"
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DataValidationList {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            values: vec!["Open".into(), "Closed".into()],
            allow_invalid: true,
            hide_dropdown: true,
            input_message: Some("Pick a status".into()),
            clear: false,
        },
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
async fn run_sheet_data_validation_list_clear_builds_set_data_validation_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "setDataValidation": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 20,
                        "startColumnIndex": 3,
                        "endColumnIndex": 4
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DataValidationList {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            values: Vec::new(),
            allow_invalid: false,
            hide_dropdown: false,
            input_message: None,
            clear: true,
        },
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
async fn run_sheet_data_validation_list_rejects_empty_value() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::DataValidationList {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            values: vec![" ".into()],
            allow_invalid: false,
            hide_dropdown: false,
            input_message: None,
            clear: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("data validation values must not be empty"));
}

#[tokio::test]
async fn run_sheet_data_validation_checkbox_builds_set_data_validation_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "setDataValidation": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 20,
                        "startColumnIndex": 3,
                        "endColumnIndex": 4
                    },
                    "rule": {
                        "condition": {
                            "type": "BOOLEAN",
                            "values": [
                                { "userEnteredValue": "Done" },
                                { "userEnteredValue": "Todo" }
                            ]
                        },
                        "strict": false,
                        "inputMessage": "Mark complete"
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DataValidationCheckbox {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            checked_value: Some("Done".into()),
            unchecked_value: Some("Todo".into()),
            allow_invalid: true,
            input_message: Some("Mark complete".into()),
            clear: false,
        },
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
async fn run_sheet_data_validation_checkbox_builds_default_boolean_rule() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "setDataValidation": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 20,
                        "startColumnIndex": 3,
                        "endColumnIndex": 4
                    },
                    "rule": {
                        "condition": {
                            "type": "BOOLEAN"
                        },
                        "strict": true
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DataValidationCheckbox {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            checked_value: None,
            unchecked_value: None,
            allow_invalid: false,
            input_message: None,
            clear: false,
        },
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
async fn run_sheet_data_validation_checkbox_clear_builds_set_data_validation_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "setDataValidation": {
                    "range": {
                        "sheetId": 42,
                        "startRowIndex": 1,
                        "endRowIndex": 20,
                        "startColumnIndex": 3,
                        "endColumnIndex": 4
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DataValidationCheckbox {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            checked_value: None,
            unchecked_value: None,
            allow_invalid: false,
            input_message: None,
            clear: true,
        },
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
async fn run_sheet_data_validation_checkbox_rejects_empty_checked_value() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::DataValidationCheckbox {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            checked_value: Some(" ".into()),
            unchecked_value: None,
            allow_invalid: false,
            input_message: None,
            clear: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--checked-value must not be empty"));
}

#[tokio::test]
async fn run_sheet_conditional_format_color_builds_add_rule_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addConditionalFormatRule": {
                    "rule": {
                        "ranges": [
                            {
                                "sheetId": 42,
                                "startRowIndex": 1,
                                "endRowIndex": 20,
                                "startColumnIndex": 3,
                                "endColumnIndex": 4
                            }
                        ],
                        "booleanRule": {
                            "condition": {
                                "type": "NUMBER_GREATER",
                                "values": [
                                    {
                                        "userEnteredValue": "100"
                                    }
                                ]
                            },
                            "format": {
                                "backgroundColor": {
                                    "red": 1.0,
                                    "green": 0.8,
                                    "blue": 0.8
                                },
                                "textFormat": {
                                    "foregroundColor": {
                                        "red": 0.6,
                                        "green": 0.0,
                                        "blue": 0.0
                                    }
                                }
                            }
                        }
                    },
                    "index": 2
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ConditionalFormatColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            condition: SheetsConditionalFormatCondition::NumberGreater,
            value: "100".into(),
            background_color: Some("#ffcccc".into()),
            text_color: Some("990000".into()),
            index: 2,
        },
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
async fn run_sheet_conditional_format_color_rejects_missing_colors() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::ConditionalFormatColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            condition: SheetsConditionalFormatCondition::TextContains,
            value: "Blocked".into(),
            background_color: None,
            text_color: None,
            index: 0,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("at least one of --background-color or --text-color is required"));
}

#[tokio::test]
async fn run_sheet_conditional_format_color_rejects_empty_value() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::ConditionalFormatColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            condition: SheetsConditionalFormatCondition::CustomFormula,
            value: " ".into(),
            background_color: Some("#ffcccc".into()),
            text_color: None,
            index: 0,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("--value must not be empty"));
}

#[tokio::test]
async fn run_sheet_conditional_format_update_builds_replace_rule_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateConditionalFormatRule": {
                    "sheetId": 42,
                    "index": 3,
                    "rule": {
                        "ranges": [
                            {
                                "sheetId": 42,
                                "startRowIndex": 1,
                                "endRowIndex": 20,
                                "startColumnIndex": 3,
                                "endColumnIndex": 4
                            }
                        ],
                        "booleanRule": {
                            "condition": {
                                "type": "TEXT_CONTAINS",
                                "values": [
                                    {
                                        "userEnteredValue": "Blocked"
                                    }
                                ]
                            },
                            "format": {
                                "backgroundColor": {
                                    "red": 1.0,
                                    "green": 0.9333333333333333,
                                    "blue": 0.9333333333333333
                                }
                            }
                        }
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ConditionalFormatUpdate {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            index: 3,
            start_row: 1,
            end_row: 20,
            start_column: 3,
            end_column: 4,
            condition: SheetsConditionalFormatCondition::TextContains,
            value: "Blocked".into(),
            background_color: Some("#ffeeee".into()),
            text_color: None,
        },
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
async fn run_sheet_conditional_format_delete_builds_delete_rule_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteConditionalFormatRule": {
                    "sheetId": 42,
                    "index": 3
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ConditionalFormatDelete {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            index: 3,
        },
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
async fn run_sheet_conditional_format_move_builds_update_rule_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateConditionalFormatRule": {
                    "sheetId": 42,
                    "index": 3,
                    "newIndex": 0
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ConditionalFormatMove {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            index: 3,
            new_index: 0,
        },
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
async fn run_sheet_protect_range_builds_add_protected_range_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addProtectedRange": {
                    "protectedRange": {
                        "range": {
                            "sheetId": 42,
                            "startRowIndex": 0,
                            "endRowIndex": 1,
                            "startColumnIndex": 0,
                            "endColumnIndex": 5
                        },
                        "warningOnly": false,
                        "description": "Lock headers"
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [
                {
                    "addProtectedRange": {
                        "protectedRange": {
                            "protectedRangeId": 7
                        }
                    }
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ProtectRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 1,
            start_column: 0,
            end_column: 5,
            description: Some("Lock headers".into()),
            warning_only: false,
        },
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"replies\":[{\"addProtectedRange\":{\"protectedRange\":{\"protectedRangeId\":7}}}],\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
}

#[tokio::test]
async fn run_sheet_protect_range_builds_warning_only_request() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addProtectedRange": {
                    "protectedRange": {
                        "range": {
                            "sheetId": 42,
                            "startRowIndex": 1,
                            "endRowIndex": 10,
                            "startColumnIndex": 0,
                            "endColumnIndex": 3
                        },
                        "warningOnly": true
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ProtectRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 10,
            start_column: 0,
            end_column: 3,
            description: None,
            warning_only: true,
        },
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
async fn run_sheet_protect_range_rejects_empty_description() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::ProtectRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 0,
            end_row: 1,
            start_column: 0,
            end_column: 5,
            description: Some("   ".into()),
            warning_only: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("--description must not be empty"));
}

#[tokio::test]
async fn run_sheet_protect_range_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::ProtectRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            start_row: 1,
            end_row: 1,
            start_column: 0,
            end_column: 5,
            description: None,
            warning_only: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_add_named_range_builds_add_named_range_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "addNamedRange": {
                    "namedRange": {
                        "name": "HeaderCells",
                        "range": {
                            "sheetId": 42,
                            "startRowIndex": 0,
                            "endRowIndex": 1,
                            "startColumnIndex": 0,
                            "endColumnIndex": 5
                        },
                        "namedRangeId": "header_cells"
                    }
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
        .and(header("authorization", "Bearer sheets-write-access"))
        .and(body_json(&request_body))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "spreadsheetId": "spreadsheet-123",
            "replies": [
                {
                    "addNamedRange": {
                        "namedRange": {
                            "namedRangeId": "header_cells"
                        }
                    }
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::AddNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            name: "HeaderCells".into(),
            start_row: 0,
            end_row: 1,
            start_column: 0,
            end_column: 5,
            named_range_id: Some("header_cells".into()),
        },
        &mut out,
        Some(&spreadsheets_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"replies\":[{\"addNamedRange\":{\"namedRange\":{\"namedRangeId\":\"header_cells\"}}}],\"spreadsheetId\":\"spreadsheet-123\"}\n"
    );
}

#[tokio::test]
async fn run_sheet_add_named_range_rejects_empty_name() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::AddNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            name: " ".into(),
            start_row: 0,
            end_row: 1,
            start_column: 0,
            end_column: 5,
            named_range_id: None,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("name must not be empty"));
}

#[tokio::test]
async fn run_sheet_add_named_range_rejects_empty_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::AddNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            name: "HeaderCells".into(),
            start_row: 0,
            end_row: 0,
            start_column: 0,
            end_column: 5,
            named_range_id: None,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-row must be greater than --start-row"));
}

#[tokio::test]
async fn run_sheet_delete_named_range_builds_delete_named_range_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteNamedRange": {
                    "namedRangeId": "header_cells"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            named_range_id: "header_cells".into(),
        },
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
async fn run_sheet_delete_named_range_rejects_empty_id() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::DeleteNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            named_range_id: " ".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("namedRangeId must not be empty"));
}

#[tokio::test]
async fn run_sheet_update_named_range_builds_update_named_range_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateNamedRange": {
                    "namedRange": {
                        "namedRangeId": "header_cells",
                        "name": "HeaderRows",
                        "range": {
                            "sheetId": 42,
                            "startRowIndex": 0,
                            "endRowIndex": 2,
                            "startColumnIndex": 0,
                            "endColumnIndex": 5
                        }
                    },
                    "fields": "name,range"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::UpdateNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            named_range_id: "header_cells".into(),
            name: Some("HeaderRows".into()),
            sheet_id: Some(42),
            start_row: Some(0),
            end_row: Some(2),
            start_column: Some(0),
            end_column: Some(5),
        },
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
async fn run_sheet_update_named_range_allows_rename_only() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateNamedRange": {
                    "namedRange": {
                        "namedRangeId": "header_cells",
                        "name": "HeaderRows"
                    },
                    "fields": "name"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::UpdateNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            named_range_id: "header_cells".into(),
            name: Some("HeaderRows".into()),
            sheet_id: None,
            start_row: None,
            end_row: None,
            start_column: None,
            end_column: None,
        },
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
async fn run_sheet_update_named_range_rejects_partial_range() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::UpdateNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            named_range_id: "header_cells".into(),
            name: None,
            sheet_id: Some(42),
            start_row: Some(0),
            end_row: Some(2),
            start_column: Some(0),
            end_column: None,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains(
        "provide --sheet-id, --start-row, --end-row, --start-column, and --end-column together"
    ));
}

#[tokio::test]
async fn run_sheet_update_named_range_rejects_empty_update() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::UpdateNamedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            named_range_id: "header_cells".into(),
            name: None,
            sheet_id: None,
            start_row: None,
            end_row: None,
            start_column: None,
            end_column: None,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("provide --name or a full range to update"));
}

#[tokio::test]
async fn run_sheet_unprotect_range_builds_delete_protected_range_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "deleteProtectedRange": {
                    "protectedRangeId": 7
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::UnprotectRange {
            spreadsheet_id: "spreadsheet-123".into(),
            protected_range_id: 7,
        },
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
async fn run_sheet_update_protected_range_builds_update_protected_range_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateProtectedRange": {
                    "protectedRange": {
                        "protectedRangeId": 7,
                        "description": "Editable warning",
                        "warningOnly": true
                    },
                    "fields": "description,warningOnly"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::UpdateProtectedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            protected_range_id: 7,
            description: Some("Editable warning".into()),
            warning_only: true,
            enforce: false,
        },
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
async fn run_sheet_update_protected_range_can_enforce_existing_range() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateProtectedRange": {
                    "protectedRange": {
                        "protectedRangeId": 7,
                        "warningOnly": false
                    },
                    "fields": "warningOnly"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::UpdateProtectedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            protected_range_id: 7,
            description: None,
            warning_only: false,
            enforce: true,
        },
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
async fn run_sheet_update_protected_range_rejects_empty_description() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::UpdateProtectedRange {
            spreadsheet_id: "spreadsheet-123".into(),
            protected_range_id: 7,
            description: Some(" ".into()),
            warning_only: false,
            enforce: false,
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("--description must not be empty"));
}

#[tokio::test]
async fn run_sheet_tab_color_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42,
                        "tabColor": {
                            "red": 0.2,
                            "green": 0.4,
                            "blue": 0.8
                        }
                    },
                    "fields": "tabColor"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::TabColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            color: "#3366cc".into(),
        },
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
async fn run_sheet_tab_color_rejects_non_hex_color() {
    let store = MemoryStore::default();
    let client = write_test_client(&store);
    let mut out = Vec::new();

    let err = run_sheet_to(
        &client,
        SheetsSheetCommand::TabColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
            color: "blue".into(),
        },
        &mut out,
        None,
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("color must be a hex RGB value like #3366cc or 3366cc"));
}

#[tokio::test]
async fn run_sheet_clear_tab_color_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42
                    },
                    "fields": "tabColor"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::ClearTabColor {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
        },
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
async fn run_sheet_hide_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42,
                        "hidden": true
                    },
                    "fields": "hidden"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Hide {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
        },
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
async fn run_sheet_unhide_builds_update_sheet_properties_batch_update() {
    let server = MockServer::start().await;
    let request_body = serde_json::json!({
        "requests": [
            {
                "updateSheetProperties": {
                    "properties": {
                        "sheetId": 42,
                        "hidden": false
                    },
                    "fields": "hidden"
                }
            }
        ]
    });
    Mock::given(method("POST"))
        .and(path("/sheets/v4/spreadsheets/spreadsheet-123:batchUpdate"))
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
    let mut out = Vec::new();
    let spreadsheets_url = spreadsheets_url(&server);

    run_sheet_to(
        &client,
        SheetsSheetCommand::Unhide {
            spreadsheet_id: "spreadsheet-123".into(),
            sheet_id: 42,
        },
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
