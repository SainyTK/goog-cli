use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path};
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
