use chrono::{Duration, Utc};
use reqwest::{IntoUrl, Method, RequestBuilder, Response, StatusCode};

use super::account::{AccountStore, Token};
use super::config::{Config, OAuthAppConfig};
use super::error::AuthError;
use super::login::GOOGLE_TOKEN_URL;

const DEFAULT_REFRESH_THRESHOLD_SECS: i64 = 60;

#[allow(dead_code)]
pub struct AuthClient<'a, S> {
    pub(super) http: reqwest::Client,
    pub(super) store: &'a S,
    pub(super) account_email: String,
    pub(super) oauth_app: OAuthAppConfig,
    pub(super) token_url: String,
    pub(super) refresh_threshold: Duration,
}

#[allow(dead_code)]
impl<'a, S: AccountStore> AuthClient<'a, S> {
    pub fn from_config(
        config: Config,
        store: &'a S,
        account_override: Option<&str>,
    ) -> Result<Self, AuthError> {
        let account_email = resolve_account(&config, store, account_override)?;
        let oauth_app = config.oauth_app.ok_or(AuthError::OAuthAppNotConfigured)?;

        Ok(Self {
            http: new_http_client()?,
            store,
            account_email,
            oauth_app,
            token_url: GOOGLE_TOKEN_URL.to_string(),
            refresh_threshold: Duration::seconds(DEFAULT_REFRESH_THRESHOLD_SECS),
        })
    }

    pub fn with_refresh_threshold(mut self, refresh_threshold: Duration) -> Self {
        self.refresh_threshold = refresh_threshold;
        self
    }

    pub(crate) fn account_email(&self) -> &str {
        &self.account_email
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
        self.send_with_scopes(request, &[]).await
    }

    pub async fn send_with_scopes(
        &self,
        request: RequestBuilder,
        required_scopes: &[&str],
    ) -> Result<Response, AuthError> {
        let token = self.current_token_with_scopes(required_scopes).await?;
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

    async fn current_token_with_scopes(
        &self,
        required_scopes: &[&str],
    ) -> Result<Token, AuthError> {
        let token = self.current_token().await?;
        self.ensure_scopes(token, required_scopes).await
    }

    async fn current_token(&self) -> Result<Token, AuthError> {
        let token = self.store.load_token(&self.account_email)?.ok_or_else(|| {
            AuthError::TokenNotFound {
                email: self.account_email.clone(),
            }
        })?;

        if token.expiry - Utc::now() <= self.refresh_threshold {
            self.refresh_token(&token).await
        } else {
            Ok(token)
        }
    }

    async fn ensure_scopes(
        &self,
        token: Token,
        required_scopes: &[&str],
    ) -> Result<Token, AuthError> {
        let missing_scopes: Vec<&str> = required_scopes
            .iter()
            .copied()
            .filter(|scope| !token.scopes.iter().any(|granted| granted == scope))
            .collect();

        if missing_scopes.is_empty() {
            return Ok(token);
        }

        Err(AuthError::MissingScopes {
            email: self.account_email.clone(),
            scopes: missing_scopes.join(", "),
        })
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

fn new_http_client() -> Result<reqwest::Client, AuthError> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|e| AuthError::Network(e.to_string()))
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

fn resolve_account(
    config: &Config,
    store: &impl AccountStore,
    account_override: Option<&str>,
) -> Result<String, AuthError> {
    if let Some(account) = account_override {
        if !store.account_exists(account)? {
            return Err(AuthError::AccountNotFound {
                email: account.to_string(),
            });
        }
        return Ok(account.to_string());
    }

    if let Some(active) = store.active_account()? {
        return Ok(active);
    }

    if let Some(active) = config.active_account() {
        if store.account_exists(active)? {
            return Ok(active.to_string());
        }
    }

    Err(AuthError::ActiveAccountNotConfigured)
}
