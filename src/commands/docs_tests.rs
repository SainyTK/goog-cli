use chrono::{Duration, Utc};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
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
    Token {
        access_token: "docs-write-access".into(),
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
        .respond_with(ResponseTemplate::new(200).set_body_json(long_document_with_toc_and_objects()))
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
    assert_eq!(output["entries"][0]["location"]["confidence"], "table-of-contents");
    assert_eq!(output["entries"][0]["location"]["page"], 3);
    assert_eq!(output["entries"][0]["preview"], "วิธีใช้งาน");
    assert_eq!(output["entries"][1]["location"]["confidence"], "unknown");
    assert!(output["entries"][1]["location"]["page"].is_null());
    assert_eq!(output["entries"][2]["kind"], "table");
    assert_eq!(output["entries"][2]["preview"], "หัวข้อ | สถานะ");
    assert_eq!(output["entries"][3]["kind"], "inline-image");
    assert_eq!(output["entries"][4]["kind"], "positioned-image");
    assert_eq!(output["entries"][5]["location"]["confidence"], "explicit-page-break");
    assert_eq!(output["entries"][5]["location"]["page"], 2);
    assert_eq!(output["entries"][5]["location"]["contentLine"], 1);
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
