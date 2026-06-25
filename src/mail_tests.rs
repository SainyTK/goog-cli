use std::sync::{Arc, Mutex, MutexGuard};

use chrono::{Duration, Utc};
use wiremock::matchers::{body_string_contains, header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::{AuthClient, AuthorizationCode, AuthorizationCodeFlow};
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::error::AuthError;
use crate::auth::testing::MemoryStore;
use crate::mail::*;

static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

struct CurrentDirGuard {
    original: std::path::PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl CurrentDirGuard {
    fn enter(path: impl AsRef<std::path::Path>) -> Self {
        let lock = CURRENT_DIR_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).unwrap();
    }
}

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

fn profile_token() -> Token {
    Token {
        access_token: "profile-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec!["openid".into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store.save_token("alice@example.com", &mail_token()).unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

fn messages_url(server: &MockServer) -> String {
    format!("{}/gmail/v1/users/me/messages", server.uri())
}

fn attachment_path(message_id: &str, attachment_id: &str) -> String {
    format!("/gmail/v1/users/me/messages/{message_id}/attachments/{attachment_id}")
}

fn download_attachment_options(
    server: &MockServer,
    message_id: &str,
    attachment_id: &str,
) -> DownloadAttachmentOptions {
    DownloadAttachmentOptions::new(message_id, attachment_id)
        .with_messages_url(messages_url(server))
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
            code: "mail-code".into(),
        })
    }
}

#[tokio::test]
async fn list_messages_defaults_to_inbox_and_hydrates_summary_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(header("authorization", "Bearer mail-access"))
        .and(query_param("maxResults", "10"))
        .and(query_param("labelIds", "INBOX"))
        .and(query_param("fields", "messages(id),nextPageToken"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "messages": [
                { "id": "message-1" },
                { "id": "message-2" }
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
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-2"))
        .and(query_param("format", "metadata"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-2",
            "payload": {
                "headers": [
                    { "name": "date", "value": "Wed, 24 Jun 2026 11:00:00 +0000" },
                    { "name": "from", "value": "Bob <bob@example.com>" },
                    { "name": "subject", "value": "Status" }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListMessagesOptions::inbox(10).with_messages_url(messages_url(&server));

    let summaries = list_messages(&client, &options).await.unwrap();

    assert_eq!(
        summaries,
        vec![
            MessageSummary {
                id: "message-1".into(),
                date: "Wed, 24 Jun 2026 10:00:00 +0000".into(),
                from: "Alice <alice@example.com>".into(),
                subject: "Roadmap".into(),
            },
            MessageSummary {
                id: "message-2".into(),
                date: "Wed, 24 Jun 2026 11:00:00 +0000".into(),
                from: "Bob <bob@example.com>".into(),
                subject: "Status".into(),
            },
        ]
    );
}

#[tokio::test]
async fn search_messages_passes_mailbox_query_through() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages"))
        .and(header("authorization", "Bearer mail-access"))
        .and(query_param("maxResults", "25"))
        .and(query_param("q", "from:alice@example.com has:attachment"))
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
    let options = ListMessagesOptions::search("from:alice@example.com has:attachment", 25)
        .with_messages_url(messages_url(&server));

    let summaries = list_messages(&client, &options).await.unwrap();

    assert_eq!(summaries[0].id, "message-1");
}

#[tokio::test]
async fn download_attachment_decodes_base64url_to_explicit_output_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(attachment_path("message-1", "attachment-1")))
        .and(header("authorization", "Bearer mail-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": "aGVsbG8gbWFpbA"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("report.txt");
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = download_attachment_options(&server, "message-1", "attachment-1")
        .with_output(output.clone());

    let downloaded = download_attachment(&client, &options).await.unwrap();

    assert_eq!(downloaded.path, output);
    assert_eq!(downloaded.bytes, 10);
    assert_eq!(std::fs::read(downloaded.path).unwrap(), b"hello mail");
}

#[tokio::test]
async fn download_attachment_uses_message_part_filename_when_output_is_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-1"))
        .and(query_param("fields", "payload"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "payload": {
                "parts": [
                    {
                        "filename": "report.txt",
                        "body": { "attachmentId": "attachment-1" }
                    }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(attachment_path("message-1", "attachment-1")))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": "cmVwb3J0"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let _current_dir = CurrentDirGuard::enter(temp.path());
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = download_attachment_options(&server, "message-1", "attachment-1");

    let downloaded = download_attachment(&client, &options).await.unwrap();

    assert_eq!(
        downloaded.path.canonicalize().unwrap(),
        temp.path().join("report.txt").canonicalize().unwrap()
    );
    assert_eq!(std::fs::read(downloaded.path).unwrap(), b"report");
}

#[tokio::test]
async fn download_attachment_uses_nested_message_part_filename_when_output_is_omitted() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-1"))
        .and(query_param("fields", "payload"))
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
        .and(path(attachment_path("message-1", "attachment-1")))
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
    let options = download_attachment_options(&server, "message-1", "attachment-1");

    let downloaded = download_attachment(&client, &options).await.unwrap();

    assert_eq!(
        downloaded.path.canonicalize().unwrap(),
        temp.path().join("invoice.pdf").canonicalize().unwrap()
    );
    assert_eq!(std::fs::read(downloaded.path).unwrap(), b"pdf");
}

#[tokio::test]
async fn download_attachment_fails_when_destination_exists() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("existing.txt");
    std::fs::write(&output, "keep").unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        DownloadAttachmentOptions::new("message-1", "attachment-1").with_output(output.clone());

    let err = download_attachment(&client, &options).await.unwrap_err();

    match err {
        MailError::Io(io) => assert_eq!(io.kind(), std::io::ErrorKind::AlreadyExists),
        _ => panic!("unexpected error: {err}"),
    }
    assert_eq!(std::fs::read_to_string(output).unwrap(), "keep");
}

#[tokio::test]
async fn download_attachment_requires_output_when_filename_is_missing() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "payload": {
                "parts": [
                    {
                        "filename": "",
                        "body": { "attachmentId": "attachment-1" }
                    }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = download_attachment_options(&server, "message-1", "attachment-1");

    let err = download_attachment(&client, &options).await.unwrap_err();

    assert!(matches!(err, MailError::MissingAttachmentFilename));
}

#[tokio::test]
async fn download_attachment_requests_only_gmail_readonly_scope_when_missing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("code=mail-code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mail-access",
            "expires_in": 3600,
            "scope": GMAIL_READONLY_SCOPE,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(attachment_path("message-1", "attachment-1")))
        .and(header("authorization", "Bearer mail-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": "c2NvcGVk"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("scoped.txt");
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
    let options =
        download_attachment_options(&server, "message-1", "attachment-1").with_output(output);

    download_attachment(&client, &options).await.unwrap();

    assert_eq!(
        scopes_seen.lock().unwrap().clone(),
        vec![GMAIL_READONLY_SCOPE.to_string()]
    );
    let saved = store.load_token("alice@example.com").unwrap().unwrap();
    assert_eq!(
        saved.scopes,
        vec!["openid".to_string(), GMAIL_READONLY_SCOPE.to_string()]
    );
}

#[tokio::test]
async fn download_attachment_returns_mail_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(attachment_path("message-1", "missing-attachment")))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = download_attachment_options(&server, "message-1", "missing-attachment")
        .with_output(temp.path().join("missing.txt"));

    let err = download_attachment(&client, &options).await.unwrap_err();

    assert!(matches!(err, MailError::NotFound));
}

#[tokio::test]
async fn download_attachment_returns_mail_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(attachment_path("message-1", "private-attachment")))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = download_attachment_options(&server, "message-1", "private-attachment")
        .with_output(temp.path().join("private.txt"));

    let err = download_attachment(&client, &options).await.unwrap_err();

    assert!(matches!(err, MailError::PermissionDenied));
}

#[tokio::test]
async fn download_attachment_returns_api_error_with_response_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(attachment_path("message-1", "attachment-1")))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream failure"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = download_attachment_options(&server, "message-1", "attachment-1")
        .with_output(temp.path().join("failed.txt"));

    let err = download_attachment(&client, &options).await.unwrap_err();

    match err {
        MailError::Api { status, body } => {
            assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "upstream failure");
        }
        _ => panic!("unexpected error: {err}"),
    }
}

#[tokio::test]
async fn get_message_fetches_raw_googlemail_message() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-123"))
        .and(header("authorization", "Bearer mail-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-123",
            "threadId": "thread-456",
            "payload": {
                "headers": [
                    { "name": "Subject", "value": "Roadmap" }
                ]
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetMessageOptions::new("message-123").with_messages_url(messages_url(&server));

    let message = get_message(&client, &options).await.unwrap();

    assert_eq!(message["id"], "message-123");
    assert_eq!(message["threadId"], "thread-456");
    assert_eq!(message["payload"]["headers"][0]["value"], "Roadmap");
}

#[tokio::test]
async fn get_message_requests_only_gmail_readonly_scope_when_missing() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("code=mail-code"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "mail-access",
            "expires_in": 3600,
            "scope": GMAIL_READONLY_SCOPE,
            "token_type": "Bearer"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-123"))
        .and(header("authorization", "Bearer mail-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "message-123"
        })))
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
    let options = GetMessageOptions::new("message-123")
        .with_messages_url(messages_url(&server));

    get_message(&client, &options).await.unwrap();

    assert_eq!(
        scopes_seen.lock().unwrap().clone(),
        vec![GMAIL_READONLY_SCOPE.to_string()]
    );
    let saved = store.load_token("alice@example.com").unwrap().unwrap();
    assert_eq!(
        saved.scopes,
        vec!["openid".to_string(), GMAIL_READONLY_SCOPE.to_string()]
    );
}

#[tokio::test]
async fn get_message_returns_mail_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/missing-message"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetMessageOptions::new("missing-message")
        .with_messages_url(messages_url(&server));

    let err = get_message(&client, &options).await.unwrap_err();

    assert!(matches!(err, MailError::NotFound));
}

#[tokio::test]
async fn get_message_returns_mail_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/private-message"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetMessageOptions::new("private-message")
        .with_messages_url(messages_url(&server));

    let err = get_message(&client, &options).await.unwrap_err();

    assert!(matches!(err, MailError::PermissionDenied));
}

#[tokio::test]
async fn get_message_returns_api_error_with_response_body() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/gmail/v1/users/me/messages/message-123"))
        .respond_with(ResponseTemplate::new(500).set_body_string("upstream failure"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetMessageOptions::new("message-123").with_messages_url(messages_url(&server));

    let err = get_message(&client, &options).await.unwrap_err();

    match err {
        MailError::Api { status, body } => {
            assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "upstream failure");
        }
        _ => panic!("unexpected error: {err}"),
    }
}
