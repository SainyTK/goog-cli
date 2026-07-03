use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::state::{load_runtime_state_from_path, resource_key};
use crate::auth::testing::MemoryStore;
use crate::mail::GMAIL_SCOPE;
use crate::test_support::CurrentDirGuard;

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
    scoped_mail_token("mail-access")
}

fn scoped_mail_token(access_token: &str) -> Token {
    Token {
        access_token: access_token.into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![GMAIL_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &mail_token())
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
        .save_token("alice@example.com", &scoped_mail_token("alice-access"))
        .unwrap();
    store
        .save_token("bob@example.com", &scoped_mail_token("bob-access"))
        .unwrap();
    store
        .save_token("carol@example.com", &scoped_mail_token("carol-access"))
        .unwrap();
    store
}

struct AbsentQueryParam(&'static str);

impl Match for AbsentQueryParam {
    fn matches(&self, request: &Request) -> bool {
        !request.url.query_pairs().any(|(name, _)| name == self.0)
    }
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
async fn run_search_defaults_to_limit_10_without_forcing_inbox_and_renders_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(header("authorization", "Bearer mail-access"))
        .and(query_param("maxResults", "10"))
        .and(query_param("q", "from:alice@example.com"))
        .and(query_param("fields", "messages(id),nextPageToken"))
        .and(AbsentQueryParam("labelIds"))
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
        .and(query_param("metadataHeaders", "Date"))
        .and(query_param("metadataHeaders", "From"))
        .and(query_param("metadataHeaders", "Subject"))
        .and(query_param("fields", "id,payload(headers(name,value))"))
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
        "from:alice@example.com".into(),
        None,
        false,
        &mut out,
        Some(&messages_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "DATE\tFROM\tSUBJECT\tMESSAGE ID\nWed, 24 Jun 2026 10:00:00 +0000\tAlice <alice@example.com>\tRoadmap\tmessage-1\n"
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
async fn run_attachment_download_uses_part_filename_when_output_is_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "payload": {
                "parts": [
                    {
                        "filename": "",
                        "parts": [
                            {
                                "filename": "invoice.pdf",
                                "body": { "attachmentId": "attachment-1" }
                            }
                        ]
                    }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/gmail/v1/users/me/messages/message-1/attachments/attachment-1",
        ))
        .and(header("authorization", "Bearer mail-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": "cGRm"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let _current_dir = CurrentDirGuard::enter(temp.path());
    let store = MemoryStore::default();
    let client = test_client(&store);
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_attachment_download_to(
        &client,
        "message-1".into(),
        "attachment-1".into(),
        None,
        true,
        Some(&messages_url),
    )
    .await
    .unwrap();

    assert_eq!(
        std::fs::read(temp.path().join("invoice.pdf")).unwrap(),
        b"pdf"
    );
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

#[tokio::test]
async fn run_read_unified_falls_back_for_account_local_message_ids_and_maps_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-123",
            "snippet": "Hello from Bob"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_read_unified_to(
        &config,
        &store,
        None,
        "message-123".into(),
        &mut out,
        Some(&messages_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("mail", "message-123")),
        Some("bob@example.com")
    );
    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"id\":\"message-123\",\"snippet\":\"Hello from Bob\"}\n"
    );
}

#[tokio::test]
async fn run_read_unified_does_not_fallback_for_explicit_account_but_maps_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-456"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-456",
            "snippet": "Hello from Bob"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    let mut denied_out = Vec::new();
    let denied = run_read_unified_to(
        &config,
        &store,
        Some("alice@example.com"),
        "message-123".into(),
        &mut denied_out,
        Some(&messages_url),
        Some(&state_path),
    )
    .await;

    let message = format!("{:#}", denied.unwrap_err());
    assert!(message.contains("failed to fetch GoogleMail Message"));
    assert!(message.contains("GoogleMail Message was not found"));
    assert!(denied_out.is_empty());

    let mut mapped_out = Vec::new();
    run_read_unified_to(
        &config,
        &store,
        Some("bob@example.com"),
        "message-456".into(),
        &mut mapped_out,
        Some(&messages_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("mail", "message-456")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_attachment_download_unified_uses_message_target_fallback_and_maps_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/gmail/v1/users/me/messages/message-123/attachments/attachment-1",
        ))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/gmail/v1/users/me/messages/message-123/attachments/attachment-1",
        ))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": "aGVsbG8gbWFpbA"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let output = temp_dir.path().join("mail.txt");
    let messages_url = format!("{}/gmail/v1/users/me/messages", server.uri());

    run_attachment_download_unified_to(
        &config,
        &store,
        None,
        "message-123".into(),
        "attachment-1".into(),
        Some(output.clone()),
        true,
        Some(&messages_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(std::fs::read(output).unwrap(), b"hello mail");
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("mail", "message-123")),
        Some("bob@example.com")
    );
}
