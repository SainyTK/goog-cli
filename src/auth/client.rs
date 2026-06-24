use chrono::{Duration, Utc};
use reqwest::{IntoUrl, Method, RequestBuilder, Response, StatusCode};

use super::account::{AccountStore, Token};
use super::config::{Config, OAuthAppConfig};
use super::error::AuthError;
use super::login::{
    build_authorize_url, exchange_code_for_scopes, random_state, LoopbackServer, GOOGLE_AUTH_URL,
    GOOGLE_TOKEN_URL,
};

const DEFAULT_REFRESH_THRESHOLD_SECS: i64 = 60;

#[allow(dead_code)]
pub struct AuthClient<'a, S> {
    pub(super) http: reqwest::Client,
    pub(super) store: &'a S,
    pub(super) account_email: String,
    pub(super) oauth_app: OAuthAppConfig,
    pub(super) auth_url: String,
    pub(super) token_url: String,
    pub(super) refresh_threshold: Duration,
    pub(super) authorization_code_flow: Box<dyn AuthorizationCodeFlow + 'a>,
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
            http: new_http_client()?,
            store,
            account_email,
            oauth_app,
            auth_url: GOOGLE_AUTH_URL.to_string(),
            token_url: GOOGLE_TOKEN_URL.to_string(),
            refresh_threshold: Duration::seconds(DEFAULT_REFRESH_THRESHOLD_SECS),
            authorization_code_flow: Box::new(LoopbackAuthorizationCodeFlow),
        })
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

        let state = random_state();
        let authorization = self.authorization_code_flow.authorize(
            &self.auth_url,
            &self.oauth_app.client_id,
            &state,
            &missing_scopes,
        )?;

        let granted = exchange_code_for_scopes(
            &self.token_url,
            &self.oauth_app.client_id,
            &self.oauth_app.client_secret,
            &authorization.redirect_uri,
            &authorization.code,
        )
        .await?;

        let mut merged = token;
        merged.access_token = granted.access_token;
        if let Some(refresh_token) = granted.refresh_token {
            merged.refresh_token = refresh_token;
        }
        merged.expiry = granted.expiry;
        merge_scopes(&mut merged.scopes, granted.scopes);

        self.store.save_token(&self.account_email, &merged)?;
        Ok(merged)
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

pub trait AuthorizationCodeFlow {
    fn authorize(
        &self,
        auth_url: &str,
        client_id: &str,
        state: &str,
        scopes: &[&str],
    ) -> Result<AuthorizationCode, AuthError>;
}

pub struct AuthorizationCode {
    pub redirect_uri: String,
    pub code: String,
}

struct LoopbackAuthorizationCodeFlow;

impl AuthorizationCodeFlow for LoopbackAuthorizationCodeFlow {
    fn authorize(
        &self,
        auth_url: &str,
        client_id: &str,
        state: &str,
        scopes: &[&str],
    ) -> Result<AuthorizationCode, AuthError> {
        let server = LoopbackServer::bind()?;
        let redirect_uri = server.redirect_uri();
        let authorize_url = build_authorize_url(auth_url, client_id, &redirect_uri, scopes, state)?;

        println!("Opening browser for additional Google consent...");
        if webbrowser::open(&authorize_url).is_err() {
            println!("Could not open a browser automatically. Open this URL manually:\n  {authorize_url}");
        }

        let code = server.wait_for_callback(state)?;
        Ok(AuthorizationCode { redirect_uri, code })
    }
}

fn merge_scopes(existing: &mut Vec<String>, granted: Vec<String>) {
    for scope in granted {
        if !existing.iter().any(|known| known == &scope) {
            existing.push(scope);
        }
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
