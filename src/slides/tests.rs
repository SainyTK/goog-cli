use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::testing::MemoryStore;

use super::{fetch_page_thumbnail_once, GetPageThumbnailOptions, SLIDES_SCOPE};

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token(
            "alice@example.com",
            &Token {
                access_token: "slides-access".into(),
                refresh_token: "refresh-123".into(),
                expiry: Utc::now() + Duration::hours(1),
                scopes: vec![SLIDES_SCOPE.into()],
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

#[tokio::test]
async fn fetch_page_thumbnail_downloads_large_png_without_forwarding_oauth_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/slides/v1/presentations/presentation-123/pages/slide-1/thumbnail",
        ))
        .and(header("authorization", "Bearer slides-access"))
        .and(query_param("thumbnailProperties.mimeType", "PNG"))
        .and(query_param("thumbnailProperties.thumbnailSize", "LARGE"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "width": 1600,
            "height": 900,
            "contentUrl": format!("{}/thumbnail.png", server.uri())
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/thumbnail.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"\x89PNG\r\n\x1a\nimage".to_vec()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetPageThumbnailOptions::new("presentation-123", "slide-1")
        .allow_insecure_content_url_for_tests()
        .with_presentations_url(format!("{}/slides/v1/presentations", server.uri()));

    let thumbnail = fetch_page_thumbnail_once(&client, &options).await.unwrap();

    assert_eq!(thumbnail.width, 1600);
    assert_eq!(thumbnail.height, 900);
    assert_eq!(thumbnail.bytes.as_ref(), b"\x89PNG\r\n\x1a\nimage");

    let requests = server.received_requests().await.unwrap();
    let image_request = requests
        .iter()
        .find(|request| request.url.path() == "/thumbnail.png")
        .unwrap();
    assert!(!image_request.headers.contains_key("authorization"));
}

#[tokio::test]
async fn fetch_page_thumbnail_rejects_an_empty_image_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/slides/v1/presentations/presentation-123/pages/slide-1/thumbnail",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "width": 1600,
            "height": 900,
            "contentUrl": format!("{}/empty.png", server.uri())
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/empty.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(Vec::new()))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetPageThumbnailOptions::new("presentation-123", "slide-1")
        .allow_insecure_content_url_for_tests()
        .with_presentations_url(format!("{}/slides/v1/presentations", server.uri()));

    let error = fetch_page_thumbnail_once(&client, &options)
        .await
        .unwrap_err();

    assert!(matches!(error, super::SlidesError::InvalidResponse(_)));
}

#[tokio::test]
async fn fetch_page_thumbnail_rejects_an_image_over_the_configured_byte_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/slides/v1/presentations/presentation-123/pages/slide-1/thumbnail",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "width": 4,
            "height": 2,
            "contentUrl": format!("{}/oversized.png", server.uri())
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/oversized.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![0_u8; 9]))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetPageThumbnailOptions::new("presentation-123", "slide-1")
        .with_max_download_bytes(8)
        .allow_insecure_content_url_for_tests()
        .with_presentations_url(format!("{}/slides/v1/presentations", server.uri()));

    let error = fetch_page_thumbnail_once(&client, &options)
        .await
        .unwrap_err();

    assert!(matches!(error, super::SlidesError::InvalidResponse(_)));
}

#[tokio::test]
async fn fetch_page_thumbnail_rejects_metadata_over_the_configured_pixel_limit() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/slides/v1/presentations/presentation-123/pages/slide-1/thumbnail",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "width": 4,
            "height": 2,
            "contentUrl": format!("{}/unfetched.png", server.uri())
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/unfetched.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1_u8]))
        .expect(0)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetPageThumbnailOptions::new("presentation-123", "slide-1")
        .with_max_pixel_count(7)
        .allow_insecure_content_url_for_tests()
        .with_presentations_url(format!("{}/slides/v1/presentations", server.uri()));

    let error = fetch_page_thumbnail_once(&client, &options)
        .await
        .unwrap_err();

    assert!(matches!(error, super::SlidesError::InvalidResponse(_)));
}

#[tokio::test]
async fn fetch_page_thumbnail_rejects_a_non_https_content_url() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(
            "/slides/v1/presentations/presentation-123/pages/slide-1/thumbnail",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "width": 1600,
            "height": 900,
            "contentUrl": format!("{}/unfetched.png", server.uri())
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/unfetched.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(vec![1_u8]))
        .expect(0)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = GetPageThumbnailOptions::new("presentation-123", "slide-1")
        .with_presentations_url(format!("{}/slides/v1/presentations", server.uri()));

    let error = fetch_page_thumbnail_once(&client, &options)
        .await
        .unwrap_err();

    assert!(matches!(error, super::SlidesError::InvalidResponse(_)));
}
