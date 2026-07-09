use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Match, Request};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::testing::MemoryStore;
use crate::drive::*;
use crate::test_support::CurrentDirGuard;

const SINGLE_PAGE_RESPONSE: &str = include_str!("../tests/fixtures/drive/files_page_single.json");
const EMPTY_PAGE_WITH_TOKEN_RESPONSE: &str =
    include_str!("../tests/fixtures/drive/files_page_empty_with_token.json");
const FOLDER_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "file-1",
      "name": "Roadmap",
      "parents": ["folder-123"],
      "mimeType": "application/vnd.google-apps.document",
      "modifiedTime": "2026-06-24T10:15:00.000Z"
    }
  ]
}"#;
const DRIVE_FOLDER_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "folder-456",
      "name": "Projects",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T11:15:00.000Z"
    }
  ]
}"#;
const DRIVE_BROWSE_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "folder-456",
      "name": "Projects",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T11:15:00.000Z"
    },
    {
      "id": "file-1",
      "name": "Roadmap",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.document",
      "modifiedTime": "2026-06-24T10:15:00.000Z"
    }
  ]
}"#;
const SHEETS_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "sheet-1",
      "name": "Budget",
      "parents": ["folder-123"],
      "mimeType": "application/vnd.google-apps.spreadsheet",
      "modifiedTime": "2026-06-24T12:15:00.000Z"
    }
  ]
}"#;
const SLIDES_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "presentation-1",
      "name": "Roadshow",
      "parents": ["folder-123"],
      "mimeType": "application/vnd.google-apps.presentation",
      "modifiedTime": "2026-06-24T13:15:00.000Z"
    }
  ]
}"#;

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
        .and(query_param(
            "q",
            "mimeType != 'application/vnd.google-apps.folder'",
        ))
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
            parent_ids: vec![],
            mime_type: "application/vnd.google-apps.document".into(),
            modified_time: "2026-06-24T10:15:00.000Z".into(),
        }]
    );
}

#[tokio::test]
async fn list_files_can_filter_to_files_inside_a_folder() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "modifiedTime desc"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType != 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::new(50)
        .with_folder("folder-123")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(
        page.files,
        vec![DriveFile {
            name: "Roadmap".into(),
            id: "file-1".into(),
            parent_ids: vec!["folder-123".into()],
            mime_type: "application/vnd.google-apps.document".into(),
            modified_time: "2026-06-24T10:15:00.000Z".into(),
        }]
    );
}

#[tokio::test]
async fn list_folders_defaults_to_folders_in_drive_root() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "modifiedTime desc"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .and(query_param(
            "q",
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(DRIVE_FOLDER_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        ListFilesOptions::folders(50).with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(
        page.files,
        vec![DriveFile {
            name: "Projects".into(),
            id: "folder-456".into(),
            parent_ids: vec!["root".into()],
            mime_type: "application/vnd.google-apps.folder".into(),
            modified_time: "2026-06-24T11:15:00.000Z".into(),
        }]
    );
}

#[tokio::test]
async fn list_folders_can_filter_to_child_folders_inside_a_parent() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "modifiedTime desc"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType = 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(DRIVE_FOLDER_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::folders(50)
        .with_folder("folder-123")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(page.files[0].id, "folder-456");
}

#[tokio::test]
async fn list_docs_filters_to_native_google_docs() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "modifiedTime desc"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .and(query_param(
            "q",
            "mimeType = 'application/vnd.google-apps.document'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        ListFilesOptions::docs(50).with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(page.files[0].id, "file-1");
}

#[tokio::test]
async fn list_sheets_can_filter_to_native_google_sheets_inside_a_folder() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "modifiedTime desc"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType = 'application/vnd.google-apps.spreadsheet'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SHEETS_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::sheets(50)
        .with_folder("folder-123")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(page.files[0].id, "sheet-1");
}

#[tokio::test]
async fn list_slides_can_filter_to_native_google_slides_inside_a_folder() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "modifiedTime desc"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType = 'application/vnd.google-apps.presentation'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SLIDES_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::slides(50)
        .with_folder("folder-123")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(page.files[0].id, "presentation-1");
}

#[tokio::test]
async fn browse_files_defaults_to_drive_root_without_mime_filter() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("orderBy", "name"))
        .and(query_param("fields", DRIVE_FILES_FIELDS))
        .and(query_param("q", "'root' in parents"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DRIVE_BROWSE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        ListFilesOptions::browse(50).with_files_url(format!("{}/drive/v3/files", server.uri()));

    let page = list_files(&client, &options).await.unwrap();

    assert_eq!(page.files[0].id, "folder-456");
    assert_eq!(page.files[1].id, "file-1");
}

#[tokio::test]
async fn browse_files_can_filter_to_children_inside_a_folder() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("q", "'folder-123' in parents"))
        .respond_with(ResponseTemplate::new(200).set_body_string(DRIVE_BROWSE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::browse(50)
        .with_folder("folder-123")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    list_files(&client, &options).await.unwrap();
}

#[tokio::test]
async fn list_files_escapes_folder_id_in_query_literal() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param(
            "q",
            r#"'folder\\\'123' in parents and mimeType != 'application/vnd.google-apps.folder'"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::new(50)
        .with_folder(r#"folder\'123"#)
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    list_files(&client, &options).await.unwrap();
}

#[tokio::test]
async fn list_folders_escapes_parent_id_in_query_literal() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param(
            "q",
            r#"'folder\\\'123' in parents and mimeType = 'application/vnd.google-apps.folder'"#,
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(DRIVE_FOLDER_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = ListFilesOptions::folders(50)
        .with_folder(r#"folder\'123"#)
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    list_files(&client, &options).await.unwrap();
}

#[tokio::test]
async fn list_files_sends_next_page_token_and_returns_next_page_token() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "25"))
        .and(query_param("pageToken", "token-1"))
        .and(query_param(
            "q",
            "mimeType != 'application/vnd.google-apps.folder'",
        ))
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
async fn download_requests_supports_all_drives_for_shared_drive_files() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/shared-file-1"))
        .and(query_param("alt", "media"))
        .and(query_param("supportsAllDrives", "true"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"shared".to_vec()))
        .expect(1)
        .mount(&server)
        .await;

    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("shared.bin");
    let store = MemoryStore::default();
    let client = test_client(&store);
    let options = DownloadFileOptions::new("shared-file-1")
        .with_output(output.clone())
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    download(&client, &options, |_| {}).await.unwrap();

    assert_eq!(std::fs::read(output).unwrap(), b"shared");
}

#[tokio::test]
async fn list_files_requests_items_from_all_drives() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("supportsAllDrives", "true"))
        .and(query_param("includeItemsFromAllDrives", "true"))
        .and(query_param("corpora", "allDrives"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let options =
        ListFilesOptions::new(50).with_files_url(format!("{}/drive/v3/files", server.uri()));

    list_files(&client, &options).await.unwrap();
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

struct BodyLength(usize);

impl Match for BodyLength {
    fn matches(&self, request: &Request) -> bool {
        request.body.len() == self.0
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
        .and(BodyLength(RESUMABLE_CHUNK_SIZE_BYTES))
        .respond_with(ResponseTemplate::new(308))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("PUT"))
        .and(path("/upload-session"))
        .and(header("authorization", "Bearer drive-access"))
        .and(header("content-range", "bytes 5242880-5242882/5242883"))
        .and(BodyLength(3))
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
