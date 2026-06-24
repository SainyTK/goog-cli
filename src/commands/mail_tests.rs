use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::testing::MemoryStore;
use crate::mail::GMAIL_READONLY_SCOPE;

use super::mail::*;

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

fn mail_token() -> Token {
    Token {
        access_token: "mail-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![GMAIL_READONLY_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store.save_token("alice@example.com", &mail_token()).unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

#[tokio::test]
async fn run_list_defaults_to_inbox_limit_10_and_renders_summary_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(header("authorization", "Bearer mail-access"))
        .and(query_param("maxResults", "10"))
        .and(query_param("labelIds", "INBOX"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "messages": [
                { "id": "message-1" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-1"))
        .and(query_param("format", "metadata"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-1",
            "payload": {
                "headers": [
                    { "name": "Date", "value": "Wed, 24 Jun 2026 10:00:00 +0000" },
                    { "name": "From", "value": "Alice <alice@example.com>" },
                    { "name": "Subject", "value": "Roadmap" }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_list_to(&client, None, false, &mut out, Some(&messages_url))
        .await
        .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "DATE\tFROM\tSUBJECT\tMESSAGE ID\nWed, 24 Jun 2026 10:00:00 +0000\tAlice <alice@example.com>\tRoadmap\tmessage-1\n"
    );
}

#[tokio::test]
async fn run_list_uses_explicit_limit_for_inbox_messages() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(header("authorization", "Bearer mail-access"))
        .and(query_param("maxResults", "25"))
        .and(query_param("labelIds", "INBOX"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "messages": []
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_list_to(&client, Some(25), false, &mut out, Some(&messages_url))
        .await
        .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "DATE\tFROM\tSUBJECT\tMESSAGE ID\n"
    );
}

#[tokio::test]
async fn run_search_emits_ndjson_summary_rows() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(query_param("maxResults", "25"))
        .and(query_param("q", "has:attachment"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "messages": [
                { "id": "message-1" }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-1"))
        .and(query_param("format", "metadata"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-1",
            "payload": {
                "headers": [
                    { "name": "Date", "value": "Wed, 24 Jun 2026 10:00:00 +0000" },
                    { "name": "From", "value": "Alice <alice@example.com>" },
                    { "name": "Subject", "value": "Roadmap" }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_search_to(
        &client,
        "has:attachment".into(),
        Some(25),
        true,
        &mut out,
        Some(&messages_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"messageId\":\"message-1\",\"date\":\"Wed, 24 Jun 2026 10:00:00 +0000\",\"from\":\"Alice <alice@example.com>\",\"subject\":\"Roadmap\"}\n"
    );
}

#[tokio::test]
async fn run_attachment_download_writes_bytes_to_output_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/gmail/v1/users/me/messages/message-1/attachments/attachment-1",
        ))
        .and(header("authorization", "Bearer mail-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": "aGVsbG8gbWFpbA"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("mail.txt");
    let store = MemoryStore::default();
    let client = test_client(&store);
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_attachment_download_to(
        &client,
        "message-1".into(),
        "attachment-1".into(),
        Some(output.clone()),
        true,
        Some(&messages_url),
    )
    .await
    .unwrap();

    assert_eq!(std::fs::read(output).unwrap(), b"hello mail");
}

#[tokio::test]
async fn run_read_prints_message_json_to_stdout() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-123"))
        .and(header("authorization", "Bearer mail-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-123",
            "snippet": "Hello from GoogleMail"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_read_to(&client, "message-123".into(), &mut out, Some(&messages_url))
        .await
        .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"id\":\"message-123\",\"snippet\":\"Hello from GoogleMail\"}\n"
    );
}
