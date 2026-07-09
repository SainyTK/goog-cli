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
use crate::cli::{
    SlidesImageReplaceMethod, SlidesLineCategory, SlidesPredefinedLayout, SlidesShapeType,
    SlidesZOrderOperation,
};
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
async fn run_slide_create_sends_create_slide_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "createSlide": {
                        "slideLayoutReference": {
                            "predefinedLayout": "TITLE_AND_BODY"
                        },
                        "objectId": "slide-2",
                        "insertionIndex": 1
                    }
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "createSlide": {
                        "objectId": "slide-2"
                    }
                }
            ]
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

    run_slide_create_unified_to(
        &test_config(),
        &store,
        None,
        SlideCreateRequest {
            presentation_id: "presentation-123".into(),
            object_id: Some("slide-2".into()),
            insertion_index: Some(1),
            layout: SlidesPredefinedLayout::TitleAndBody,
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"createSlide\":{\"objectId\":\"slide-2\"}}]}\n"
    );
}

#[tokio::test]
async fn run_slide_delete_sends_delete_object_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "deleteObject": {
                        "objectId": "slide-2"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_slide_delete_unified_to(
        &test_config(),
        &store,
        None,
        SlideDeleteRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-2".into(),
        },
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
async fn run_object_delete_sends_delete_object_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "deleteObject": {
                        "objectId": "box-1"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_delete_unified_to(
        &test_config(),
        &store,
        None,
        ObjectDeleteRequest {
            presentation_id: "presentation-123".into(),
            object_id: "box-1".into(),
        },
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
async fn run_object_move_sends_update_transform_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updatePageElementTransform": {
                        "objectId": "box-1",
                        "applyMode": "ABSOLUTE",
                        "transform": {
                            "scaleX": 1.5,
                            "scaleY": 0.75,
                            "translateX": 120.0,
                            "translateY": 240.0,
                            "unit": "PT"
                        }
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_move_unified_to(
        &test_config(),
        &store,
        None,
        ObjectMoveRequest {
            presentation_id: "presentation-123".into(),
            object_id: "box-1".into(),
            x: 120.0,
            y: 240.0,
            scale_x: 1.5,
            scale_y: 0.75,
        },
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
async fn run_object_order_sends_update_z_order_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updatePageElementsZOrder": {
                        "pageElementObjectIds": ["box-1", "image-1"],
                        "operation": "BRING_TO_FRONT"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_order_unified_to(
        &test_config(),
        &store,
        None,
        ObjectOrderRequest {
            presentation_id: "presentation-123".into(),
            object_ids: vec!["box-1".into(), "image-1".into()],
            operation: SlidesZOrderOperation::BringToFront,
        },
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
async fn run_object_style_sends_update_shape_properties_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updateShapeProperties": {
                        "objectId": "shape-1",
                        "shapeProperties": {
                            "shapeBackgroundFill": {
                                "solidFill": {
                                    "color": {
                                        "rgbColor": {
                                            "red": 26.0 / 255.0,
                                            "green": 115.0 / 255.0,
                                            "blue": 232.0 / 255.0
                                        }
                                    }
                                }
                            },
                            "outline": {
                                "outlineFill": {
                                    "solidFill": {
                                        "color": {
                                            "rgbColor": {
                                                "red": 32.0 / 255.0,
                                                "green": 33.0 / 255.0,
                                                "blue": 36.0 / 255.0
                                            }
                                        }
                                    }
                                },
                                "weight": {
                                    "magnitude": 2.0,
                                    "unit": "PT"
                                }
                            }
                        },
                        "fields": "shapeBackgroundFill.solidFill.color,outline.outlineFill.solidFill.color,outline.weight"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_style_unified_to(
        &test_config(),
        &store,
        None,
        ObjectStyleRequest {
            presentation_id: "presentation-123".into(),
            object_id: "shape-1".into(),
            fill_color: Some("#1a73e8".into()),
            outline_color: Some("202124".into()),
            outline_weight: Some(2.0),
        },
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

#[test]
fn object_style_requires_at_least_one_style_flag() {
    let err = build_object_style_batch_update(ObjectStyleRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        fill_color: None,
        outline_color: None,
        outline_weight: None,
    })
    .unwrap_err();

    assert!(err.to_string().contains("at least one style flag"));
}

#[test]
fn object_style_rejects_malformed_hex_color() {
    let err = build_object_style_batch_update(ObjectStyleRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        fill_color: Some("blue".into()),
        outline_color: None,
        outline_weight: None,
    })
    .unwrap_err();

    assert!(err.to_string().contains("6-digit hex"));
}

#[tokio::test]
async fn run_object_text_style_sends_update_text_style_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updateTextStyle": {
                        "objectId": "shape-1",
                        "style": {
                            "foregroundColor": {
                                "opaqueColor": {
                                    "rgbColor": {
                                        "red": 26.0 / 255.0,
                                        "green": 115.0 / 255.0,
                                        "blue": 232.0 / 255.0
                                    }
                                }
                            },
                            "fontFamily": "Georgia",
                            "fontSize": {
                                "magnitude": 18.0,
                                "unit": "PT"
                            },
                            "bold": true,
                            "italic": false,
                            "underline": true
                        },
                        "textRange": {
                            "type": "ALL"
                        },
                        "fields": "foregroundColor,fontFamily,fontSize,bold,italic,underline"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_text_style_unified_to(
        &test_config(),
        &store,
        None,
        ObjectTextStyleRequest {
            presentation_id: "presentation-123".into(),
            object_id: "shape-1".into(),
            color: Some("#1a73e8".into()),
            font_family: Some("Georgia".into()),
            font_size: Some(18.0),
            bold: Some(true),
            italic: Some(false),
            underline: Some(true),
            start_index: None,
            end_index: None,
        },
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

#[test]
fn object_text_style_requires_at_least_one_style_flag() {
    let err = build_object_text_style_batch_update(ObjectTextStyleRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        color: None,
        font_family: None,
        font_size: None,
        bold: None,
        italic: None,
        underline: None,
        start_index: None,
        end_index: None,
    })
    .unwrap_err();

    assert!(err.to_string().contains("at least one text style flag"));
}

#[test]
fn object_text_style_can_style_fixed_range() {
    let request_body = build_object_text_style_batch_update(ObjectTextStyleRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        color: None,
        font_family: None,
        font_size: None,
        bold: Some(true),
        italic: None,
        underline: None,
        start_index: Some(2),
        end_index: Some(9),
    })
    .unwrap();

    assert_eq!(
        request_body,
        serde_json::json!({
            "requests": [
                {
                    "updateTextStyle": {
                        "objectId": "shape-1",
                        "style": {
                            "bold": true
                        },
                        "textRange": {
                            "type": "FIXED_RANGE",
                            "startIndex": 2,
                            "endIndex": 9
                        },
                        "fields": "bold"
                    }
                }
            ]
        })
    );
}

#[test]
fn object_text_style_rejects_partial_or_backwards_range() {
    let err = build_object_text_style_batch_update(ObjectTextStyleRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        color: None,
        font_family: None,
        font_size: None,
        bold: Some(true),
        italic: None,
        underline: None,
        start_index: Some(9),
        end_index: Some(2),
    })
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));

    let err = build_object_text_style_batch_update(ObjectTextStyleRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        color: None,
        font_family: None,
        font_size: None,
        bold: Some(true),
        italic: None,
        underline: None,
        start_index: Some(2),
        end_index: None,
    })
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("use both --start-index and --end-index"));
}

#[test]
fn object_text_style_rejects_malformed_hex_color() {
    let err = build_object_text_style_batch_update(ObjectTextStyleRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        color: Some("blue".into()),
        font_family: None,
        font_size: None,
        bold: None,
        italic: None,
        underline: None,
        start_index: None,
        end_index: None,
    })
    .unwrap_err();

    assert!(err.to_string().contains("6-digit hex"));
}

#[tokio::test]
async fn run_object_insert_text_sends_insert_text_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "insertText": {
                        "objectId": "shape-1",
                        "text": "Quarterly plan",
                        "insertionIndex": 3
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_insert_text_unified_to(
        &test_config(),
        &store,
        None,
        ObjectInsertTextRequest {
            presentation_id: "presentation-123".into(),
            object_id: "shape-1".into(),
            text: "Quarterly plan".into(),
            index: 3,
        },
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

#[test]
fn object_insert_text_rejects_empty_text() {
    let err = build_object_insert_text_batch_update(ObjectInsertTextRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        text: String::new(),
        index: 0,
    })
    .unwrap_err();

    assert!(err.to_string().contains("--text must not be empty"));
}

#[tokio::test]
async fn run_object_delete_text_sends_delete_text_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "deleteText": {
                        "objectId": "shape-1",
                        "textRange": {
                            "type": "FIXED_RANGE",
                            "startIndex": 3,
                            "endIndex": 12
                        }
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_delete_text_unified_to(
        &test_config(),
        &store,
        None,
        ObjectDeleteTextRequest {
            presentation_id: "presentation-123".into(),
            object_id: "shape-1".into(),
            all: false,
            start_index: Some(3),
            end_index: Some(12),
        },
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

#[test]
fn object_delete_text_can_delete_all_text() {
    let request = build_object_delete_text_batch_update(ObjectDeleteTextRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        all: true,
        start_index: None,
        end_index: None,
    })
    .unwrap();

    assert_eq!(
        request,
        serde_json::json!({
            "requests": [
                {
                    "deleteText": {
                        "objectId": "shape-1",
                        "textRange": {
                            "type": "ALL"
                        }
                    }
                }
            ]
        })
    );
}

#[test]
fn object_delete_text_requires_all_or_fixed_range() {
    let err = build_object_delete_text_batch_update(ObjectDeleteTextRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        all: false,
        start_index: Some(3),
        end_index: None,
    })
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("use either --all or both --start-index and --end-index"));
}

#[test]
fn object_delete_text_rejects_empty_or_backwards_range() {
    let err = build_object_delete_text_batch_update(ObjectDeleteTextRequest {
        presentation_id: "presentation-123".into(),
        object_id: "shape-1".into(),
        all: false,
        start_index: Some(12),
        end_index: Some(12),
    })
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("--end-index must be greater than --start-index"));
}

#[tokio::test]
async fn run_slide_duplicate_sends_duplicate_object_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "duplicateObject": {
                        "objectId": "slide-1",
                        "objectIds": {
                            "slide-1": "slide-2"
                        }
                    }
                },
                {
                    "updateSlidesPosition": {
                        "slideObjectIds": ["slide-2"],
                        "insertionIndex": 1
                    }
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "duplicateObject": {
                        "objectId": "slide-2"
                    }
                },
                {}
            ]
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

    run_slide_duplicate_unified_to(
        &test_config(),
        &store,
        None,
        SlideDuplicateRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            object_id: Some("slide-2".into()),
            insertion_index: Some(1),
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"duplicateObject\":{\"objectId\":\"slide-2\"}},{}]}\n"
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

#[tokio::test]
async fn run_shape_sends_create_shape_request() {
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
                        "objectId": "shape-1",
                        "shapeType": "ROUND_RECTANGLE",
                        "elementProperties": {
                            "pageObjectId": "slide-1",
                            "size": {
                                "width": {
                                    "magnitude": 300.0,
                                    "unit": "PT"
                                },
                                "height": {
                                    "magnitude": 180.0,
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
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "createShape": {
                        "objectId": "shape-1"
                    }
                }
            ]
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

    run_shape_unified_to(
        &test_config(),
        &store,
        None,
        ShapeRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            shape_type: SlidesShapeType::RoundRectangle,
            object_id: Some("shape-1".into()),
            x: 48.0,
            y: 96.0,
            width: 300.0,
            height: 180.0,
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"createShape\":{\"objectId\":\"shape-1\"}}]}\n"
    );
}

#[tokio::test]
async fn run_line_sends_create_line_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "createLine": {
                        "objectId": "line-1",
                        "category": "CURVED",
                        "elementProperties": {
                            "pageObjectId": "slide-1",
                            "size": {
                                "width": {
                                    "magnitude": 300.0,
                                    "unit": "PT"
                                },
                                "height": {
                                    "magnitude": 120.0,
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
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "createLine": {
                        "objectId": "line-1"
                    }
                }
            ]
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

    run_line_unified_to(
        &test_config(),
        &store,
        None,
        LineRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            category: SlidesLineCategory::Curved,
            object_id: Some("line-1".into()),
            x: 48.0,
            y: 96.0,
            width: 300.0,
            height: 120.0,
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"createLine\":{\"objectId\":\"line-1\"}}]}\n"
    );
}

#[tokio::test]
async fn run_slide_background_sends_update_page_properties_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updatePageProperties": {
                        "objectId": "slide-1",
                        "pageProperties": {
                            "pageBackgroundFill": {
                                "solidFill": {
                                    "color": {
                                        "rgbColor": {
                                            "red": 251.0 / 255.0,
                                            "green": 188.0 / 255.0,
                                            "blue": 4.0 / 255.0
                                        }
                                    }
                                }
                            }
                        },
                        "fields": "pageBackgroundFill.solidFill.color"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_slide_background_unified_to(
        &test_config(),
        &store,
        None,
        SlideBackgroundRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            color: "#fbbc04".into(),
        },
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

#[test]
fn build_slide_background_rejects_non_hex_color() {
    let err = build_slide_background_batch_update(SlideBackgroundRequest {
        presentation_id: "presentation-123".into(),
        page_id: "slide-1".into(),
        color: "yellow".into(),
    })
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("color must be a 6-digit hex value like #1a73e8"));
}

#[tokio::test]
async fn run_image_sends_create_image_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "createImage": {
                        "objectId": "image-1",
                        "url": "https://example.com/chart.png",
                        "elementProperties": {
                            "pageObjectId": "slide-1",
                            "size": {
                                "width": {
                                    "magnitude": 300.0,
                                    "unit": "PT"
                                },
                                "height": {
                                    "magnitude": 180.0,
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
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "createImage": {
                        "objectId": "image-1"
                    }
                }
            ]
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

    run_image_unified_to(
        &test_config(),
        &store,
        None,
        ImageRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            url: "https://example.com/chart.png".into(),
            object_id: Some("image-1".into()),
            x: 48.0,
            y: 96.0,
            width: 300.0,
            height: 180.0,
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"createImage\":{\"objectId\":\"image-1\"}}]}\n"
    );
}

#[tokio::test]
async fn run_table_sends_create_table_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "createTable": {
                        "objectId": "table-1",
                        "rows": 3,
                        "columns": 4,
                        "elementProperties": {
                            "pageObjectId": "slide-1",
                            "size": {
                                "width": {
                                    "magnitude": 300.0,
                                    "unit": "PT"
                                },
                                "height": {
                                    "magnitude": 180.0,
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
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "createTable": {
                        "objectId": "table-1"
                    }
                }
            ]
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

    run_table_unified_to(
        &test_config(),
        &store,
        None,
        TableRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            rows: 3,
            columns: 4,
            object_id: Some("table-1".into()),
            x: 48.0,
            y: 96.0,
            width: 300.0,
            height: 180.0,
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"createTable\":{\"objectId\":\"table-1\"}}]}\n"
    );
}

#[tokio::test]
async fn run_table_rejects_zero_dimensions() {
    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &slides_token("slides-access"))
        .unwrap();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    let err = run_table_unified_to(
        &test_config(),
        &store,
        None,
        TableRequest {
            presentation_id: "presentation-123".into(),
            page_id: "slide-1".into(),
            rows: 0,
            columns: 4,
            object_id: Some("table-1".into()),
            x: 48.0,
            y: 96.0,
            width: 300.0,
            height: 180.0,
        },
        &mut out,
        Some("https://example.invalid/slides/v1/presentations"),
        Some(&state_path),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("--rows must be greater than zero"));
}

#[tokio::test]
async fn run_object_alt_text_sends_update_page_element_alt_text_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "updatePageElementAltText": {
                        "objectId": "image-1",
                        "title": "Pipeline chart",
                        "description": "Bar chart showing qualified pipeline by stage"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_alt_text_unified_to(
        &test_config(),
        &store,
        None,
        ObjectAltTextRequest {
            presentation_id: "presentation-123".into(),
            object_id: "image-1".into(),
            title: Some("Pipeline chart".into()),
            description: Some("Bar chart showing qualified pipeline by stage".into()),
        },
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

#[test]
fn build_object_alt_text_rejects_empty_update() {
    let err = build_object_alt_text_batch_update(ObjectAltTextRequest {
        presentation_id: "presentation-123".into(),
        object_id: "image-1".into(),
        title: None,
        description: None,
    })
    .unwrap_err();

    assert!(err.to_string().contains("at least one alt text flag"));
}

#[tokio::test]
async fn run_object_replace_image_sends_replace_image_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "replaceImage": {
                        "imageObjectId": "image-1",
                        "url": "https://example.com/new-chart.png",
                        "imageReplaceMethod": "CENTER_CROP"
                    }
                }
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
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_object_replace_image_unified_to(
        &test_config(),
        &store,
        None,
        ObjectReplaceImageRequest {
            presentation_id: "presentation-123".into(),
            image_id: "image-1".into(),
            url: "https://example.com/new-chart.png".into(),
            method: SlidesImageReplaceMethod::CenterCrop,
        },
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
async fn run_table_fill_sends_insert_text_requests_for_cells() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "insertText": {
                        "objectId": "table-1",
                        "cellLocation": {
                            "rowIndex": 1,
                            "columnIndex": 2
                        },
                        "insertionIndex": 0,
                        "text": "Metric"
                    }
                },
                {
                    "insertText": {
                        "objectId": "table-1",
                        "cellLocation": {
                            "rowIndex": 1,
                            "columnIndex": 3
                        },
                        "insertionIndex": 0,
                        "text": "Value"
                    }
                },
                {
                    "insertText": {
                        "objectId": "table-1",
                        "cellLocation": {
                            "rowIndex": 2,
                            "columnIndex": 2
                        },
                        "insertionIndex": 0,
                        "text": "ARR"
                    }
                },
                {
                    "insertText": {
                        "objectId": "table-1",
                        "cellLocation": {
                            "rowIndex": 2,
                            "columnIndex": 3
                        },
                        "insertionIndex": 0,
                        "text": "1200000"
                    }
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [{}, {}, {}, {}]
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

    run_table_fill_unified_to(
        &test_config(),
        &store,
        None,
        TableFillRequest {
            presentation_id: "presentation-123".into(),
            table_id: "table-1".into(),
            rows: vec!["Metric|Value".into(), "ARR|1200000".into()],
            delimiter: "|".into(),
            start_row: 1,
            start_column: 2,
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{},{},{},{}]}\n"
    );
}

#[test]
fn build_table_fill_skips_empty_cells_by_default() {
    let batch_update = build_table_fill_batch_update(TableFillRequest {
        presentation_id: "presentation-123".into(),
        table_id: "table-1".into(),
        rows: vec!["A||C".into()],
        delimiter: "|".into(),
        start_row: 0,
        start_column: 0,
    })
    .unwrap();

    assert_eq!(batch_update["requests"].as_array().unwrap().len(), 2);
    assert_eq!(
        batch_update["requests"][1]["insertText"]["cellLocation"]["columnIndex"],
        2
    );
}

#[test]
fn build_table_fill_rejects_empty_delimiter() {
    let err = build_table_fill_batch_update(TableFillRequest {
        presentation_id: "presentation-123".into(),
        table_id: "table-1".into(),
        rows: vec!["A|B".into()],
        delimiter: String::new(),
        start_row: 0,
        start_column: 0,
    })
    .unwrap_err();

    assert!(err.to_string().contains("--delimiter must not be empty"));
}

#[tokio::test]
async fn run_replace_text_sends_replace_all_text_request() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "replaceAllText": {
                        "containsText": {
                            "text": "{{title}}",
                            "matchCase": true
                        },
                        "replaceText": "Quarterly plan",
                        "pageObjectIds": ["slide-1", "slide-2"]
                    }
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "replaceAllText": {
                        "occurrencesChanged": 3
                    }
                }
            ]
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

    run_replace_text_unified_to(
        &test_config(),
        &store,
        None,
        ReplaceTextRequest {
            presentation_id: "presentation-123".into(),
            find: "{{title}}".into(),
            replacement: "Quarterly plan".into(),
            match_case: true,
            page_ids: vec!["slide-1".into(), "slide-2".into()],
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"replaceAllText\":{\"occurrencesChanged\":3}}]}\n"
    );
}

#[tokio::test]
async fn run_replace_text_omits_page_object_ids_without_page_scope() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path(
            "/slides/v1/presentations/presentation-123:batchUpdate",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(body_json(serde_json::json!({
            "requests": [
                {
                    "replaceAllText": {
                        "containsText": {
                            "text": "draft",
                            "matchCase": false
                        },
                        "replaceText": "final"
                    }
                }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "replies": [
                {
                    "replaceAllText": {
                        "occurrencesChanged": 1
                    }
                }
            ]
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

    run_replace_text_unified_to(
        &test_config(),
        &store,
        None,
        ReplaceTextRequest {
            presentation_id: "presentation-123".into(),
            find: "draft".into(),
            replacement: "final".into(),
            match_case: false,
            page_ids: vec![],
        },
        &mut out,
        Some(&presentations_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"presentationId\":\"presentation-123\",\"replies\":[{\"replaceAllText\":{\"occurrencesChanged\":1}}]}\n"
    );
}

#[tokio::test]
async fn run_replace_text_rejects_empty_find_text() {
    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &slides_token("slides-access"))
        .unwrap();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    let err = run_replace_text_unified_to(
        &test_config(),
        &store,
        None,
        ReplaceTextRequest {
            presentation_id: "presentation-123".into(),
            find: String::new(),
            replacement: "final".into(),
            match_case: false,
            page_ids: vec![],
        },
        &mut out,
        Some("https://example.invalid/slides/v1/presentations"),
        Some(&state_path),
    )
    .await
    .unwrap_err();

    assert!(err.to_string().contains("--find must not be empty"));
}
