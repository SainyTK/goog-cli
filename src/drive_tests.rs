use std::sync::{Mutex, MutexGuard};

use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Match, Request};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, SettingsConfig};
use crate::auth::testing::MemoryStore;
use crate::drive::*;

const SINGLE_PAGE_RESPONSE: &str = include_str!("../tests/fixtures/drive/files_page_single.json");
const EMPTY_PAGE_WITH_TOKEN_RESPONSE: &str =
    include_str!("../tests/fixtures/drive/files_page_empty_with_token.json");

static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

struct CurrentDirGuard {
    original: std::path::PathBuf,
    _lock: MutexGuard<'static, ()>,
}

impl CurrentDirGuard {
    fn enter(path: impl AsRef<std::path::Path>) -> Self {
        let lock = CURRENT_DIR_LOCK.lock().unwrap();
        let original = std::env::current_dir().unwrap();
        std::env::set_current_dir(path).unwrap();
        Self {
            original,
            _lock: lock,
        }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        std::env::set_current_dir(&self.original).unwrap();
    }
}

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

fn drive_token() -> Token {
    Token {
        access_token: "drive-access".into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![DRIVE_SCOPE.into()],
    }
}

fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
    store
        .save_token("alice@example.com", &drive_token())
        .unwrap();
    AuthClient::from_config(test_config(), store, None).unwrap()
}

#[tokio::test]
async fn list_files_deserializes_a_single_page_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "modifiedTime desc"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        ListFilesOptions::new(50).with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(page.next_page_token, None);
    assert_eq!(
        page.files,
        vec![DriveFile {
            name: "Roadmap".into(),
            id: "file-1".into(),
            mime_type: "application/vnd.google-apps.document".into(),
            modified_time: "2026-06-24T10:15:00.000Z".into(),
        }]
    );
}

#[tokio::test]
async fn list_files_sends_next_page_token_and_returns_next_page_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "25"))
        .and(query_param("pageToken", "token-1"))
        .respond_with(ResponseTemplate::new(200).set_body_string(EMPTY_PAGE_WITH_TOKEN_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::new(25)
        .with_page_token("token-1")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(page.next_page_token.as_deref(), Some("token-2"));
    assert!(page.files.is_empty());
}

#[tokio::test]
async fn list_files_returns_drive_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        ListFilesOptions::new(50).with_files_url(format!("{}/drive/v3/files", server.uri()));

    let err = list_files(&client, &options).await.unwrap_err();

    assert!(matches!(err, DriveError::NotFound));
}

#[tokio::test]
async fn list_files_returns_drive_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        ListFilesOptions::new(50).with_files_url(format!("{}/drive/v3/files", server.uri()));

    let err = list_files(&client, &options).await.unwrap_err();

    assert!(matches!(err, DriveError::PermissionDenied));
}

#[tokio::test]
async fn download_streams_binary_response_to_explicit_output_path() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/file-1"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("alt", "media"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello\x00drive".to_vec()))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("download.bin");
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = DownloadFileOptions::new("file-1")
        .with_output(output.clone())
        .with_files_url(format!("{}/drive/v3/files", server.uri()));
    let mut progress = Vec::new();

    let downloaded = download(&client, &options, |bytes| progress.push(bytes))
        .await
        .unwrap();

    assert_eq!(downloaded.path, output);
    assert_eq!(downloaded.bytes, 11);
    assert_eq!(std::fs::read(downloaded.path).unwrap(), b"hello\x00drive");
    assert_eq!(progress, vec![11]);
}

#[tokio::test]
async fn download_uses_drive_file_name_in_current_directory_by_default() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/file-1"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("fields", "name"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "name": "report.txt"
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/file-1"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("alt", "media"))
        .respond_with(ResponseTemplate::new(200).set_body_string("report contents"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let _current_dir = CurrentDirGuard::enter(temp.path());

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = DownloadFileOptions::new("file-1")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let downloaded = download(&client, &options, |_| ()).await.unwrap();

    assert_eq!(
        downloaded.path.canonicalize().unwrap(),
        temp.path().join("report.txt").canonicalize().unwrap()
    );
    assert_eq!(
        std::fs::read_to_string(downloaded.path).unwrap(),
        "report contents"
    );
}

#[tokio::test]
async fn download_returns_drive_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/missing-file"))
        .and(query_param("alt", "media"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = DownloadFileOptions::new("missing-file")
        .with_output(temp.path().join("missing.bin"))
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let err = download(&client, &options, |_| ()).await.unwrap_err();

    assert!(matches!(err, DriveError::NotFound));
}

#[tokio::test]
async fn download_returns_drive_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/private-file"))
        .and(query_param("alt", "media"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = DownloadFileOptions::new("private-file")
        .with_output(temp.path().join("private.bin"))
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let err = download(&client, &options, |_| ()).await.unwrap_err();

    assert!(matches!(err, DriveError::PermissionDenied));
}

struct BodyContains(&'static [u8]);

impl Match for BodyContains {
    fn matches(&self, request: &Request) -> bool {
        request
            .body
            .windows(self.0.len())
            .any(|chunk| chunk == self.0)
    }
}

#[tokio::test]
async fn upload_small_file_uses_multipart_upload_and_returns_drive_location() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("uploadType", "multipart"))
        .and(query_param("fields", "id,webViewLink"))
        .and(BodyContains(br#""name":"report.txt""#))
        .and(BodyContains(b"hello drive"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file-123",
            "webViewLink": "https://drive.google.com/file/d/file-123/view"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("report.txt");
    std::fs::write(&path, "hello drive").unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = UploadFileOptions::new(&path)
        .with_upload_url(format!("{}/upload/drive/v3/files", server.uri()));
    let mut progress = Vec::new();

    let uploaded = upload(&client, &options, |bytes| progress.push(bytes))
        .await
        .unwrap();

    assert_eq!(uploaded.id, "file-123");
    assert_eq!(
        uploaded.web_view_link,
        "https://drive.google.com/file/d/file-123/view"
    );
    assert_eq!(progress, vec![11]);
}

#[tokio::test]
async fn upload_small_file_includes_parent_folder_in_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload/drive/v3/files"))
        .and(query_param("uploadType", "multipart"))
        .and(BodyContains(br#""parents":["folder-123"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "file-123",
            "webViewLink": "https://drive.google.com/file/d/file-123/view"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("report.txt");
    std::fs::write(&path, "hello drive").unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = UploadFileOptions::new(&path)
        .with_folder("folder-123")
        .with_upload_url(format!("{}/upload/drive/v3/files", server.uri()));

    upload(&client, &options, |_| ()).await.unwrap();
}

#[tokio::test]
async fn upload_large_file_uses_resumable_upload_chunks() {
    let server = MockServer::start().await;
    let session_uri = format!("{}/upload-session", server.uri());
    Mock::given(method("POST"))
        .and(path("/upload/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("uploadType", "resumable"))
        .and(query_param("fields", "id,webViewLink"))
        .and(header("x-upload-content-length", "5242883"))
        .and(BodyContains(br#""name":"large.bin""#))
        .respond_with(ResponseTemplate::new(200).insert_header("Location", session_uri.clone()))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/upload-session"))
        .and(header("authorization", "Bearer drive-access"))
        .and(header("content-range", "bytes 0-5242879/5242883"))
        .respond_with(ResponseTemplate::new(308))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/upload-session"))
        .and(header("authorization", "Bearer drive-access"))
        .and(header("content-range", "bytes 5242880-5242882/5242883"))
        .and(BodyContains(b"end"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "large-file-123",
            "webViewLink": "https://drive.google.com/file/d/large-file-123/view"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("large.bin");
    let mut contents = vec![b'a'; MULTIPART_UPLOAD_LIMIT_BYTES as usize];
    contents.extend_from_slice(b"end");
    std::fs::write(&path, contents).unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = UploadFileOptions::new(&path)
        .with_upload_url(format!("{}/upload/drive/v3/files", server.uri()));
    let mut progress = Vec::new();

    let uploaded = upload(&client, &options, |bytes| progress.push(bytes))
        .await
        .unwrap();

    assert_eq!(uploaded.id, "large-file-123");
    assert_eq!(progress, vec![5242880, 5242883]);
}

#[tokio::test]
async fn upload_returns_drive_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload/drive/v3/files"))
        .and(query_param("uploadType", "multipart"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("report.txt");
    std::fs::write(&path, "hello drive").unwrap();

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = UploadFileOptions::new(&path)
        .with_upload_url(format!("{}/upload/drive/v3/files", server.uri()));

    let err = upload(&client, &options, |_| ()).await.unwrap_err();

    assert!(matches!(err, DriveError::PermissionDenied));
}
