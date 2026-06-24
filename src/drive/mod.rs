pub mod error;

pub use error::DriveError;

use std::path::PathBuf;

use reqwest::{Response, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
pub const DRIVE_SCOPES: &[&str] = &[DRIVE_SCOPE];
const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_FILES_FIELDS: &str = "nextPageToken,files(id,name,mimeType,modifiedTime)";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveFile {
    pub name: String,
    pub id: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "modifiedTime")]
    pub modified_time: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FilesPage {
    #[serde(default)]
    pub files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedFile {
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Clone)]
pub struct DownloadFileOptions {
    pub file_id: String,
    pub output: Option<PathBuf>,
    files_url: String,
}

impl DownloadFileOptions {
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
            output: None,
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn with_output(mut self, output: impl Into<PathBuf>) -> Self {
        self.output = Some(output.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn metadata_url(&self) -> Result<Url, DriveError> {
        let mut url = self.file_url()?;
        url.query_pairs_mut().append_pair("fields", "name");
        Ok(url)
    }

    fn media_url(&self) -> Result<Url, DriveError> {
        let mut url = self.file_url()?;
        url.query_pairs_mut().append_pair("alt", "media");
        Ok(url)
    }

    fn file_url(&self) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.files_url)?;
        url.path_segments_mut()
            .map_err(|_| {
                DriveError::InvalidResponse("Google Drive API URL cannot be a base".into())
            })?
            .push(&self.file_id);
        Ok(url)
    }
}

#[derive(Debug, Deserialize)]
struct FileMetadata {
    name: String,
}

#[derive(Debug, Clone)]
pub struct ListFilesOptions {
    pub page_size: u32,
    pub page_token: Option<String>,
    files_url: String,
}

impl ListFilesOptions {
    pub fn new(page_size: u32) -> Self {
        Self {
            page_size,
            page_token: None,
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn with_page_token(mut self, page_token: impl Into<String>) -> Self {
        self.page_token = Some(page_token.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.files_url)?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("pageSize", &self.page_size.to_string())
                .append_pair("orderBy", "modifiedTime desc")
                .append_pair("fields", DRIVE_FILES_FIELDS);
            if let Some(page_token) = &self.page_token {
                query.append_pair("pageToken", page_token);
            }
        }
        Ok(url)
    }
}

pub async fn list_files<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListFilesOptions,
) -> Result<FilesPage, DriveError> {
    let url = options.request_url()?;
    let response = client
        .send_with_scopes(client.get(url), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;

    parse_files_response(response).await
}

pub async fn download<S, F>(
    client: &AuthClient<'_, S>,
    options: &DownloadFileOptions,
    mut progress: F,
) -> Result<DownloadedFile, DriveError>
where
    S: AccountStore,
    F: FnMut(u64),
{
    let path = match &options.output {
        Some(output) => output.clone(),
        None => {
            let metadata = fetch_metadata(client, options).await?;
            std::env::current_dir()
                .map_err(DriveError::Io)?
                .join(metadata.name)
        }
    };

    let response = client
        .send_with_scopes(client.get(options.media_url()?), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;

    let mut response = ensure_success_response(response).await?;
    let mut file = tokio::fs::File::create(&path).await.map_err(DriveError::Io)?;
    let mut bytes = 0_u64;

    while let Some(chunk) = response.chunk().await.map_err(DriveError::Network)? {
        file.write_all(&chunk).await.map_err(DriveError::Io)?;
        bytes += chunk.len() as u64;
        progress(bytes);
    }

    file.flush().await.map_err(DriveError::Io)?;
    Ok(DownloadedFile { path, bytes })
}

async fn fetch_metadata<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DownloadFileOptions,
) -> Result<FileMetadata, DriveError> {
    let response = client
        .send_with_scopes(client.get(options.metadata_url()?), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;

    let response = ensure_success_response(response).await?;
    response
        .json::<FileMetadata>()
        .await
        .map_err(|e| DriveError::InvalidResponse(e.to_string()))
}

async fn parse_files_response(response: Response) -> Result<FilesPage, DriveError> {
    let response = ensure_success_response(response).await?;
    response
        .json::<FilesPage>()
        .await
        .map_err(|e| DriveError::InvalidResponse(e.to_string()))
}

async fn ensure_success_response(response: Response) -> Result<Response, DriveError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    match status {
        StatusCode::NOT_FOUND => Err(DriveError::NotFound),
        StatusCode::FORBIDDEN => Err(DriveError::PermissionDenied),
        status => {
            let body = response.text().await.unwrap_or_default();
            Err(DriveError::Api { status, body })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Mutex, MutexGuard};

    use chrono::{Duration, Utc};
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::auth::account::{testing::MemoryStore, AccountStore, Token};
    use crate::auth::config::{Config, OAuthAppConfig, SettingsConfig};

    const SINGLE_PAGE_RESPONSE: &str =
        include_str!("../../tests/fixtures/drive/files_page_single.json");
    const EMPTY_PAGE_WITH_TOKEN_RESPONSE: &str =
        include_str!("../../tests/fixtures/drive/files_page_empty_with_token.json");

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
        store.save_token("alice@example.com", &drive_token()).unwrap();
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
        let options = ListFilesOptions::new(50)
            .with_files_url(format!("{}/drive/v3/files", server.uri()));

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
        let options = ListFilesOptions::new(50)
            .with_files_url(format!("{}/drive/v3/files", server.uri()));

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
        let options = ListFilesOptions::new(50)
            .with_files_url(format!("{}/drive/v3/files", server.uri()));

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
            .respond_with(
                ResponseTemplate::new(200).set_body_bytes(b"hello\x00drive".to_vec()),
            )
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

        assert_eq!(downloaded.path, temp.path().join("report.txt"));
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
}
