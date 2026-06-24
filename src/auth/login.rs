use std::net::{Ipv4Addr, SocketAddr, TcpListener};
use std::time::{Duration as StdDuration, SystemTime, UNIX_EPOCH};

use chrono::{Duration, Utc};
use serde::Deserialize;
use url::Url;

use super::account::Token;
use super::error::AuthError;

pub const DEFAULT_LOGIN_SCOPES: &[&str] = &[
    "openid",
    "https://www.googleapis.com/auth/userinfo.email",
    "https://www.googleapis.com/auth/userinfo.profile",
];

pub const DEFAULT_DEVICE_LOGIN_SCOPES: &[&str] = &["openid", "email", "profile"];

pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

const GOOGLE_DEVICE_AUTH_CLIENT_TYPE: &str = "TVs and Limited Input devices";
const DEVICE_GRANT_TYPE: &str = "urn:ietf:params:oauth:grant-type:device_code";
const DEFAULT_DEVICE_POLL_INTERVAL_SECS: u64 = 5;
const SLOW_DOWN_INTERVAL_SECS: u64 = 5;

pub fn random_state() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let pid = std::process::id() as u128;
    format!("{:032x}", nanos ^ pid.wrapping_mul(0x9e3779b97f4a7c15))
}

pub fn build_authorize_url(
    auth_url: &str,
    client_id: &str,
    redirect_uri: &str,
    scopes: &[&str],
    state: &str,
) -> Result<String, AuthError> {
    let mut url = Url::parse(auth_url).map_err(|e| AuthError::OAuthFlow(e.to_string()))?;
    url.query_pairs_mut()
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", &scopes.join(" "))
        .append_pair("state", state)
        .append_pair("access_type", "offline")
        .append_pair("include_granted_scopes", "true")
        .append_pair("prompt", "consent");
    Ok(url.to_string())
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
    expires_in: i64,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopedToken {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expiry: chrono::DateTime<Utc>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceAuthorization {
    pub device_code: String,
    pub user_code: String,
    pub verification_url: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceAuthorizationResponse {
    device_code: String,
    user_code: String,
    #[serde(alias = "verification_uri")]
    verification_url: String,
    expires_in: u64,
    interval: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct OAuthErrorResponse {
    error: String,
    error_description: Option<String>,
}

enum DevicePollOutcome {
    Continue,
    SlowDown,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    email: String,
}

pub async fn exchange_code(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<Token, AuthError> {
    token_from_parsed_response(
        request_authorization_code_token(token_url, client_id, client_secret, redirect_uri, code)
            .await?,
    )
}

pub async fn exchange_code_for_scopes(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<ScopedToken, AuthError> {
    Ok(scoped_token_from_parsed_response(
        request_authorization_code_token(token_url, client_id, client_secret, redirect_uri, code)
            .await?,
    ))
}

async fn request_authorization_code_token(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    redirect_uri: &str,
    code: &str,
) -> Result<TokenResponse, AuthError> {
    let client = reqwest::Client::new();
    let params = [
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
        ("redirect_uri", redirect_uri),
        ("grant_type", "authorization_code"),
    ];
    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AuthError::Network(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AuthError::TokenExchange(format!("HTTP {status}: {body}")));
    }

    parse_token_response(response).await
}

async fn token_from_response(response: reqwest::Response) -> Result<Token, AuthError> {
    token_from_parsed_response(parse_token_response(response).await?)
}

async fn parse_token_response(response: reqwest::Response) -> Result<TokenResponse, AuthError> {
    response
        .json()
        .await
        .map_err(|e| AuthError::TokenExchange(format!("invalid token response: {e}")))
}

fn token_from_parsed_response(parsed: TokenResponse) -> Result<Token, AuthError> {
    let refresh_token = parsed
        .refresh_token
        .ok_or_else(|| AuthError::TokenExchange("response missing refresh_token".into()))?;

    Ok(Token {
        access_token: parsed.access_token,
        refresh_token,
        expiry: Utc::now() + Duration::seconds(parsed.expires_in),
        scopes: parse_scopes(&parsed.scope),
    })
}

fn scoped_token_from_parsed_response(parsed: TokenResponse) -> ScopedToken {
    ScopedToken {
        access_token: parsed.access_token,
        refresh_token: parsed.refresh_token,
        expiry: Utc::now() + Duration::seconds(parsed.expires_in),
        scopes: parse_scopes(&parsed.scope),
    }
}

fn parse_scopes(scope: &str) -> Vec<String> {
    scope.split_whitespace().map(str::to_string).collect()
}

pub async fn request_device_authorization(
    device_code_url: &str,
    client_id: &str,
    scopes: &[&str],
) -> Result<DeviceAuthorization, AuthError> {
    let client = reqwest::Client::new();
    let scope = scopes.join(" ");
    let params = [("client_id", client_id), ("scope", scope.as_str())];
    let response = client
        .post(device_code_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AuthError::Network(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AuthError::OAuthFlow(format!(
            "device authorization request failed with HTTP {status}: {}",
            describe_device_authorization_error(&body)
        )));
    }

    let parsed: DeviceAuthorizationResponse = response
        .json()
        .await
        .map_err(|e| AuthError::OAuthFlow(format!("invalid device authorization response: {e}")))?;

    Ok(DeviceAuthorization {
        device_code: parsed.device_code,
        user_code: parsed.user_code,
        verification_url: parsed.verification_url,
        expires_in: parsed.expires_in,
        interval: parsed.interval.unwrap_or(DEFAULT_DEVICE_POLL_INTERVAL_SECS),
    })
}

fn describe_device_authorization_error(body: &str) -> String {
    let Ok(error) = serde_json::from_str::<OAuthErrorResponse>(body) else {
        return body.to_string();
    };

    if error.error == "invalid_client" {
        let description = error
            .error_description
            .unwrap_or_else(|| "Invalid client".to_string());
        return format!(
            "{description}. Google device authorization requires an OAuth client of type \"{GOOGLE_DEVICE_AUTH_CLIENT_TYPE}\"."
        );
    }

    match error.error_description {
        Some(description) => format!("{}: {description}", error.error),
        None => error.error,
    }
}

pub fn render_device_authorization_prompt(authorization: &DeviceAuthorization) -> String {
    format!(
        "Open this URL on any device to authorize:\n  {}\nUser code: {}\n",
        authorization.verification_url, authorization.user_code
    )
}

pub async fn poll_device_token(
    token_url: &str,
    client_id: &str,
    client_secret: &str,
    device_code: &str,
    interval_secs: u64,
    expires_in_secs: u64,
) -> Result<Token, AuthError> {
    let client = reqwest::Client::new();
    let deadline = tokio::time::Instant::now() + StdDuration::from_secs(expires_in_secs);
    let mut interval = StdDuration::from_secs(interval_secs);

    loop {
        let params = [
            ("client_id", client_id),
            ("client_secret", client_secret),
            ("device_code", device_code),
            ("grant_type", DEVICE_GRANT_TYPE),
        ];
        let response = client
            .post(token_url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AuthError::Network(e.to_string()))?;

        if response.status().is_success() {
            return token_from_response(response).await;
        }

        match device_poll_outcome(response).await? {
            DevicePollOutcome::Continue => {}
            DevicePollOutcome::SlowDown => {
                interval += StdDuration::from_secs(SLOW_DOWN_INTERVAL_SECS);
            }
        }

        if tokio::time::Instant::now() >= deadline {
            return Err(AuthError::OAuthFlow(
                "device authorization timed out".into(),
            ));
        }

        tokio::time::sleep(interval).await;
    }
}

async fn device_poll_outcome(response: reqwest::Response) -> Result<DevicePollOutcome, AuthError> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let token_error = serde_json::from_str::<OAuthErrorResponse>(&body)
        .map_err(|_| AuthError::TokenExchange(format!("device token HTTP {status}: {body}")))?;

    let OAuthErrorResponse {
        error,
        error_description,
    } = token_error;

    match error.as_str() {
        "authorization_pending" => Ok(DevicePollOutcome::Continue),
        "slow_down" => Ok(DevicePollOutcome::SlowDown),
        "access_denied" => Err(AuthError::OAuthFlow(
            "device authorization was denied by the user".into(),
        )),
        "expired_token" => Err(AuthError::OAuthFlow(
            "device authorization timed out".into(),
        )),
        error => {
            let description = error_description.unwrap_or(body);
            Err(AuthError::TokenExchange(format!(
                "device token error {error}: {description}"
            )))
        }
    }
}

pub async fn fetch_email(userinfo_url: &str, access_token: &str) -> Result<String, AuthError> {
    let client = reqwest::Client::new();
    let response = client
        .get(userinfo_url)
        .bearer_auth(access_token)
        .send()
        .await
        .map_err(|e| AuthError::Network(e.to_string()))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(AuthError::OAuthFlow(format!(
            "userinfo HTTP {status}: {body}"
        )));
    }

    let info: UserInfoResponse = response
        .json()
        .await
        .map_err(|e| AuthError::OAuthFlow(format!("invalid userinfo response: {e}")))?;
    Ok(info.email)
}

pub struct LoopbackServer {
    server: tiny_http::Server,
    port: u16,
}

impl LoopbackServer {
    pub fn bind() -> Result<Self, AuthError> {
        let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, 0));
        let listener = TcpListener::bind(addr).map_err(|e| AuthError::OAuthFlow(e.to_string()))?;
        let port = listener
            .local_addr()
            .map_err(|e| AuthError::OAuthFlow(e.to_string()))?
            .port();
        let server = tiny_http::Server::from_listener(listener, None)
            .map_err(|e| AuthError::OAuthFlow(e.to_string()))?;
        Ok(Self { server, port })
    }

    pub fn redirect_uri(&self) -> String {
        format!("http://127.0.0.1:{}/", self.port)
    }

    pub fn wait_for_callback(&self, expected_state: &str) -> Result<String, AuthError> {
        let request = self
            .server
            .recv_timeout(StdDuration::from_secs(300))
            .map_err(|e| AuthError::OAuthFlow(format!("loopback recv failed: {e}")))?
            .ok_or_else(|| AuthError::OAuthFlow("timed out waiting for OAuth redirect".into()))?;

        let url = format!("http://127.0.0.1{}", request.url());
        let parsed = Url::parse(&url).map_err(|e| AuthError::OAuthFlow(e.to_string()))?;
        let callback = parse_callback_params(&parsed, expected_state);

        let body = match &callback {
            Ok(_) => "You can close this tab. The CLI has the authorization code.",
            Err(_) => "Authorization failed. Check the terminal for details.",
        };
        let response = tiny_http::Response::from_string(body).with_header(
            "Content-Type: text/plain; charset=utf-8"
                .parse::<tiny_http::Header>()
                .unwrap(),
        );
        let _ = request.respond(response);

        callback
    }
}

pub fn parse_callback_params(url: &Url, expected_state: &str) -> Result<String, AuthError> {
    let mut code: Option<String> = None;
    let mut state: Option<String> = None;
    let mut error: Option<String> = None;

    for (k, v) in url.query_pairs() {
        match k.as_ref() {
            "code" => code = Some(v.into_owned()),
            "state" => state = Some(v.into_owned()),
            "error" => error = Some(v.into_owned()),
            _ => {}
        }
    }

    if let Some(err) = error {
        return Err(AuthError::OAuthFlow(format!(
            "consent screen returned error: {err}"
        )));
    }

    let state =
        state.ok_or_else(|| AuthError::OAuthFlow("callback missing state parameter".into()))?;
    if state != expected_state {
        return Err(AuthError::OAuthFlow(format!(
            "state mismatch: expected {expected_state}, got {state}"
        )));
    }

    code.ok_or_else(|| AuthError::OAuthFlow("callback missing code parameter".into()))
}
