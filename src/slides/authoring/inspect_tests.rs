use std::collections::VecDeque;
use std::io::Cursor;
use std::sync::Mutex;
use std::time::Duration as StdDuration;

use chrono::{Duration, Utc};
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, Request, Respond, ResponseTemplate};

use super::inspect::{inspect_deck, InspectDeckRequest};
use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::testing::MemoryStore;
use crate::drive::DRIVE_SCOPE;
use crate::slides::SLIDES_SCOPE;

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token(
            "alice@example.com",
            &Token {
                access_token: "slides-drive-access".into(),
                refresh_token: "refresh-123".into(),
                expiry: Utc::now() + Duration::hours(1),
                scopes: vec![SLIDES_SCOPE.into(), DRIVE_SCOPE.into()],
            },
        )
        .unwrap();
    AuthClient::from_config(
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
        },
        store,
        None,
    )
    .unwrap()
}

fn thumbnail_png() -> Vec<u8> {
    let image = RgbaImage::from_pixel(16, 9, Rgba([40, 90, 180, 255]));
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();
    bytes.into_inner()
}

struct ResponseSequence {
    responses: Mutex<VecDeque<ResponseTemplate>>,
}

impl ResponseSequence {
    fn new(responses: Vec<ResponseTemplate>) -> Self {
        Self {
            responses: Mutex::new(responses.into()),
        }
    }
}

impl Respond for ResponseSequence {
    fn respond(&self, _request: &Request) -> ResponseTemplate {
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .unwrap_or_else(|| ResponseTemplate::new(500).set_body_string("response exhausted"))
    }
}

#[tokio::test]
async fn inspect_deck_creates_a_complete_private_qa_bundle() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slides/v1/presentations/presentation-123"))
        .and(header("authorization", "Bearer slides-drive-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "title": "Quarterly plan",
            "slides": [{
                "objectId": "slide-1",
                "pageElements": [{
                    "objectId": "title-1",
                    "shape": {
                        "text": {
                            "textElements": [{
                                "textRun": { "content": "Quarterly results\n" }
                            }]
                        }
                    }
                }]
            }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/slides/v1/presentations/presentation-123/pages/slide-1/thumbnail",
        ))
        .and(header("authorization", "Bearer slides-drive-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "width": 16,
            "height": 9,
            "contentUrl": format!("{}/thumbnail-content/slide-1", server.uri())
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/thumbnail-content/slide-1"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(thumbnail_png()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/presentation-123/export"))
        .and(query_param(
            "mimeType",
            "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        ))
        .and(header("authorization", "Bearer slides-drive-access"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"PK\x03\x04pptx"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/presentation-123/export"))
        .and(query_param("mimeType", "application/pdf"))
        .and(header("authorization", "Bearer slides-drive-access"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"%PDF-1.4\npdf"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let qa_dir = temp.path().join("qa");
    let pptx = temp.path().join("deck.pptx");
    let pdf = temp.path().join("deck.pdf");
    let stale_thumbnail = qa_dir.join("thumbnails/slide-99.png");
    std::fs::create_dir_all(stale_thumbnail.parent().unwrap()).unwrap();
    std::fs::write(&stale_thumbnail, b"stale thumbnail").unwrap();
    let mut request = InspectDeckRequest::new("presentation-123", &qa_dir);
    request.export_pptx = Some(pptx.clone());
    request.export_pdf = Some(pdf.clone());
    request.presentations_url = Some(format!("{}/slides/v1/presentations", server.uri()));
    request.drive_files_url = Some(format!("{}/drive/v3/files", server.uri()));
    let store = MemoryStore::default();
    let client = test_client(&store);

    let report = inspect_deck(&client, &request).await.unwrap();

    assert_eq!(report.report_version, 1);
    assert_eq!(report.result, "success");
    assert_eq!(report.presentation_id, "presentation-123");
    assert_eq!(report.title, "Quarterly plan");
    assert_eq!(report.slides.len(), 1);
    assert_eq!(report.slides[0].object_id, "slide-1");
    assert_eq!(report.slides[0].element_count, 1);
    assert_eq!(report.slides[0].visible_text, ["Quarterly results"]);
    assert!(std::fs::read(&report.slides[0].thumbnail)
        .unwrap()
        .starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(std::fs::read(&report.artifacts.montage)
        .unwrap()
        .starts_with(b"\x89PNG\r\n\x1a\n"));
    assert!(std::fs::read(&pptx).unwrap().starts_with(b"PK\x03\x04"));
    assert!(std::fs::read(&pdf).unwrap().starts_with(b"%PDF-"));
    assert!(!stale_thumbnail.exists());

    let saved_report: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&report.artifacts.report).unwrap()).unwrap();
    assert_eq!(saved_report["reportVersion"], 1);
    assert_eq!(
        saved_report["slides"][0]["visibleText"][0],
        "Quarterly results"
    );

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&qa_dir).unwrap().permissions().mode() & 0o777,
            0o700
        );
        assert_eq!(
            std::fs::metadata(&report.artifacts.report)
                .unwrap()
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}

#[tokio::test]
async fn inspect_deck_retries_a_thumbnail_missing_during_google_consistency_delay() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/slides/v1/presentations/presentation-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "presentationId": "presentation-123",
            "title": "Eventually consistent deck",
            "slides": [{ "objectId": "slide-1" }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path(
            "/slides/v1/presentations/presentation-123/pages/slide-1/thumbnail",
        ))
        .respond_with(ResponseSequence::new(vec![
            ResponseTemplate::new(404),
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "width": 16,
                "height": 9,
                "contentUrl": format!("{}/thumbnail-content/slide-1", server.uri())
            })),
        ]))
        .expect(2)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/thumbnail-content/slide-1"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(thumbnail_png()))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let mut request = InspectDeckRequest::new("presentation-123", temp.path().join("qa"));
    request.presentations_url = Some(format!("{}/slides/v1/presentations", server.uri()));
    request.consistency_timeout = StdDuration::from_secs(1);
    let store = MemoryStore::default();
    let client = test_client(&store);

    let report = inspect_deck(&client, &request).await.unwrap();

    assert_eq!(report.slides.len(), 1);
    assert!(report.slides[0].thumbnail.exists());
}
