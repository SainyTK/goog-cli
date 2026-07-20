use chrono::{Duration, Utc};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Match, Mock, MockServer, Request, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::client::AuthClient;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::state::{
    load_runtime_state_from_path, resource_key, save_runtime_state_to_path, RuntimeState,
};
use crate::auth::testing::MemoryStore;
use crate::drive::{CreateFolderOptions, DriveFile, DRIVE_SCOPE};

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

fn scoped_drive_token(access_token: &str) -> Token {
    Token {
        access_token: access_token.into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![DRIVE_SCOPE.into()],
    }
}

fn multi_account_config() -> Config {
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
        accounts: vec![
            "alice@example.com".into(),
            "bob@example.com".into(),
            "carol@example.com".into(),
        ],
    }
}

fn multi_account_store() -> MemoryStore {
    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &scoped_drive_token("alice-access"))
        .unwrap();
    store
        .save_token("bob@example.com", &scoped_drive_token("bob-access"))
        .unwrap();
    store
        .save_token("carol@example.com", &scoped_drive_token("carol-access"))
        .unwrap();
    store
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
    assert!(rendered
        .contains("Roadmap\tfile-1\tfolder-123,folder-456\tapplication/vnd.google-apps.document"));
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
fn write_docs_table_uses_document_id_header() {
    let mut out = Vec::new();
    let mut wrote_header = false;
    write_docs_table(
        &[DriveFile {
            name: "Roadmap".into(),
            id: "doc-1".into(),
            parent_ids: vec!["folder-123".into()],
            mime_type: "application/vnd.google-apps.document".into(),
            modified_time: "2026-06-24T10:15:00.000Z".into(),
        }],
        &mut out,
        &mut wrote_header,
    )
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert_eq!(
        rendered,
        "NAME\tDOCUMENT ID\tPARENT FOLDER IDS\tMODIFIED\nRoadmap\tdoc-1\tfolder-123\t2026-06-24T10:15:00.000Z\n"
    );
}

#[test]
fn write_sheets_table_uses_spreadsheet_id_header() {
    let mut out = Vec::new();
    let mut wrote_header = false;
    write_sheets_table(
        &[DriveFile {
            name: "Budget".into(),
            id: "sheet-1".into(),
            parent_ids: vec!["folder-123".into()],
            mime_type: "application/vnd.google-apps.spreadsheet".into(),
            modified_time: "2026-06-24T12:15:00.000Z".into(),
        }],
        &mut out,
        &mut wrote_header,
    )
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert_eq!(
        rendered,
        "NAME\tSPREADSHEET ID\tPARENT FOLDER IDS\tMODIFIED\nBudget\tsheet-1\tfolder-123\t2026-06-24T12:15:00.000Z\n"
    );
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
    assert_eq!(
        ordered_ids,
        vec!["folder-a", "folder-b", "file-a", "file-z"]
    );
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
        .and(query_param("q", "'root' in parents and trashed = false"))
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
        .and(query_param(
            "q",
            "'folder-123' in parents and trashed = false",
        ))
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
        .and(query_param("q", "'root' in parents and trashed = false"))
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
async fn run_ls_all_lists_following_pages_and_reports_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
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
        "Listed 1 items...\nListed 2 items...\n"
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
            "'folder-123' in parents and mimeType != 'application/vnd.google-apps.folder' and trashed = false",
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
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
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
            "'folder-123' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
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
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
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
async fn run_folder_list_all_lists_following_pages_and_reports_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param(
            "q",
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
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
            "'root' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
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
        "Listed 1 folders...\nListed 2 folders...\n"
    );
}

#[tokio::test]
async fn run_list_all_lists_following_pages_and_reports_progress() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param(
            "q",
            "mimeType != 'application/vnd.google-apps.folder' and trashed = false",
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
            "mimeType != 'application/vnd.google-apps.folder' and trashed = false",
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
        "Listed 1 files...\nListed 2 files...\n"
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
            "mimeType != 'application/vnd.google-apps.folder' and trashed = false",
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
            "mimeType != 'application/vnd.google-apps.folder' and trashed = false",
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

#[tokio::test]
async fn run_mkdir_uses_parent_folder_account_and_prints_folder_location() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .and(query_param("supportsAllDrives", "true"))
        .and(BodyContains(br#""parents":["parent-folder-123"]"#))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .and(query_param("supportsAllDrives", "true"))
        .and(BodyContains(br#""name":"Candidate CVs""#))
        .and(BodyContains(
            br#""mimeType":"application/vnd.google-apps.folder""#,
        ))
        .and(BodyContains(br#""parents":["parent-folder-123"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "folder-456",
            "webViewLink": "https://drive.google.com/drive/folders/folder-456"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let options = CreateFolderOptions::new("Candidate CVs", "parent-folder-123")
        .with_files_url(format!("{}/drive/v3/files", server.uri()));

    run_mkdir_unified_to(&config, &store, None, options, &mut out, Some(&state_path))
        .await
        .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "folder-456\thttps://drive.google.com/drive/folders/folder-456\n"
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("drive", "parent-folder-123")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_delete_removes_the_file_and_confirms_its_id() {
    let server = MockServer::start().await;
    Mock::given(method("DELETE"))
        .and(path("/drive/v3/files/file-123"))
        .and(query_param("supportsAllDrives", "true"))
        .and(header("authorization", "Bearer drive-access"))
        .respond_with(ResponseTemplate::new(204))
        .expect(1)
        .mount(&server)
        .await;
    let store = MemoryStore::default();
    let client = test_client(&store);
    let mut out = Vec::new();

    run_delete_to(
        &client,
        "file-123",
        &mut out,
        Some(&format!("{}/drive/v3/files", server.uri())),
    )
    .await
    .unwrap();

    assert_eq!(String::from_utf8(out).unwrap(), "Deleted\tfile-123\n");
}

#[tokio::test]
async fn run_download_unified_falls_back_on_target_access_failure_and_maps_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/file-123"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/file-123"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for bob"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/file-123"))
        .and(header("authorization", "Bearer carol-access"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(b"hello drive".to_vec()))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let output = temp_dir.path().join("download.txt");
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_download_unified_to(
        &config,
        &store,
        None,
        "file-123".into(),
        Some(output.clone()),
        true,
        Some(&files_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(std::fs::read(output).unwrap(), b"hello drive");
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("drive", "file-123")),
        Some("carol@example.com")
    );
}

#[tokio::test]
async fn run_ls_files_without_target_stays_on_active_account_and_defaults_to_root() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .and(query_param(
            "q",
            "'root' in parents and mimeType != 'application/vnd.google-apps.folder' and trashed = false",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_ls_command_to(
        &config,
        &store,
        None,
        DriveListKind::Files,
        None,
        false,
        None,
        false,
        false,
        true,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "NAME\tFILE ID\tPARENT FOLDER IDS\tMIME TYPE\tMODIFIED\nRoadmap\tfile-1\t\tapplication/vnd.google-apps.document\t2026-06-24T10:15:00.000Z\n"
    );
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_ls_show_all_includes_soft_deleted_items() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .and(query_param(
            "q",
            "'root' in parents and mimeType != 'application/vnd.google-apps.folder'",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_ls_command_to(
        &config,
        &store,
        None,
        DriveListKind::Files,
        None,
        false,
        None,
        true,
        false,
        true,
        &mut out,
        &mut err,
        Some(&files_url),
    )
    .await
    .unwrap();

    assert!(String::from_utf8(out).unwrap().contains("Roadmap\tfile-1"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_list_unified_tries_mapped_folder_before_active_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType != 'application/vnd.google-apps.folder' and trashed = false",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut state = RuntimeState::default();
    state.set_resource_account(resource_key("drive", "folder-123"), "bob@example.com");
    save_runtime_state_to_path(&state, &state_path).unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_list_unified_to(
        &config,
        &store,
        None,
        DriveListKind::Files,
        None,
        false,
        Some("folder-123".into()),
        false,
        false,
        true,
        &mut out,
        &mut err,
        Some(&files_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("Roadmap\tfile-1\t"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_ls_unified_browses_target_folder_with_mapped_account() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .and(query_param(
            "q",
            "'folder-123' in parents and trashed = false",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_MIXED_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut state = RuntimeState::default();
    state.set_resource_account(resource_key("drive", "folder-123"), "bob@example.com");
    save_runtime_state_to_path(&state, &state_path).unwrap();
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_list_unified_to(
        &config,
        &store,
        None,
        DriveListKind::Browse,
        None,
        false,
        Some("folder-123".into()),
        false,
        false,
        true,
        &mut out,
        &mut err,
        Some(&files_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    let rendered = String::from_utf8(out).unwrap();
    assert!(rendered.contains("folder\tAlpha\tfolder-a\t\t"));
    assert!(rendered.contains("file\tArchive\tfile-a\tapplication/pdf"));
    assert!(err.is_empty());
}

#[tokio::test]
async fn run_folder_list_unified_does_not_fallback_for_explicit_account_but_maps_success() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .and(query_param(
            "q",
            "'folder-123' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
        ))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .and(query_param(
            "q",
            "'folder-456' in parents and mimeType = 'application/vnd.google-apps.folder' and trashed = false",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_string(FOLDER_SINGLE_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let files_url = format!("{}/drive/v3/files", server.uri());

    let mut denied_out = Vec::new();
    let mut denied_err = Vec::new();
    let denied = run_list_unified_to(
        &config,
        &store,
        Some("alice@example.com"),
        DriveListKind::Folders,
        None,
        false,
        Some("folder-123".into()),
        false,
        false,
        true,
        &mut denied_out,
        &mut denied_err,
        Some(&files_url),
        Some(&state_path),
    )
    .await;

    let message = format!("{:#}", denied.unwrap_err());
    assert!(message.contains("failed to list Google Drive files"));
    assert!(message.contains("Google Drive permission denied"));
    assert!(denied_out.is_empty());

    let mut mapped_out = Vec::new();
    let mut mapped_err = Vec::new();
    run_list_unified_to(
        &config,
        &store,
        Some("bob@example.com"),
        DriveListKind::Folders,
        None,
        false,
        Some("folder-456".into()),
        false,
        false,
        true,
        &mut mapped_out,
        &mut mapped_err,
        Some(&files_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("drive", "folder-456")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_upload_unified_uses_target_folder_for_account_mapping() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/upload/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .and(query_param("uploadType", "multipart"))
        .and(BodyContains(br#""parents":["folder-123"]"#))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/upload/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .and(query_param("uploadType", "multipart"))
        .and(BodyContains(br#""parents":["folder-123"]"#))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "uploaded-123",
            "webViewLink": "https://drive.google.com/file/d/uploaded-123/view"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let path = temp_dir.path().join("report.txt");
    std::fs::write(&path, "hello drive").unwrap();
    let mut out = Vec::new();
    let upload_url = format!("{}/upload/drive/v3/files", server.uri());

    run_upload_unified_to(
        &config,
        &store,
        None,
        path,
        Some("folder-123".into()),
        true,
        &mut out,
        Some(&upload_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "uploaded-123\thttps://drive.google.com/file/d/uploaded-123/view\n"
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("drive", "folder-123")),
        Some("bob@example.com")
    );
}

#[derive(Clone, Default)]
struct SharedErrBuffer(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl SharedErrBuffer {
    fn snapshot(&self) -> String {
        String::from_utf8(self.0.lock().unwrap().clone()).unwrap()
    }
}

impl std::io::Write for SharedErrBuffer {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[tokio::test]
async fn run_list_unified_all_streams_progress_live_instead_of_buffering_until_done() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(BROWSE_SECOND_PAGE_RESPONSE)
                .set_delay(std::time::Duration::from_millis(200)),
        )
        .expect(1)
        .mount(&server)
        .await;

    let config = test_config();
    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &drive_token())
        .unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let mut err = SharedErrBuffer::default();
    let err_probe = err.clone();
    let files_url = format!("{}/drive/v3/files", server.uri());

    let run_future = run_list_unified_to(
        &config,
        &store,
        None,
        DriveListKind::Browse,
        Some(2),
        true,
        Some("root".into()),
        false,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
        Some(&state_path),
    );
    tokio::pin!(run_future);

    tokio::select! {
        _ = &mut run_future => panic!("expected the second page to still be in flight"),
        _ = tokio::time::sleep(std::time::Duration::from_millis(80)) => {}
    }

    assert_eq!(
        err_probe.snapshot(),
        "Listed 1 items...\n",
        "progress for the first page must reach the real writer before the second page finishes"
    );

    run_future.await.unwrap();

    assert_eq!(
        err_probe.snapshot(),
        "Listed 1 items...\nListed 2 items...\n"
    );
}

#[tokio::test]
async fn run_list_unified_all_keeps_progress_monotonic_after_mid_pagination_fallback() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .and(query_param("pageSize", "2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer alice-access"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
        .respond_with(ResponseTemplate::new(404).set_body_string("missing for alice"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .and(query_param("pageSize", "2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_FIRST_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files"))
        .and(header("authorization", "Bearer bob-access"))
        .and(query_param("pageSize", "1"))
        .and(query_param("pageToken", "token-2"))
        .and(query_param("q", "'root' in parents and trashed = false"))
        .respond_with(ResponseTemplate::new(200).set_body_string(BROWSE_SECOND_PAGE_RESPONSE))
        .expect(1)
        .mount(&server)
        .await;

    let config = multi_account_config();
    let store = multi_account_store();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let mut out = Vec::new();
    let mut err = Vec::new();
    let files_url = format!("{}/drive/v3/files", server.uri());

    run_list_unified_to(
        &config,
        &store,
        None,
        DriveListKind::Browse,
        Some(2),
        true,
        Some("root".into()),
        false,
        false,
        false,
        &mut out,
        &mut err,
        Some(&files_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(err).unwrap(),
        "Listed 1 items...\nListed 2 items...\n"
    );
    assert_eq!(
        load_runtime_state_from_path(&state_path)
            .unwrap()
            .account_for_resource(&resource_key("drive", "root")),
        Some("bob@example.com")
    );
}
