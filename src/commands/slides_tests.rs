use chrono::{Duration, Utc};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::state::{
    load_runtime_state_from_path, resource_key, save_runtime_state_to_path, RuntimeState,
};
use crate::auth::testing::MemoryStore;
use crate::slides::SLIDES_SCOPE;

use super::slides::*;

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

fn slides_token(access_token: &str) -> Token {
    Token {
        access_token: access_token.into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![SLIDES_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &slides_token("slides-access"))
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

fn presentations_url(server: &MockServer) -> String {
    format!("{}/slides/v1/presentations", server.uri())
}

fn write_test_state() -> (tempfile::TempDir, std::path::PathBuf) {
    let state_dir = tempfile::tempdir().unwrap();
    let state_path = state_dir.path().join("auth.json");
    save_runtime_state_to_path(
        &RuntimeState {
            version: crate::auth::state::AUTH_STATE_VERSION,
            active_account: Some("alice@example.com".into()),
            accounts: vec![],
            resource_account_mappings: Default::default(),
        },
        &state_path,
    )
    .unwrap();
    (state_dir, state_path)
}

#[tokio::test]
async fn run_create_prints_presentation_id_and_edit_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/slides/v1/presentations"))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({ "title": "goog-e2e-slides" })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-456",
            "title": "goog-e2e-slides"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();

    run_create_to(
        &client,
        "goog-e2e-slides".into(),
        &mut out,
        Some(&presentations_url(&server)),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "presentation-456\thttps://docs.google.com/presentation/d/presentation-456/edit\n"
    );
}

#[tokio::test]
async fn run_get_unified_falls_back_and_maps_successful_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slides/v1/presentations/presentation-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/slides/v1/presentations/presentation-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "title": "Deck"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &slides_token("alice-access"))
        .unwrap();
    store
        .save_token("bob@example.com", &slides_token("bob-access"))
        .unwrap();
    let config = Config {
        oauth_app: test_config().oauth_app,
        settings: test_config().settings,
        accounts: vec!["alice@example.com".into(), "bob@example.com".into()],
    };
    let (_state_dir, state_path) = write_test_state();

    let mut out = Vec::new();
    run_get_unified_to(
        &config,
        &store,
        None,
        "presentation-123".into(),
        None,
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"title\":\"Deck\"}\n"
    );
    let state = load_runtime_state_from_path(&state_path).unwrap();
    assert_eq!(
        state.account_for_resource(&resource_key("slides", "presentation-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_batch_update_sends_request_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                { "createSlide": { "objectId": "slide-1" } }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [{}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &slides_token("slides-access"))
        .unwrap();
    let mut input = br#"{"requests":[{"createSlide":{"objectId":"slide-1"}}]}"#.as_slice();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_batch_update_unified_to(
        &test_config(),
        &store,
        None,
        "presentation-123".into(),
        "-".into(),
        &mut input,
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{}]}\n"
    );
}

#[tokio::test]
async fn run_text_box_sends_create_shape_and_insert_text_requests() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "createShape": {
                        "objectId": "box-1",
                        "shapeType": "TEXT_BOX",
                        "elementProperties": {
                            "pageObjectId": "slide-1",
                            "size": {
                                "width": {
                                    "magnitude": 300.0,
                                    "unit": "PT"
                                },
                                "height": {
                                    "magnitude": 80.0,
                                    "unit": "PT"
                                }
                            },
                            "transform": {
                                "scaleX": 1.0,
                                "scaleY": 1.0,
                                "translateX": 48.0,
                                "translateY": 96.0,
                                "unit": "PT"
                            }
                        }
                    }
                },
                {
                    "insertText": {
                        "objectId": "box-1",
                        "insertionIndex": 0,
                        "text": "Quarterly plan"
                    }
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [{}, {}]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &slides_token("slides-access"))
        .unwrap();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_text_box_unified_to(
        &test_config(),
        &store,
        None,
        TextBoxRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            text: "Quarterly plan".into(),
            object_id: Some("box-1".into()),
            x: 48.0,
            y: 96.0,
            width: 300.0,
            height: 80.0,
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{},{}]}\n"
    );
}
