use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, SettingsConfig};
use crate::auth::testing::MemoryStore;
use crate::drive::{DriveFile, DRIVE_SCOPE};

use super::drive::*;

const SINGLE_PAGE_RESPONSE: &str =
    include_str!("../../tests/fixtures/drive/files_page_single.json");
const FIRST_PAGE_RESPONSE: &str = include_str!("../../tests/fixtures/drive/files_page_first.json");
const SECOND_PAGE_RESPONSE: &str =
    include_str!("../../tests/fixtures/drive/files_page_second.json");
const FOLDER_SINGLE_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "folder-1",
      "name": "Projects",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T11:15:00.000Z"
    }
  ]
}"#;
const FOLDER_FIRST_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "nextPageToken": "token-2",
  "files": [
    {
      "id": "folder-1",
      "name": "First",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T11:15:00.000Z"
    }
  ]
}"#;
const FOLDER_SECOND_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "folder-2",
      "name": "Second",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T11:16:00.000Z"
    }
  ]
}"#;
const BROWSE_MIXED_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "file-z",
      "name": "Zeta",
      "parents": ["root"],
      "mimeType": "text/plain",
      "modifiedTime": "2026-06-24T12:00:00.000Z"
    },
    {
      "id": "folder-b",
      "name": "Beta",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T11:00:00.000Z"
    },
    {
      "id": "folder-a",
      "name": "Alpha",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T10:00:00.000Z"
    },
    {
      "id": "file-a",
      "name": "Archive",
      "parents": ["root"],
      "mimeType": "application/pdf",
      "modifiedTime": "2026-06-24T09:00:00.000Z"
    }
  ]
}"#;
const BROWSE_FIRST_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "nextPageToken": "token-2",
  "files": [
    {
      "id": "folder-1",
      "name": "Projects",
      "parents": ["root"],
      "mimeType": "application/vnd.google-apps.folder",
      "modifiedTime": "2026-06-24T11:15:00.000Z"
    }
  ]
}"#;
const BROWSE_SECOND_PAGE_RESPONSE: &str = r#"{
  "kind": "drive#fileList",
  "files": [
    {
      "id": "file-2",
      "name": "Notes",
      "parents": ["root"],
      "mimeType": "text/plain",
      "modifiedTime": "2026-06-24T11:16:00.000Z"
    }
  ]
}"#;

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

struct BodyContains(&'static [u8]);

impl Match for BodyContains {
    fn matches(&self, request: &Request) -> bool {
        request
            .body
            .windows(self.0.len())
            .any(|chunk| chunk == self.0)
    }
}

#[test]
fn write_ndjson_uses_drive_api_field_names() {
    let mut out = Vec::new();
    write_ndjson(
        &[DriveFile {
            name: "Roadmap".into(),
            id: "file-1".into(),
            parent_ids: vec!["folder-123".into(), "folder-456".into()],
            mime_type: "application/vnd.google-apps.document".into(),
            modified_time: "2026-06-24T10:15:00.000Z".into(),
        }],
        &mut out,
    )
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert_eq!(
        rendered,
        "{\"name\":\"Roadmap\",\"id\":\"file-1\",\"parentIds\":[\"folder-123\",\"folder-456\"],\"mimeType\":\"application/vnd.google-apps.document\",\"modifiedTime\":\"2026-06-24T10:15:00.000Z\"}\n"
    );
}

#[test]
fn write_table_includes_expected_columns() {
    let mut out = Vec::new();
    let mut wrote_header = false;
    write_table(
        &[DriveFile {
            name: "Roadmap".into(),
            id: "file-1".into(),
            parent_ids: vec!["folder-123".into(), "folder-456".into()],
            mime_type: "application/vnd.google-apps.document".into(),
            modified_time: "2026-06-24T10:15:00.000Z".into(),
        }],
        &mut out,
        &mut wrote_header,
    )
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("NAME\tFILE ID\tPARENT FOLDER IDS\tMIME TYPE\tMODIFIED"));
    assert!(rendered.contains(
        "Roadmap\tfile-1\tfolder-123,folder-456\tapplication/vnd.google-apps.document"
    ));
}

#[test]
fn write_folder_table_includes_expected_columns() {
    let mut out = Vec::new();
    let mut wrote_header = false;
    write_folder_table(
        &[DriveFile {
            name: "Projects".into(),
            id: "folder-1".into(),
            parent_ids: vec!["root".into()],
            mime_type: "application/vnd.google-apps.folder".into(),
            modified_time: "2026-06-24T11:15:00.000Z".into(),
        }],
        &mut out,
        &mut wrote_header,
    )
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("NAME\tFOLDER ID\tPARENT FOLDER IDS\tMODIFIED"));
    assert!(rendered.contains("Projects\tfolder-1\troot\t2026-06-24T11:15:00.000Z"));
    assert!(!rendered.contains("MIME TYPE"));
}

#[test]
fn write_browse_table_includes_type_and_blanks_folder_mime_type() {
    let mut out = Vec::new();
    let mut wrote_header = false;
    write_browse_table(
        &[
            DriveFile {
                name: "Projects".into(),
                id: "folder-1".into(),
                parent_ids: vec!["root".into()],
                mime_type: "application/vnd.google-apps.folder".into(),
                modified_time: "2026-06-24T11:15:00.000Z".into(),
            },
            DriveFile {
                name: "Roadmap".into(),
                id: "file-1".into(),
                parent_ids: vec!["root".into()],
                mime_type: "application/vnd.google-apps.document".into(),
                modified_time: "2026-06-24T10:15:00.000Z".into(),
            },
        ],
        &mut out,
        &mut wrote_header,
    )
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("TYPE\tNAME\tID\tMIME TYPE\tMODIFIED"));
    assert!(rendered.contains("folder\tProjects\tfolder-1\t\t2026-06-24T11:15:00.000Z"));
    assert!(rendered.contains(
        "file\tRoadmap\tfile-1\tapplication/vnd.google-apps.document\t2026-06-24T10:15:00.000Z"
    ));
}

#[test]
fn sort_browse_items_orders_folders_first_then_files_by_name() {
    let mut files = vec![
        DriveFile {
            name: "Zeta".into(),
            id: "file-z".into(),
            parent_ids: vec![],
            mime_type: "text/plain".into(),
            modified_time: "2026-06-24T12:00:00.000Z".into(),
        },
        DriveFile {
            name: "Beta".into(),
            id: "folder-b".into(),
            parent_ids: vec![],
            mime_type: "application/vnd.google-apps.folder".into(),
            modified_time: "2026-06-24T11:00:00.000Z".into(),
        },
        DriveFile {
            name: "Alpha".into(),
            id: "folder-a".into(),
            parent_ids: vec![],
            mime_type: "application/vnd.google-apps.folder".into(),
            modified_time: "2026-06-24T10:00:00.000Z".into(),
        },
        DriveFile {
            name: "Archive".into(),
            id: "file-a".into(),
            parent_ids: vec![],
            mime_type: "application/pdf".into(),
            modified_time: "2026-06-24T09:00:00.000Z".into(),
        },
    ];

    sort_browse_items(&mut files);

    let ordered_ids: Vec<_> = files.iter().map(|file| file.id.as_str()).collect();
    assert_eq!(ordered_ids, vec!["folder-a", "folder-b", "file-a", "file-z"]);
}

#[test]
fn json_flag_overrides_table_default() {
    assert!(should_emit_json(true, false));
}

#[test]
fn config_output_json_flips_default_output() {
    assert!(should_emit_json(false, true));
}

#[tokio::test]
async fn run_ls_defaults_to_drive_root_and_renders_mixed_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("q", "'root' in parents"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_MIXED_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_ls_to(
        &client,
        None,
        false,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert_eq!(
        rendered,
        "TYPE\tNAME\tID\tMIME TYPE\tMODIFIED\nfolder\tAlpha\tfolder-a\t\t2026-06-24T10:00:00.000Z\nfolder\tBeta\tfolder-b\t\t2026-06-24T11:00:00.000Z\nfile\tArchive\tfile-a\tapplication/pdf\t2026-06-24T09:00:00.000Z\nfile\tZeta\tfile-z\ttext/plain\t2026-06-24T12:00:00.000Z\n"
    );
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_ls_filters_to_folder_when_requested() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param("q", "'folder-123' in parents"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_MIXED_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_ls_to(
        &client,
        None,
        false,
        Some("folder-123".into()),
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("folder\tAlpha\tfolder-a\t\t"));
    assert!(rendered.contains("file\tArchive\tfile-a\tapplication/pdf"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_ls_emits_ndjson_with_drive_native_mime_type_field() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("q", "'root' in parents"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_MIXED_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_ls_to(
        &client,
        None,
        false,
        None,
        true,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("\"parents\":[\"root\"]"));
    assert!(!rendered.contains("\"parentIds\""));
    assert!(rendered.contains("\"mimeType\":\"application/vnd.google-apps.folder\""));
    assert!(rendered.contains("\"mimeType\":\"application/pdf\""));
    assert!(rendered.starts_with(
        "{\"name\":\"Alpha\",\"id\":\"folder-a\",\"parents\":[\"root\"],\"mimeType\""
    ));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_ls_all_fetches_following_pages_and_reports_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param("q", "'root' in parents"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param("q", "'root' in parents"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_SECOND_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_ls_to(
        &client,
        Some(2),
        true,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("folder\tProjects\tfolder-1\t\t"));
    assert!(rendered.contains("file\tNotes\tfile-2\ttext/plain"));
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "Fetched 1 items...\nFetched 2 items...\n"
    );
}

#[tokio::test]
async fn run_list_uses_json_when_config_default_requests_json() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_list_to(
        &client,
        None,
        false,
        None,
        true,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"name\":\"Roadmap\",\"id\":\"file-1\",\"parentIds\":[],\"mimeType\":\"application/vnd.google-apps.document\",\"modifiedTime\":\"2026-06-24T10:15:00.000Z\"}\n"
    );
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_list_filters_to_folder_when_requested() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType != 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_list_to(
        &client,
        None,
        false,
        Some("folder-123".into()),
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("Roadmap\tfile-1\t"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_folder_list_defaults_to_drive_root() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param(
            "q",
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_folder_list_to(
        &client,
        None,
        false,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("NAME\tFOLDER ID\tPARENT FOLDER IDS\tMODIFIED"));
    assert!(rendered.contains("Projects\tfolder-1\troot\t2026-06-24T11:15:00.000Z"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_folder_list_filters_to_parent_when_requested() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType = 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_folder_list_to(
        &client,
        None,
        false,
        Some("folder-123".into()),
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("Projects\tfolder-1\troot\t"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_folder_list_emits_ndjson_with_parent_ids() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("pageSize", "50"))
        .and(query_param(
            "q",
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_folder_list_to(
        &client,
        None,
        false,
        None,
        true,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"name\":\"Projects\",\"id\":\"folder-1\",\"parentIds\":[\"root\"],\"mimeType\":\"application/vnd.google-apps.folder\",\"modifiedTime\":\"2026-06-24T11:15:00.000Z\"}\n"
    );
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_folder_list_all_fetches_following_pages_and_reports_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param(
            "q",
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param(
            "q",
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_SECOND_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_folder_list_to(
        &client,
        Some(2),
        true,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("First\tfolder-1\troot\t"));
    assert!(rendered.contains("Second\tfolder-2\troot\t"));
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "Fetched 1 folders...\nFetched 2 folders...\n"
    );
}

#[tokio::test]
async fn run_list_all_fetches_following_pages_and_reports_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param(
            "q",
            "mimeType != 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param(
            "q",
            "mimeType != 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SECOND_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_list_to(
        &client,
        Some(2),
        true,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("First\tfile-1\t\ttext/plain"));
    assert!(rendered.contains("Second\tfile-2\t\ttext/plain"));
    assert_eq!(
        String::from_utf8(err).unwrap(),
        "Fetched 1 files...\nFetched 2 files...\n"
    );
}

#[tokio::test]
async fn run_list_limit_can_span_multiple_pages_without_all() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param(
            "q",
            "mimeType != 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param(
            "q",
            "mimeType != 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SECOND_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_list_to(
        &client,
        Some(2),
        false,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("First\tfile-1\t\ttext/plain"));
    assert!(rendered.contains("Second\tfile-2\t\ttext/plain"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_list_returns_clear_error_for_not_found_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    let result = run_list_to(
        &client,
        None,
        false,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to list Google Drive files"));
    assert!(message.contains("Google Drive resource was not found"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_list_returns_clear_error_for_permission_denied_response() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    let result = run_list_to(
        &client,
        None,
        false,
        None,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await;

    let message = format!("{:#}", result.unwrap_err());
    assert!(message.contains("failed to list Google Drive files"));
    assert!(message.contains("Google Drive permission denied"));
    assert!(out.is_empty());
}

#[tokio::test]
async fn run_upload_prints_uploaded_file_id_and_url() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload/drive/v3/files"))
        .and(header("authorization", "Bearer drive-access"))
        .and(query_param("uploadType", "multipart"))
        .and(BodyContains(br#""name":"report.txt""#))
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
    let mut out = Vec::new();
    let upload_url = format!("{}/upload/drive/v3/files", server.uri());

    run_upload_to(&client, path, None, true, &mut out, Some(&upload_url))
        .await
        .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "file-123\thttps://drive.google.com/file/d/file-123/view\n"
    );
}
