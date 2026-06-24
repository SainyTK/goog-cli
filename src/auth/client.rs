use chrono::{Duration, Utc};
use reqwest::{IntoUrl, Method, RequestBuilder, Response, StatusCode};

use super::account::{AccountStore, Token};
use super::config::{Config, OAuthAppConfig};
use super::error::AuthError;
use super::login::GOOGLE_TOKEN_URL;

const DEFAULT_REFRESH_THRESHOLD_SECS: i64 = 60;

#[allow(dead_code)]
pub struct AuthClient<'a, S> {
    http: reqwest::Client,
    store: &'a S,
    account_email: String,
    oauth_app: OAuthAppConfig,
    token_url: String,
    refresh_threshold: Duration,
}

#[allow(dead_code)]
impl<'a, S: AccountStore> AuthClient<'a, S> {
    pub fn from_config(
        config: Config,
        store: &'a S,
        account_override: Option<&str>,
    ) -> Result<Self, AuthError> {
        let account_email = resolve_account(&config, account_override)?;
        let oauth_app = config.oauth_app.ok_or(AuthError::OAuthAppNotConfigured)?;

        Ok(Self {
            http: reqwest::Client::new(),
            store,
            account_email,
            oauth_app,
            token_url: GOOGLE_TOKEN_URL.to_string(),
            refresh_threshold: Duration::seconds(DEFAULT_REFRESH_THRESHOLD_SECS),
        })
    }

    #[cfg(test)]
    fn with_token_url(mut self, token_url: impl Into<String>) -> Self {
        self.token_url = token_url.into();
        self
    }

    pub fn with_refresh_threshold(mut self, refresh_threshold: Duration) -> Self {
        self.refresh_threshold = refresh_threshold;
        self
    }

    pub fn request<U: IntoUrl>(&self, method: Method, url: U) -> RequestBuilder {
        self.http.request(method, url)
    }

    pub fn get<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.http.get(url)
    }

    pub fn post<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.http.post(url)
    }

    pub fn put<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.http.put(url)
    }

    pub fn delete<U: IntoUrl>(&self, url: U) -> RequestBuilder {
        self.http.delete(url)
    }

    pub async fn send(&self, request: RequestBuilder) -> Result<Response, AuthError> {
        let token = self.current_token().await?;
        let retry_request = request.try_clone();
        let response = send_with_access_token(request, &token.access_token).await?;

        if response.status() != StatusCode::UNAUTHORIZED {
            return Ok(response);
        }

        let retry_request = retry_request.ok_or(AuthError::RequestNotRetryable)?;
        let token = self.refresh_token(&token).await?;
        let response = send_with_access_token(retry_request, &token.access_token).await?;

        if response.status() == StatusCode::UNAUTHORIZED {
            return Err(AuthError::Unauthorized(
                "request returned 401 after refreshing the token".into(),
            ));
        }

        Ok(response)
    }

    async fn current_token(&self) -> Result<Token, AuthError> {
        let token = self
            .store
            .load_token(&self.account_email)?
            .ok_or_else(|| AuthError::TokenNotFound {
                email: self.account_email.clone(),
            })?;

        if token.expiry - Utc::now() <= self.refresh_threshold {
            self.refresh_token(&token).await
        } else {
            Ok(token)
        }
    }

    async fn refresh_token(&self, token: &Token) -> Result<Token, AuthError> {
        let response = self
            .http
            .post(&self.token_url)
            .form(&[
                ("client_id", self.oauth_app.client_id.as_str()),
                ("client_secret", self.oauth_app.client_secret.as_str()),
                ("refresh_token", token.refresh_token.as_str()),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| AuthError::Network(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            if body.contains("invalid_grant") {
                return Err(AuthError::TokenRevoked(format!(
                    "refresh token for {} was rejected by Google; run `goog auth login` again",
                    self.account_email
                )));
            }
            return Err(AuthError::TokenExchange(format!(
                "refresh HTTP {status}: {body}"
            )));
        }

        #[derive(serde::Deserialize)]
        struct RefreshResponse {
            access_token: String,
            expires_in: i64,
            scope: Option<String>,
        }

        let parsed: RefreshResponse = response
            .json()
            .await
            .map_err(|e| AuthError::TokenExchange(format!("invalid refresh response: {e}")))?;

        let mut refreshed = token.clone();
        refreshed.access_token = parsed.access_token;
        refreshed.expiry = Utc::now() + Duration::seconds(parsed.expires_in);
        if let Some(scope) = parsed.scope {
            refreshed.scopes = scope.split_whitespace().map(str::to_string).collect();
        }

        self.store.save_token(&self.account_email, &refreshed)?;
        Ok(refreshed)
    }
}

async fn send_with_access_token(
    request: RequestBuilder,
    access_token: &str,
) -> Result<Response, AuthError> {
    request
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AuthError::Network(e.to_string()))
}

fn resolve_account(config: &Config, account_override: Option<&str>) -> Result<String, AuthError> {
    if let Some(account) = account_override {
        return Ok(account.to_string());
    }

    config
        .settings
        .as_ref()
        .and_then(|settings| settings.active_account.as_deref())
        .map(str::to_string)
        .ok_or(AuthError::ActiveAccountNotConfigured)
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::auth::account::{testing::MemoryStore, AccountStore, Token};
    use crate::auth::config::{Config, OAuthAppConfig, SettingsConfig};

    fn test_config() -> Config {
        Config {
            oauth_app: Some(OAuthAppConfig {
                client_id: "client-123".into(),
                client_secret: "secret-456".into(),
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

        let client =
            AuthClient::from_config(test_config(), &store, None)
                .unwrap()
                .with_token_url(format!("{}/token", server.uri()));
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

        let client =
            AuthClient::from_config(test_config(), &store, None)
                .unwrap()
                .with_token_url(format!("{}/token", server.uri()));
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

        let client =
            AuthClient::from_config(test_config(), &store, None)
                .unwrap()
                .with_token_url(format!("{}/token", server.uri()));
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

        let client =
            AuthClient::from_config(test_config(), &store, None)
                .unwrap()
                .with_token_url(format!("{}/token", server.uri()));
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
    async fn uses_active_account_from_config_by_default() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(header("authorization", "Bearer bob-access"))
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

        let client =
            AuthClient::from_config(test_config_with_active("bob@example.com"), &store, None)
                .unwrap();
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
}
