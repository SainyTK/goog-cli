use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::AccountStore;
use crate::auth::testing::MemoryStore;

use super::drive_location::*;
use super::drive_tests::{drive_token, test_client, test_config};

fn folder_metadata() -> serde_json::Value {
    serde_json::json!({
        "id": "folder-123",
        "name": "Meeting Notes",
        "mimeType": "application/vnd.google-apps.folder",
        "parents": ["shared-drive-123"],
        "driveId": "shared-drive-123",
        "webViewLink": "https://drive.google.com/drive/folders/folder-123",
        "modifiedTime": "2026-06-24T10:00:00.000Z",
        "trashed": false
    })
}

async fn run_info(server: &MockServer, json: bool) -> String {
    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &drive_token())
        .unwrap();
    let temp_dir = tempfile::tempdir().unwrap();
    let state_path = temp_dir.path().join("state.toml");
    let files_url = format!("{}/drive/v3/files", server.uri());
    let mut out = Vec::new();

    run_info_unified_to(
        &test_config(),
        &store,
        None,
        "folder-123".into(),
        json,
        &mut out,
        Some(&files_url),
        Some(&state_path),
    )
    .await
    .unwrap();

    String::from_utf8(out).unwrap()
}

async fn mount_folder_metadata(server: &MockServer) {
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/folder-123"))
        .and(query_param("supportsAllDrives", "true"))
        .and(query_param(
            "fields",
            "id,name,mimeType,parents,driveId,webViewLink,modifiedTime,trashed",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(folder_metadata()))
        .expect(1)
        .mount(server)
        .await;
}

#[tokio::test]
async fn info_prints_parents_shared_drive_and_link() {
    let server = MockServer::start().await;
    mount_folder_metadata(&server).await;

    assert_eq!(
        run_info(&server, false).await,
        "NAME\tMeeting Notes\n\
         ID\tfolder-123\n\
         MIME TYPE\tapplication/vnd.google-apps.folder\n\
         PARENT FOLDER IDS\tshared-drive-123\n\
         SHARED DRIVE ID\tshared-drive-123\n\
         MODIFIED\t2026-06-24T10:00:00.000Z\n\
         TRASHED\tfalse\n\
         LINK\thttps://drive.google.com/drive/folders/folder-123\n"
    );
}

#[tokio::test]
async fn info_json_uses_parent_ids_field() {
    let server = MockServer::start().await;
    mount_folder_metadata(&server).await;

    let value: serde_json::Value = serde_json::from_str(&run_info(&server, true).await).unwrap();

    assert_eq!(value["parentIds"], serde_json::json!(["shared-drive-123"]));
    assert_eq!(value["driveId"], "shared-drive-123");
    assert!(value.get("parents").is_none());
}

#[tokio::test]
async fn info_marks_missing_parents_and_drive_with_a_dash() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/files/folder-123"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "folder-123",
            "name": "Shared with me",
            "mimeType": "application/vnd.google-apps.folder",
            "modifiedTime": "2026-06-24T10:00:00.000Z"
        })))
        .mount(&server)
        .await;

    let rendered = run_info(&server, false).await;

    assert!(rendered.contains("PARENT FOLDER IDS\t-\n"));
    assert!(rendered.contains("SHARED DRIVE ID\t-\n"));
    assert!(rendered.contains("LINK\t-\n"));
}

#[tokio::test]
async fn drives_lists_every_page_sorted_by_name() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/drives"))
        .and(query_param("pageToken", "token-2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "drives": [{ "id": "drive-a", "name": "alpha" }]
        })))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/drive/v3/drives"))
        .and(query_param("fields", "nextPageToken,drives(id,name)"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "drives": [{ "id": "drive-b", "name": "Beta" }],
            "nextPageToken": "token-2"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    let client = test_client(&store);
    let drives_url = format!("{}/drive/v3/drives", server.uri());
    let mut out = Vec::new();

    run_drives_to(&client, false, &mut out, Some(&drives_url))
        .await
        .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "NAME\tDRIVE ID\nalpha\tdrive-a\nBeta\tdrive-b\n"
    );
}
