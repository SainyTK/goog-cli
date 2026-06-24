pub mod error;

pub use error::DriveError;

use bytes::Bytes;
use futures_util::{stream, StreamExt};
use reqwest::header::{CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, LOCATION};
use std::path::PathBuf;

use reqwest::{Body, Response, StatusCode};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt, SeekFrom};
use tokio_util::io::ReaderStream;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
pub const DRIVE_SCOPES: &[&str] = &[DRIVE_SCOPE];
const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_UPLOAD_URL: &str = "https://www.googleapis.com/upload/drive/v3/files";
const DRIVE_FILES_FIELDS: &str = "nextPageToken,files(id,name,mimeType,modifiedTime)";
const UPLOAD_RESPONSE_FIELDS: &str = "id,webViewLink";
const MULTIPART_UPLOAD_LIMIT_BYTES: u64 = 5 * 1024 * 1024;
const RESUMABLE_CHUNK_SIZE_BYTES: usize = 5 * 1024 * 1024;
const DEFAULT_UPLOAD_MIME_TYPE: &str = "application/octet-stream";
const JSON_CONTENT_TYPE: &str = "application/json; charset=UTF-8";
const MULTIPART_UPLOAD_BOUNDARY: &str = "goog-drive-upload-boundary";
const UPLOAD_CONTENT_TYPE_HEADER: &str = "X-Upload-Content-Type";
const UPLOAD_CONTENT_LENGTH_HEADER: &str = "X-Upload-Content-Length";

#[derive(Debug, Clone, Copy)]
enum UploadType {
    Multipart,
    Resumable,
}

impl UploadType {
    fn as_query_value(self) -> &'static str {
        match self {
            Self::Multipart => "multipart",
            Self::Resumable => "resumable",
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UploadedFile {
    pub id: String,
    #[serde(rename = "webViewLink")]
    pub web_view_link: String,
}

#[derive(Debug, Clone)]
pub struct UploadFileOptions {
    pub path: PathBuf,
    pub folder: Option<String>,
    upload_url: String,
}

impl UploadFileOptions {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            folder: None,
            upload_url: DRIVE_UPLOAD_URL.to_string(),
        }
    }

    pub fn with_folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn with_upload_url(mut self, upload_url: impl Into<String>) -> Self {
        self.upload_url = upload_url.into();
        self
    }

    fn upload_url(&self, upload_type: UploadType) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.upload_url)?;
        url.query_pairs_mut()
            .append_pair("uploadType", upload_type.as_query_value())
            .append_pair("fields", UPLOAD_RESPONSE_FIELDS);
        Ok(url)
    }
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

#[derive(Debug, Serialize)]
struct UploadMetadata {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parents: Option<Vec<String>>,
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

pub async fn upload<S, F>(
    client: &AuthClient<'_, S>,
    options: &UploadFileOptions,
    progress: F,
) -> Result<UploadedFile, DriveError>
where
    S: AccountStore,
    F: FnMut(u64),
{
    let metadata = tokio::fs::metadata(&options.path)
        .await
        .map_err(DriveError::Io)?;
    let file_size = metadata.len();

    if file_size <= MULTIPART_UPLOAD_LIMIT_BYTES {
        upload_multipart(client, options, file_size, progress).await
    } else {
        upload_resumable(client, options, file_size, progress).await
    }
}

async fn upload_multipart<S, F>(
    client: &AuthClient<'_, S>,
    options: &UploadFileOptions,
    file_size: u64,
    mut progress: F,
) -> Result<UploadedFile, DriveError>
where
    S: AccountStore,
    F: FnMut(u64),
{
    let metadata = upload_metadata(options)?;
    let metadata_json =
        serde_json::to_string(&metadata).map_err(|e| DriveError::InvalidResponse(e.to_string()))?;
    let header = Bytes::from(format!(
        "--{MULTIPART_UPLOAD_BOUNDARY}\r\n\
         Content-Type: {JSON_CONTENT_TYPE}\r\n\r\n\
         {metadata_json}\r\n\
         --{MULTIPART_UPLOAD_BOUNDARY}\r\n\
         Content-Type: {DEFAULT_UPLOAD_MIME_TYPE}\r\n\r\n"
    ));
    let footer = Bytes::from(format!("\r\n--{MULTIPART_UPLOAD_BOUNDARY}--\r\n"));
    let content_length = header.len() as u64 + file_size + footer.len() as u64;
    let file = tokio::fs::File::open(&options.path)
        .await
        .map_err(DriveError::Io)?;
    let body = multipart_body(header, file, footer);

    let response = client
        .send_with_scopes(
            client
                .post(options.upload_url(UploadType::Multipart)?)
                .header(
                    CONTENT_TYPE,
                    format!("multipart/related; boundary={MULTIPART_UPLOAD_BOUNDARY}"),
                )
                .header(CONTENT_LENGTH, content_length.to_string())
                .body(body),
            DRIVE_SCOPES,
        )
        .await
        .map_err(DriveError::Auth)?;

    let uploaded = parse_uploaded_file_response(response).await?;
    progress(file_size);
    Ok(uploaded)
}

async fn upload_resumable<S, F>(
    client: &AuthClient<'_, S>,
    options: &UploadFileOptions,
    file_size: u64,
    mut progress: F,
) -> Result<UploadedFile, DriveError>
where
    S: AccountStore,
    F: FnMut(u64),
{
    let session_uri = initiate_resumable_upload(client, options, file_size).await?;
    let mut uploaded = 0_u64;

    while uploaded < file_size {
        let read = next_resumable_chunk_size(uploaded, file_size);
        let start = uploaded;
        let end = uploaded + read as u64 - 1;
        let body = resumable_chunk_body(&options.path, start, read).await?;
        let response = client
            .send_with_scopes(
                client
                    .put(session_uri.as_str())
                    .header(CONTENT_LENGTH, read.to_string())
                    .header(CONTENT_RANGE, format!("bytes {start}-{end}/{file_size}"))
                    .body(body),
                DRIVE_SCOPES,
            )
            .await
            .map_err(DriveError::Auth)?;

        uploaded += read as u64;
        progress(uploaded);

        let upload_complete = uploaded == file_size;
        let server_expects_more_chunks = response.status() == StatusCode::PERMANENT_REDIRECT;
        if upload_complete || !server_expects_more_chunks {
            return parse_uploaded_file_response(response).await;
        }
    }

    Err(DriveError::InvalidResponse(
        "resumable upload completed without a final response".into(),
    ))
}

fn next_resumable_chunk_size(uploaded: u64, file_size: u64) -> usize {
    let remaining = file_size - uploaded;
    remaining.min(RESUMABLE_CHUNK_SIZE_BYTES as u64) as usize
}

async fn resumable_chunk_body(
    path: &std::path::Path,
    start: u64,
    length: usize,
) -> Result<Body, DriveError> {
    let mut file = tokio::fs::File::open(path).await.map_err(DriveError::Io)?;
    file.seek(SeekFrom::Start(start))
        .await
        .map_err(DriveError::Io)?;
    let reader = file.take(length as u64);

    Ok(Body::wrap_stream(ReaderStream::new(reader)))
}

async fn initiate_resumable_upload<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &UploadFileOptions,
    file_size: u64,
) -> Result<String, DriveError> {
    let metadata = upload_metadata(options)?;
    let response = client
        .send_with_scopes(
            client
                .post(options.upload_url(UploadType::Resumable)?)
                .header(CONTENT_TYPE, JSON_CONTENT_TYPE)
                .header(UPLOAD_CONTENT_TYPE_HEADER, DEFAULT_UPLOAD_MIME_TYPE)
                .header(UPLOAD_CONTENT_LENGTH_HEADER, file_size.to_string())
                .json(&metadata),
            DRIVE_SCOPES,
        )
        .await
        .map_err(DriveError::Auth)?;

    let response = ensure_success_response(response).await?;
    let location = response
        .headers()
        .get(LOCATION)
        .ok_or_else(|| DriveError::InvalidResponse("missing resumable upload location".into()))?
        .to_str()
        .map_err(|e| DriveError::InvalidResponse(e.to_string()))?;

    Ok(location.to_string())
}

fn upload_metadata(options: &UploadFileOptions) -> Result<UploadMetadata, DriveError> {
    let name = options
        .path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| DriveError::InvalidResponse("upload path has no file name".into()))?
        .to_string();

    Ok(UploadMetadata {
        name,
        parents: options.folder.as_ref().map(|folder| vec![folder.clone()]),
    })
}

fn multipart_body(header: Bytes, file: tokio::fs::File, footer: Bytes) -> Body {
    let header = stream::once(async move { Ok::<Bytes, std::io::Error>(header) });
    let file = ReaderStream::new(file);
    let footer = stream::once(async move { Ok::<Bytes, std::io::Error>(footer) });
    Body::wrap_stream(header.chain(file).chain(footer))
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

async fn parse_uploaded_file_response(response: Response) -> Result<UploadedFile, DriveError> {
    let response = ensure_success_response(response).await?;
    response
        .json::<UploadedFile>()
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
    use wiremock::{Match, Request};
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

    struct BodyContains(&'static [u8]);

    impl Match for BodyContains {
        fn matches(&self, request: &Request) -> bool {
            request.body.windows(self.0.len()).any(|chunk| chunk == self.0)
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
            .respond_with(
                ResponseTemplate::new(200).insert_header("Location", session_uri.clone()),
            )
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
}
