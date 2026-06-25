use std::collections::VecDeque;
use std::sync::Mutex;

use chrono::Utc;
use url::Url;
use wiremock::matchers::{body_string_contains, header, method, path};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use super::error::AuthError;
use super::login::{
    build_authorize_url, exchange_code, fetch_email, parse_callback_params, poll_device_token,
    render_device_authorization_prompt, request_device_authorization, DeviceAuthorization,
    DEFAULT_DEVICE_LOGIN_SCOPES, GOOGLE_AUTH_URL,
};

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
                ResponseTemplate::new(500).set_body_string("device token mock response exhausted")
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
    assert_eq!(
        pairs.get("redirect_uri").unwrap(),
        "http://127.0.0.1:54321/"
    );
    assert_eq!(pairs.get("response_type").unwrap(), "code");
    assert_eq!(pairs.get("scope").unwrap(), "openid email");
    assert_eq!(pairs.get("state").unwrap(), "state-xyz");
    assert_eq!(pairs.get("access_type").unwrap(), "offline");
    assert_eq!(pairs.get("include_granted_scopes").unwrap(), "true");
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
    assert_eq!(
        authorization.verification_url,
        "https://www.google.com/device"
    );
    assert_eq!(authorization.expires_in, 1800);
    assert_eq!(authorization.interval, 7);
}

#[tokio::test]
async fn request_device_authorization_explains_invalid_client_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/device/code"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_client",
            "error_description": "Invalid client type.",
        })))
        .mount(&server)
        .await;

    let err = request_device_authorization(
        &format!("{}/device/code", server.uri()),
        "client-123",
        DEFAULT_DEVICE_LOGIN_SCOPES,
    )
    .await
    .unwrap_err();

    match err {
        AuthError::OAuthFlow(msg) => assert_device_client_type_guidance(&msg),
        other => panic!("expected OAuthFlow, got {other:?}"),
    }
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
async fn poll_device_token_explains_invalid_client_type() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/token"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "error": "invalid_client",
            "error_description": "Invalid client type.",
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
        AuthError::OAuthFlow(msg) => assert_device_client_type_guidance(&msg),
        other => panic!("expected OAuthFlow, got {other:?}"),
    }
}

fn assert_device_client_type_guidance(msg: &str) {
    assert!(msg.contains("Invalid client type"));
    assert!(msg.contains("TVs and Limited Input devices"));
    assert!(msg.contains("goog auth setup"));
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

    let err = exchange_code(&format!("{}/token", server.uri()), "c", "s", "r", "code")
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
