pub mod error;

pub use error::DriveError;

use bytes::Bytes;
use futures_util::{stream, StreamExt};
use reqwest::header::{CONTENT_LENGTH, CONTENT_RANGE, CONTENT_TYPE, LOCATION};
use std::path::{Path, PathBuf};

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
pub(super) const DRIVE_FILES_FIELDS: &str =
    "nextPageToken,files(id,name,parents,mimeType,modifiedTime)";
pub(crate) const DRIVE_FOLDER_MIME_TYPE: &str = "application/vnd.google-apps.folder";
pub(crate) const GOOGLE_DOC_MIME_TYPE: &str = "application/vnd.google-apps.document";
pub(crate) const GOOGLE_SHEET_MIME_TYPE: &str = "application/vnd.google-apps.spreadsheet";
pub(crate) const GOOGLE_SLIDES_MIME_TYPE: &str = "application/vnd.google-apps.presentation";
const UPLOAD_RESPONSE_FIELDS: &str = "id,webViewLink";
const CREATE_FOLDER_RESPONSE_FIELDS: &str = "id,webViewLink";
pub(super) const MULTIPART_UPLOAD_LIMIT_BYTES: u64 = 5 * 1024 * 1024;
pub(super) const RESUMABLE_CHUNK_SIZE_BYTES: usize = 5 * 1024 * 1024;
const DEFAULT_UPLOAD_MIME_TYPE: &str = "application/octet-stream";
const JSON_CONTENT_TYPE: &str = "application/json; charset=UTF-8";
const MULTIPART_UPLOAD_BOUNDARY: &str = "goog-drive-upload-boundary";
const UPLOAD_CONTENT_TYPE_HEADER: &str = "X-Upload-Content-Type";
const UPLOAD_CONTENT_LENGTH_HEADER: &str = "X-Upload-Content-Length";
const DEFAULT_MAX_EXPORT_BYTES: usize = 10 * 1024 * 1024;

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
    #[serde(default, rename(serialize = "parentIds", deserialize = "parents"))]
    pub parent_ids: Vec<String>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GoogleFileExportFormat {
    Word,
    Excel,
    PowerPoint,
    Pdf,
}

impl GoogleFileExportFormat {
    fn mime_type(self) -> &'static str {
        match self {
            Self::Word => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
            Self::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            Self::PowerPoint => {
                "application/vnd.openxmlformats-officedocument.presentationml.presentation"
            }
            Self::Pdf => "application/pdf",
        }
    }

    fn has_valid_signature(self, signature: &[u8]) -> bool {
        match self {
            Self::Word | Self::Excel | Self::PowerPoint => signature.starts_with(b"PK\x03\x04"),
            Self::Pdf => signature.starts_with(b"%PDF-"),
        }
    }

    fn display_name(self) -> &'static str {
        match self {
            Self::Word => "Word",
            Self::Excel => "Excel",
            Self::PowerPoint => "PowerPoint",
            Self::Pdf => "PDF",
        }
    }

    fn file_extension(self) -> &'static str {
        match self {
            Self::Word => "docx",
            Self::Excel => "xlsx",
            Self::PowerPoint => "pptx",
            Self::Pdf => "pdf",
        }
    }

    fn for_google_mime_type(mime_type: &str) -> Option<Self> {
        match mime_type {
            GOOGLE_DOC_MIME_TYPE => Some(Self::Word),
            GOOGLE_SHEET_MIME_TYPE => Some(Self::Excel),
            GOOGLE_SLIDES_MIME_TYPE => Some(Self::PowerPoint),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct UploadedFile {
    pub id: String,
    #[serde(rename = "webViewLink")]
    pub web_view_link: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct CreatedFolder {
    pub id: String,
    #[serde(rename = "webViewLink")]
    pub web_view_link: String,
}

#[derive(Debug, Clone)]
pub struct CreateFolderOptions {
    pub name: String,
    pub parent_folder: String,
    files_url: String,
}

impl CreateFolderOptions {
    pub fn new(name: impl Into<String>, parent_folder: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            parent_folder: parent_folder.into(),
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    #[cfg(test)]
    pub(super) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.files_url)?;
        url.query_pairs_mut()
            .append_pair("fields", CREATE_FOLDER_RESPONSE_FIELDS)
            .append_pair("supportsAllDrives", "true");
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct UploadFileOptions {
    pub path: PathBuf,
    pub folder: Option<String>,
    mime_type: String,
    upload_url: String,
}

impl UploadFileOptions {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            folder: None,
            mime_type: DEFAULT_UPLOAD_MIME_TYPE.to_string(),
            upload_url: DRIVE_UPLOAD_URL.to_string(),
        }
    }

    pub fn with_folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    pub fn with_mime_type(mut self, mime_type: impl Into<String>) -> Self {
        self.mime_type = mime_type.into();
        self
    }

    pub(super) fn with_upload_url(mut self, upload_url: impl Into<String>) -> Self {
        self.upload_url = upload_url.into();
        self
    }

    fn upload_url(&self, upload_type: UploadType) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.upload_url)?;
        url.query_pairs_mut()
            .append_pair("uploadType", upload_type.as_query_value())
            .append_pair("fields", UPLOAD_RESPONSE_FIELDS)
            .append_pair("supportsAllDrives", "true");
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct DriveFileOperationOptions {
    pub file_id: String,
    files_url: String,
}

impl DriveFileOperationOptions {
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub(super) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn file_url(&self) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.files_url)?;
        url.path_segments_mut()
            .map_err(|_| {
                DriveError::InvalidResponse("Google Drive API URL cannot be a base".into())
            })?
            .push(&self.file_id);
        url.query_pairs_mut()
            .append_pair("supportsAllDrives", "true");
        Ok(url)
    }

    fn permissions_url(&self) -> Result<Url, DriveError> {
        let mut url = self.file_url()?;
        url.path_segments_mut()
            .map_err(|_| {
                DriveError::InvalidResponse("Google Drive API URL cannot be a base".into())
            })?
            .push("permissions");
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

    pub(super) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn metadata_url(&self) -> Result<Url, DriveError> {
        let mut url = self.file_url()?;
        url.query_pairs_mut().append_pair("fields", "name,mimeType");
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
        url.query_pairs_mut()
            .append_pair("supportsAllDrives", "true");
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct ExportGoogleFileOptions {
    pub file_id: String,
    pub format: GoogleFileExportFormat,
    pub output: PathBuf,
    max_download_bytes: usize,
    files_url: String,
}

impl ExportGoogleFileOptions {
    pub fn new(
        file_id: impl Into<String>,
        format: GoogleFileExportFormat,
        output: impl Into<PathBuf>,
    ) -> Self {
        Self {
            file_id: file_id.into(),
            format,
            output: output.into(),
            max_download_bytes: DEFAULT_MAX_EXPORT_BYTES,
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn with_max_download_bytes(mut self, max_download_bytes: usize) -> Self {
        self.max_download_bytes = max_download_bytes;
        self
    }

    pub(crate) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn export_url(&self) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.files_url)?;
        url.path_segments_mut()
            .map_err(|_| {
                DriveError::InvalidResponse("Google Drive API URL cannot be a base".into())
            })?
            .push(&self.file_id)
            .push("export");
        url.query_pairs_mut()
            .append_pair("mimeType", self.format.mime_type());
        Ok(url)
    }
}

#[derive(Debug, Deserialize)]
struct FileMetadata {
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
}

#[derive(Debug, Serialize)]
struct UploadMetadata {
    name: String,
    #[serde(rename = "mimeType")]
    mime_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parents: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
struct CreateFolderMetadata<'a> {
    name: &'a str,
    #[serde(rename = "mimeType")]
    mime_type: &'static str,
    parents: [&'a str; 1],
}

#[derive(Debug, Clone)]
pub struct ListFilesOptions {
    pub page_size: u32,
    pub page_token: Option<String>,
    pub folder: Option<String>,
    show_all: bool,
    mode: ListFilesMode,
    files_url: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ListFilesMode {
    Files,
    Folders,
    Browse,
    Docs,
    Sheets,
    Slides,
}

impl ListFilesOptions {
    pub fn new(page_size: u32) -> Self {
        Self {
            page_size,
            page_token: None,
            folder: None,
            show_all: false,
            mode: ListFilesMode::Files,
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn folders(page_size: u32) -> Self {
        Self {
            mode: ListFilesMode::Folders,
            ..Self::new(page_size)
        }
    }

    pub fn browse(page_size: u32) -> Self {
        Self {
            mode: ListFilesMode::Browse,
            ..Self::new(page_size)
        }
    }

    pub fn docs(page_size: u32) -> Self {
        Self {
            mode: ListFilesMode::Docs,
            ..Self::new(page_size)
        }
    }

    pub fn sheets(page_size: u32) -> Self {
        Self {
            mode: ListFilesMode::Sheets,
            ..Self::new(page_size)
        }
    }

    pub fn slides(page_size: u32) -> Self {
        Self {
            mode: ListFilesMode::Slides,
            ..Self::new(page_size)
        }
    }

    pub fn with_page_token(mut self, page_token: impl Into<String>) -> Self {
        self.page_token = Some(page_token.into());
        self
    }

    pub fn with_folder(mut self, folder: impl Into<String>) -> Self {
        self.folder = Some(folder.into());
        self
    }

    pub fn with_show_all(mut self) -> Self {
        self.show_all = true;
        self
    }

    pub(super) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.files_url)?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("pageSize", &self.page_size.to_string())
                .append_pair("orderBy", self.mode.order_by())
                .append_pair("fields", DRIVE_FILES_FIELDS)
                .append_pair("q", &self.query())
                .append_pair("supportsAllDrives", "true")
                .append_pair("includeItemsFromAllDrives", "true")
                .append_pair("corpora", "allDrives");
            if let Some(page_token) = &self.page_token {
                query.append_pair("pageToken", page_token);
            }
        }
        Ok(url)
    }

    fn query(&self) -> String {
        let mut filters = Vec::new();
        if let Some(parent_filter) = self.parent_filter() {
            filters.push(parent_filter);
        }
        if let Some(mime_type_filter) = self.mode.mime_type_filter() {
            filters.push(mime_type_filter);
        }
        if !self.show_all {
            filters.push("trashed = false".into());
        }
        filters.join(" and ")
    }

    fn parent_filter(&self) -> Option<String> {
        match (self.mode, self.folder.as_deref()) {
            (_, Some(folder)) => Some(parent_query_filter(folder)),
            (ListFilesMode::Folders, None) => Some(parent_query_filter("root")),
            (ListFilesMode::Browse, None) => Some(parent_query_filter("root")),
            (
                ListFilesMode::Files
                | ListFilesMode::Docs
                | ListFilesMode::Sheets
                | ListFilesMode::Slides,
                None,
            ) => None,
        }
    }
}

impl ListFilesMode {
    fn mime_type_filter(self) -> Option<String> {
        match self {
            Self::Files => Some(format!("mimeType != '{DRIVE_FOLDER_MIME_TYPE}'")),
            Self::Folders => Some(format!("mimeType = '{DRIVE_FOLDER_MIME_TYPE}'")),
            Self::Browse => None,
            Self::Docs => Some(format!("mimeType = '{GOOGLE_DOC_MIME_TYPE}'")),
            Self::Sheets => Some(format!("mimeType = '{GOOGLE_SHEET_MIME_TYPE}'")),
            Self::Slides => Some(format!("mimeType = '{GOOGLE_SLIDES_MIME_TYPE}'")),
        }
    }

    fn order_by(self) -> &'static str {
        match self {
            Self::Browse => "name",
            Self::Files | Self::Folders | Self::Docs | Self::Sheets | Self::Slides => {
                "modifiedTime desc"
            }
        }
    }
}

fn escape_query_literal(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "\\'")
}

fn parent_query_filter(parent_id: &str) -> String {
    format!("'{}' in parents", escape_query_literal(parent_id))
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

pub async fn create_folder<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &CreateFolderOptions,
) -> Result<CreatedFolder, DriveError> {
    let metadata = CreateFolderMetadata {
        name: &options.name,
        mime_type: DRIVE_FOLDER_MIME_TYPE,
        parents: [&options.parent_folder],
    };
    let response = client
        .send_with_scopes(
            client.post(options.request_url()?).json(&metadata),
            DRIVE_SCOPES,
        )
        .await
        .map_err(DriveError::Auth)?;
    let response = ensure_success_response(response).await?;
    response
        .json::<CreatedFolder>()
        .await
        .map_err(|error| DriveError::InvalidResponse(error.to_string()))
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
    let metadata = fetch_metadata(client, options).await?;
    let export_format = GoogleFileExportFormat::for_google_mime_type(&metadata.mime_type);
    let path = match &options.output {
        Some(output) => output.clone(),
        None => std::env::current_dir()
            .map_err(DriveError::Io)?
            .join(default_download_name(&metadata.name, export_format)),
    };

    if let Some(format) = export_format {
        let export_options = ExportGoogleFileOptions::new(&options.file_id, format, path)
            .with_files_url(&options.files_url);
        return export_google_file(client, &export_options, progress).await;
    }

    let response = client
        .send_with_scopes(client.get(options.media_url()?), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;

    let mut response = ensure_success_response(response).await?;
    let mut file = tokio::fs::File::create(&path)
        .await
        .map_err(DriveError::Io)?;
    let mut bytes = 0_u64;

    while let Some(chunk) = response.chunk().await.map_err(DriveError::Network)? {
        file.write_all(&chunk).await.map_err(DriveError::Io)?;
        bytes += chunk.len() as u64;
        progress(bytes);
    }

    file.flush().await.map_err(DriveError::Io)?;
    Ok(DownloadedFile { path, bytes })
}

fn default_download_name(name: &str, export_format: Option<GoogleFileExportFormat>) -> String {
    let Some(export_format) = export_format else {
        return name.to_string();
    };
    let extension = export_format.file_extension();
    if Path::new(name)
        .extension()
        .is_some_and(|value| value.eq_ignore_ascii_case(extension))
    {
        name.to_string()
    } else {
        format!("{name}.{extension}")
    }
}

pub async fn export_google_file<S, F>(
    client: &AuthClient<'_, S>,
    options: &ExportGoogleFileOptions,
    mut progress: F,
) -> Result<DownloadedFile, DriveError>
where
    S: AccountStore,
    F: FnMut(u64),
{
    let response = client
        .send_with_scopes(client.get(options.export_url()?), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;

    let mut response = ensure_success_response(response).await?;
    if response
        .content_length()
        .is_some_and(|length| length > options.max_download_bytes as u64)
    {
        return Err(export_size_error(options.max_download_bytes));
    }
    let output_parent = options
        .output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let temporary_file = tempfile::NamedTempFile::new_in(output_parent).map_err(DriveError::Io)?;
    let (temporary_file, temporary_path) = temporary_file.into_parts();
    let mut file = tokio::fs::File::from_std(temporary_file);
    let mut bytes = 0_u64;
    let mut signature = Vec::with_capacity(5);

    while let Some(chunk) = response.chunk().await.map_err(DriveError::Network)? {
        let next_bytes = bytes
            .checked_add(chunk.len() as u64)
            .ok_or_else(|| DriveError::InvalidResponse("export size overflow".into()))?;
        if next_bytes > options.max_download_bytes as u64 {
            return Err(export_size_error(options.max_download_bytes));
        }
        file.write_all(&chunk).await.map_err(DriveError::Io)?;
        let remaining_signature_bytes = 5_usize.saturating_sub(signature.len());
        signature.extend_from_slice(&chunk[..chunk.len().min(remaining_signature_bytes)]);
        bytes = next_bytes;
        progress(bytes);
    }

    file.flush().await.map_err(DriveError::Io)?;
    if !options.format.has_valid_signature(&signature) {
        return Err(DriveError::InvalidResponse(format!(
            "Google Drive returned an invalid {} export",
            options.format.display_name()
        )));
    }
    file.sync_all().await.map_err(DriveError::Io)?;
    drop(file);
    temporary_path
        .persist(&options.output)
        .map_err(|error| DriveError::Io(error.error))?;

    Ok(DownloadedFile {
        path: options.output.clone(),
        bytes,
    })
}

fn export_size_error(max_download_bytes: usize) -> DriveError {
    DriveError::InvalidResponse(format!(
        "Google Drive export exceeds the {max_download_bytes}-byte download limit"
    ))
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
         Content-Type: {}\r\n\r\n",
        options.mime_type
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
        let chunk_size = next_resumable_chunk_size(uploaded, file_size);
        let start = uploaded;
        let end = uploaded + chunk_size as u64 - 1;
        let body = resumable_chunk_body(&options.path, start, chunk_size).await?;
        let response = client
            .send_with_scopes(
                client
                    .put(session_uri.as_str())
                    .header(CONTENT_LENGTH, chunk_size.to_string())
                    .header(CONTENT_RANGE, format!("bytes {start}-{end}/{file_size}"))
                    .body(body),
                DRIVE_SCOPES,
            )
            .await
            .map_err(DriveError::Auth)?;

        uploaded += chunk_size as u64;
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

async fn resumable_chunk_body(path: &Path, start: u64, length: usize) -> Result<Body, DriveError> {
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
                .header(UPLOAD_CONTENT_TYPE_HEADER, &options.mime_type)
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
        mime_type: options.mime_type.clone(),
        parents: options.folder.as_ref().map(|folder| vec![folder.clone()]),
    })
}

pub async fn create_anyone_reader_permission<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DriveFileOperationOptions,
) -> Result<(), DriveError> {
    let response = client
        .send_with_scopes(
            client
                .post(options.permissions_url()?)
                .header(CONTENT_TYPE, JSON_CONTENT_TYPE)
                .json(&serde_json::json!({"type": "anyone", "role": "reader"})),
            DRIVE_SCOPES,
        )
        .await
        .map_err(DriveError::Auth)?;
    ensure_success_response(response).await?;
    Ok(())
}

pub async fn delete_file<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DriveFileOperationOptions,
) -> Result<(), DriveError> {
    let response = client
        .send_with_scopes(client.delete(options.file_url()?), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;
    ensure_success_response(response).await?;
    Ok(())
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
