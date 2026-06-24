use std::sync::{Arc, Mutex};

use chrono::{Duration, Utc};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::{AuthClient, AuthorizationCode, AuthorizationCodeFlow};
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::error::AuthError;
use crate::auth::testing::MemoryStore;
use crate::mail::*;

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
    let options = GetMessageOptions::new("message-123")
        .with_messages_url(format!("{}/gmail/v1/users/me/messages", server.uri()));

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
        .with_messages_url(format!("{}/gmail/v1/users/me/messages", server.uri()));

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
        .with_messages_url(format!("{}/gmail/v1/users/me/messages", server.uri()));

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
        .with_messages_url(format!("{}/gmail/v1/users/me/messages", server.uri()));

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
    let options = GetMessageOptions::new("message-123")
        .with_messages_url(format!("{}/gmail/v1/users/me/messages", server.uri()));

    let err = get_message(&client, &options).await.unwrap_err();

    match err {
        MailError::Api { status, body } => {
            assert_eq!(status, reqwest::StatusCode::INTERNAL_SERVER_ERROR);
            assert_eq!(body, "upstream failure");
        }
        _ => panic!("unexpected error: {err}"),
    }
}
