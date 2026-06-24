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

pub const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
pub const GOOGLE_DEVICE_CODE_URL: &str = "https://oauth2.googleapis.com/device/code";
pub const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GOOGLE_USERINFO_URL: &str = "https://openidconnect.googleapis.com/v1/userinfo";

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
struct DeviceTokenError {
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
        return Err(AuthError::TokenExchange(format!(
            "HTTP {status}: {body}"
        )));
    }

    token_from_response(response).await
}

async fn token_from_response(response: reqwest::Response) -> Result<Token, AuthError> {
    let parsed: TokenResponse = response
        .json()
        .await
        .map_err(|e| AuthError::TokenExchange(format!("invalid token response: {e}")))?;

    token_from_parsed_response(parsed)
}

fn token_from_parsed_response(parsed: TokenResponse) -> Result<Token, AuthError> {
    let refresh_token = parsed
        .refresh_token
        .ok_or_else(|| AuthError::TokenExchange("response missing refresh_token".into()))?;

    let scopes = parsed
        .scope
        .split_whitespace()
        .map(|s| s.to_string())
        .collect();

    Ok(Token {
        access_token: parsed.access_token,
        refresh_token,
        expiry: Utc::now() + Duration::seconds(parsed.expires_in),
        scopes,
    })
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
            "device authorization request failed with HTTP {status}: {body}"
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
            return Err(AuthError::OAuthFlow("device authorization timed out".into()));
        }

        tokio::time::sleep(interval).await;
    }
}

async fn device_poll_outcome(response: reqwest::Response) -> Result<DevicePollOutcome, AuthError> {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let token_error = serde_json::from_str::<DeviceTokenError>(&body).map_err(|_| {
        AuthError::TokenExchange(format!("device token HTTP {status}: {body}"))
    })?;

    let DeviceTokenError {
        error,
        error_description,
    } = token_error;

    match error.as_str() {
        "authorization_pending" => Ok(DevicePollOutcome::Continue),
        "slow_down" => Ok(DevicePollOutcome::SlowDown),
        "access_denied" => Err(AuthError::OAuthFlow(
            "device authorization was denied by the user".into(),
        )),
        "expired_token" => Err(AuthError::OAuthFlow("device authorization timed out".into())),
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
        let response = tiny_http::Response::from_string(body)
            .with_header(
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;
    use wiremock::matchers::{body_string_contains, header, method, path};
    use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

    struct DeviceTokenSequence {
        responses: Mutex<VecDeque<ResponseTemplate>>,
    }

    impl DeviceTokenSequence {
        fn new(responses: Vec<ResponseTemplate>) -> Self {
            Self {
                responses: Mutex::new(responses.into()),
            }
        }
    }

    impl Respond for DeviceTokenSequence {
        fn respond(&self, _request: &Request) -> ResponseTemplate {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .unwrap_or_else(|| {
                    ResponseTemplate::new(500)
                        .set_body_string("device token mock response exhausted")
                })
        }
    }

    #[test]
    fn authorize_url_includes_required_params() {
        let url = build_authorize_url(
            GOOGLE_AUTH_URL,
            "client-123",
            "http://127.0.0.1:54321/",
            &["openid", "email"],
            "state-xyz",
        )
        .unwrap();

        let parsed = Url::parse(&url).unwrap();
        let pairs: std::collections::HashMap<_, _> = parsed.query_pairs().into_owned().collect();
        assert_eq!(pairs.get("client_id").unwrap(), "client-123");
        assert_eq!(pairs.get("redirect_uri").unwrap(), "http://127.0.0.1:54321/");
        assert_eq!(pairs.get("response_type").unwrap(), "code");
        assert_eq!(pairs.get("scope").unwrap(), "openid email");
        assert_eq!(pairs.get("state").unwrap(), "state-xyz");
        assert_eq!(pairs.get("access_type").unwrap(), "offline");
        assert_eq!(pairs.get("prompt").unwrap(), "consent");
        assert!(url.starts_with(GOOGLE_AUTH_URL));
    }

    #[tokio::test]
    async fn request_device_authorization_parses_verification_url_and_user_code() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/device/code"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(body_string_contains("client_id=client-123"))
            .and(body_string_contains("scope=openid"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "device_code": "device-code-123",
                "user_code": "ABCD-EFGH",
                "verification_url": "https://www.google.com/device",
                "expires_in": 1800,
                "interval": 7,
            })))
            .mount(&server)
            .await;

        let authorization = request_device_authorization(
            &format!("{}/device/code", server.uri()),
            "client-123",
            &["openid", "email"],
        )
        .await
        .unwrap();

        assert_eq!(authorization.device_code, "device-code-123");
        assert_eq!(authorization.user_code, "ABCD-EFGH");
        assert_eq!(authorization.verification_url, "https://www.google.com/device");
        assert_eq!(authorization.expires_in, 1800);
        assert_eq!(authorization.interval, 7);
    }

    #[test]
    fn device_authorization_prompt_shows_verification_url_and_user_code() {
        let prompt = render_device_authorization_prompt(&DeviceAuthorization {
            device_code: "device-code-123".into(),
            user_code: "ABCD-EFGH".into(),
            verification_url: "https://www.google.com/device".into(),
            expires_in: 1800,
            interval: 5,
        });

        assert!(prompt.contains("https://www.google.com/device"));
        assert!(prompt.contains("ABCD-EFGH"));
    }

    #[tokio::test]
    async fn poll_device_token_waits_through_pending_then_returns_token() {
        let server = MockServer::start().await;
        let responder = DeviceTokenSequence::new(vec![
            ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "authorization_pending",
            })),
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ya29.device",
                "refresh_token": "1//device-refresh",
                "expires_in": 3599,
                "scope": "openid https://www.googleapis.com/auth/userinfo.email",
                "token_type": "Bearer",
            })),
        ]);

        Mock::given(method("POST"))
            .and(path("/token"))
            .and(body_string_contains(
                "grant_type=urn%3Aietf%3Aparams%3Aoauth%3Agrant-type%3Adevice_code",
            ))
            .and(body_string_contains("device_code=device-code-123"))
            .respond_with(responder)
            .mount(&server)
            .await;

        let token = poll_device_token(
            &format!("{}/token", server.uri()),
            "client-123",
            "shh",
            "device-code-123",
            0,
            60,
        )
        .await
        .unwrap();

        assert_eq!(token.access_token, "ya29.device");
        assert_eq!(token.refresh_token, "1//device-refresh");
        assert_eq!(
            token.scopes,
            vec![
                "openid".to_string(),
                "https://www.googleapis.com/auth/userinfo.email".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn poll_device_token_errors_when_authorization_is_denied() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "access_denied",
                "error_description": "The user denied access",
            })))
            .mount(&server)
            .await;

        let err = poll_device_token(
            &format!("{}/token", server.uri()),
            "client-123",
            "shh",
            "device-code-123",
            0,
            60,
        )
        .await
        .unwrap_err();

        match err {
            AuthError::OAuthFlow(msg) => assert!(msg.contains("denied")),
            other => panic!("expected OAuthFlow, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn poll_device_token_errors_when_authorization_times_out() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_json(serde_json::json!({
                "error": "authorization_pending",
            })))
            .mount(&server)
            .await;

        let err = poll_device_token(
            &format!("{}/token", server.uri()),
            "client-123",
            "shh",
            "device-code-123",
            0,
            0,
        )
        .await
        .unwrap_err();

        match err {
            AuthError::OAuthFlow(msg) => assert!(msg.contains("timed out")),
            other => panic!("expected OAuthFlow, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn exchange_code_parses_a_successful_token_response() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .and(header("content-type", "application/x-www-form-urlencoded"))
            .and(body_string_contains("code=auth-code-789"))
            .and(body_string_contains("client_id=client-123"))
            .and(body_string_contains("client_secret=shh"))
            .and(body_string_contains("grant_type=authorization_code"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "ya29.access",
                "refresh_token": "1//refresh",
                "expires_in": 3599,
                "scope": "openid https://www.googleapis.com/auth/userinfo.email",
                "token_type": "Bearer",
            })))
            .mount(&server)
            .await;

        let token = exchange_code(
            &format!("{}/token", server.uri()),
            "client-123",
            "shh",
            "http://127.0.0.1:5000/",
            "auth-code-789",
        )
        .await
        .unwrap();

        assert_eq!(token.access_token, "ya29.access");
        assert_eq!(token.refresh_token, "1//refresh");
        assert_eq!(
            token.scopes,
            vec![
                "openid".to_string(),
                "https://www.googleapis.com/auth/userinfo.email".to_string()
            ]
        );
        assert!(token.expiry > Utc::now());
    }

    #[tokio::test]
    async fn exchange_code_errors_when_response_is_a_non_2xx() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(400).set_body_string("invalid_grant"))
            .mount(&server)
            .await;

        let err = exchange_code(
            &format!("{}/token", server.uri()),
            "client-123",
            "shh",
            "http://127.0.0.1:5000/",
            "bad-code",
        )
        .await
        .unwrap_err();

        match err {
            AuthError::TokenExchange(msg) => {
                assert!(msg.contains("400"));
                assert!(msg.contains("invalid_grant"));
            }
            other => panic!("expected TokenExchange, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn exchange_code_errors_when_refresh_token_is_missing() {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "access_token": "a",
                "expires_in": 60,
                "scope": "openid",
                "token_type": "Bearer",
            })))
            .mount(&server)
            .await;

        let err = exchange_code(
            &format!("{}/token", server.uri()),
            "c",
            "s",
            "r",
            "code",
        )
        .await
        .unwrap_err();

        match err {
            AuthError::TokenExchange(msg) => assert!(msg.contains("refresh_token")),
            other => panic!("expected TokenExchange, got {other:?}"),
        }
    }

    #[test]
    fn parse_callback_extracts_code_when_state_matches() {
        let url = Url::parse("http://127.0.0.1:5000/?code=abc123&state=expected").unwrap();
        let code = parse_callback_params(&url, "expected").unwrap();
        assert_eq!(code, "abc123");
    }

    #[test]
    fn parse_callback_errors_on_state_mismatch() {
        let url = Url::parse("http://127.0.0.1:5000/?code=abc&state=evil").unwrap();
        let err = parse_callback_params(&url, "expected").unwrap_err();
        match err {
            AuthError::OAuthFlow(m) => assert!(m.contains("state mismatch")),
            other => panic!("expected OAuthFlow, got {other:?}"),
        }
    }

    #[test]
    fn parse_callback_errors_when_error_param_is_present() {
        let url = Url::parse("http://127.0.0.1:5000/?error=access_denied&state=expected").unwrap();
        let err = parse_callback_params(&url, "expected").unwrap_err();
        match err {
            AuthError::OAuthFlow(m) => assert!(m.contains("access_denied")),
            other => panic!("expected OAuthFlow, got {other:?}"),
        }
    }

    #[test]
    fn parse_callback_errors_when_code_is_missing() {
        let url = Url::parse("http://127.0.0.1:5000/?state=expected").unwrap();
        let err = parse_callback_params(&url, "expected").unwrap_err();
        match err {
            AuthError::OAuthFlow(m) => assert!(m.contains("missing code")),
            other => panic!("expected OAuthFlow, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn fetch_email_returns_email_from_userinfo() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/userinfo"))
            .and(header("authorization", "Bearer ya29.access"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "email": "user@example.com",
                "sub": "1234",
            })))
            .mount(&server)
            .await;

        let email = fetch_email(&format!("{}/userinfo", server.uri()), "ya29.access")
            .await
            .unwrap();
        assert_eq!(email, "user@example.com");
    }
}
