use chrono::{Duration, Utc};
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::account::{AccountStore, Token};
use super::client::AuthClient;
use super::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use super::error::AuthError;
use super::testing::MemoryStore;

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

fn test_config_with_active(active_account: &str) -> Config {
    let mut config = test_config();
    config.settings = Some(SettingsConfig {
        active_account: Some(active_account.into()),
        output: None,
    });
    config.accounts = vec!["alice@example.com".into(), "bob@example.com".into()];
    config
}

fn test_token(access_token: &str) -> Token {
    Token {
        access_token: access_token.into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec!["openid".into()],
    }
}

fn expiring_token(access_token: &str) -> Token {
    Token {
        expiry: Utc::now() + Duration::seconds(10),
        ..test_token(access_token)
    }
}

fn client_with_token_url<'a>(
    store: &'a MemoryStore,
    token_url: String,
) -> AuthClient<'a, MemoryStore> {
    let mut client = AuthClient::from_config(test_config(), store, None).unwrap();
    client.token_url = token_url;
    client
}

#[tokio::test]
async fn sends_bearer_authorization_header() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer access-abc"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &test_token("access-abc"))
        .unwrap();

    let client = AuthClient::from_config(test_config(), &store, None).unwrap();
    let response = client
        .send(client.get(format!("{}/drive/v3/files", server.uri())))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn already_granted_scopes_do_not_trigger_incremental_authorization() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token(
            "alice@example.com",
            &Token {
                scopes: vec![
                    "openid".into(),
                    "https://www.googleapis.com/auth/drive".into(),
                ],
                ..test_token("drive-access")
            },
        )
        .unwrap();

    let client = client_with_token_url(&store, format!("{}/token", server.uri()));

    let response = client
        .send_with_scopes(
            client.get(format!("{}/drive/v3/files", server.uri())),
            &["https://www.googleapis.com/auth/drive"],
        )
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        store
            .load_token("alice@example.com")
            .unwrap()
            .unwrap()
            .access_token,
        "drive-access"
    );
}

#[tokio::test]
async fn missing_scopes_do_not_open_incremental_authorization() {
    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &test_token("profile-access"))
        .unwrap();

    let client = client_with_token_url(&store, "https://example.test/token".into());

    let error = client
        .send_with_scopes(
            client.get("https://example.test/drive/v3/files"),
            &["https://www.googleapis.com/auth/drive"],
        )
        .await
        .unwrap_err();

    assert_eq!(
        error.to_string(),
        "account alice@example.com is missing required Google scopes: https://www.googleapis.com/auth/drive; run `goog auth login` once to authorize all supported services"
    );
}

#[tokio::test]
async fn refreshes_expiring_token_before_sending_request_and_saves_it() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("client_id=client-123"))
        .and(body_string_contains("client_secret=secret-456"))
        .and(body_string_contains("refresh_token=refresh-123"))
        .and(body_string_contains("grant_type=refresh_token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "fresh-access",
            "expires_in": 3600,
            "scope": "openid https://www.googleapis.com/auth/drive",
            "token_type": "Bearer",
        })))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer fresh-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &expiring_token("stale-access"))
        .unwrap();

    let client = client_with_token_url(&store, format!("{}/token", server.uri()));
    let response = client
        .send(client.get(format!("{}/drive/v3/files", server.uri())))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    let saved = store.load_token("alice@example.com").unwrap().unwrap();
    assert_eq!(saved.access_token, "fresh-access");
    assert!(saved.expiry > Utc::now() + Duration::minutes(50));
    assert_eq!(
        saved.scopes,
        vec![
            "openid".to_string(),
            "https://www.googleapis.com/auth/drive".to_string()
        ]
    );
}

#[tokio::test]
async fn refreshes_once_and_retries_after_unauthorized_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer expired-access"))
        .respond_with(ResponseTemplate::new(401).set_body_string("expired"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .and(body_string_contains("refresh_token=refresh-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "retried-access",
            "expires_in": 3600,
            "token_type": "Bearer",
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer retried-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &test_token("expired-access"))
        .unwrap();

    let client = client_with_token_url(&store, format!("{}/token", server.uri()));
    let response = client
        .send(client.get(format!("{}/drive/v3/files", server.uri())))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert_eq!(
        store
            .load_token("alice@example.com")
            .unwrap()
            .unwrap()
            .access_token,
        "retried-access"
    );
}

#[tokio::test]
async fn second_unauthorized_after_refresh_is_terminal() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer expired-access"))
        .respond_with(ResponseTemplate::new(401).set_body_string("expired"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "access_token": "still-unauthorized",
            "expires_in": 3600,
            "token_type": "Bearer",
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer still-unauthorized"))
        .respond_with(ResponseTemplate::new(401).set_body_string("nope"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &test_token("expired-access"))
        .unwrap();

    let client = client_with_token_url(&store, format!("{}/token", server.uri()));
    let err = client
        .send(client.get(format!("{}/drive/v3/files", server.uri())))
        .await
        .unwrap_err();

    match err {
        AuthError::Unauthorized(msg) => assert!(msg.contains("401")),
        other => panic!("expected Unauthorized, got {other:?}"),
    }
}

#[tokio::test]
async fn revoked_refresh_token_returns_token_revoked() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
            "error": "invalid_grant",
            "error_description": "Token has been expired or revoked.",
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &expiring_token("stale-access"))
        .unwrap();

    let client = client_with_token_url(&store, format!("{}/token", server.uri()));
    let err = client
        .send(client.get(format!("{}/drive/v3/files", server.uri())))
        .await
        .unwrap_err();

    match err {
        AuthError::TokenRevoked(msg) => assert!(msg.contains("goog auth login")),
        other => panic!("expected TokenRevoked, got {other:?}"),
    }
}

#[tokio::test]
async fn uses_active_account_from_config_when_store_has_no_active_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store.seed_account_without_activating("bob@example.com", &test_token("bob-access"));

    let client =
        AuthClient::from_config(test_config_with_active("bob@example.com"), &store, None).unwrap();
    let response = client
        .send(client.get(format!("{}/drive/v3/files", server.uri())))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}

#[tokio::test]
async fn account_override_wins_over_active_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{}"))
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &test_token("alice-access"))
        .unwrap();
    store
        .save_token("bob@example.com", &test_token("bob-access"))
        .unwrap();

    let client = AuthClient::from_config(
        test_config_with_active("bob@example.com"),
        &store,
        Some("alice@example.com"),
    )
    .unwrap();
    let response = client
        .send(client.get(format!("{}/drive/v3/files", server.uri())))
        .await
        .unwrap();

    assert_eq!(response.status(), reqwest::StatusCode::OK);
}
