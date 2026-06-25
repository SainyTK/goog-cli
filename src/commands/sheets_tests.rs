use chrono::{Duration, Utc};
use url::Url;
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
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
    Token {
        access_token: "sheets-write-access".into(),
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
        SheetsValuesCommand::Update {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B2".into(),
            values: values_path.to_string_lossy().into_owned(),
            value_input_option: SheetsValueInputOption::UserEntered,
        },
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
        SheetsValuesCommand::Update {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B2".into(),
            values: "-".into(),
            value_input_option: SheetsValueInputOption::Raw,
        },
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

    assert_eq!(String::from_utf8(out).unwrap(), "{\"totalUpdatedCells\":1}\n");
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
        SheetsValuesCommand::BatchClear {
            spreadsheet_id: "spreadsheet-123".into(),
            ranges: vec!["Sheet1!A1:B2".into(), "Summary!A:A".into()],
        },
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
        SheetsValuesCommand::Clear {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B2".into(),
        },
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
        SheetsValuesCommand::Update {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B2".into(),
            values: "-".into(),
            value_input_option: SheetsValueInputOption::UserEntered,
        },
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
async fn run_values_batch_update_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
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
    assert!(message.contains("failed to batch update Google Sheets values"));
    assert!(message.contains("Google Sheets API error (400 Bad Request)"));
    assert!(message.contains("bad batch value update request"));
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
        SheetsValuesCommand::Clear {
            spreadsheet_id: "spreadsheet-123".into(),
            range: "Sheet1!A1:B2".into(),
        },
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to clear Google Sheets values"));
    assert!(message.contains("Google Sheets API error (400 Bad Request)"));
    assert!(message.contains("bad clear request"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_values_batch_clear_returns_clear_error_for_api_failure() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
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
        SheetsValuesCommand::BatchClear {
            spreadsheet_id: "spreadsheet-123".into(),
            ranges: vec!["Sheet1!A1:B2".into(), "Summary!A:A".into()],
        },
        &mut input,
        &mut out,
        Some(&spreadsheets_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to batch clear Google Sheets values"));
    assert!(message.contains("Google Sheets API error (400 Bad Request)"));
    assert!(message.contains("bad batch clear request"));
    assert!(out.is_empty());
}
