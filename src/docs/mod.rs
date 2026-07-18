pub mod change;
pub mod error;
pub(crate) mod image_fit;
#[cfg(test)]
mod image_fit_tests;
pub(crate) mod image_metadata;
#[cfg(test)]
mod image_metadata_tests;
pub(crate) mod image_staging;
#[cfg(test)]
mod image_staging_tests;
pub mod map;
pub(crate) mod page_layout;
#[cfg(test)]
mod page_layout_tests;
pub mod style_template;

#[cfg(test)]
mod change_tests;

#[cfg(test)]
mod style_template_tests;

pub use error::DocsError;
pub use style_template::{
    extract_style_template, load_style_template, save_style_template, StyleTemplate,
};

use std::future::Future;

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::drive::DRIVE_SCOPES;

pub const DOCS_SCOPE: &str = "https://www.googleapis.com/auth/documents";
pub const DOCS_SCOPES: &[&str] = &[DOCS_SCOPE];
const DOCS_DOCUMENTS_URL: &str = "https://docs.googleapis.com/v1/documents";
const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const OFFICE_FILE_PRECONDITION_STATUS: &str = "FAILED_PRECONDITION";
const OFFICE_FILE_PRECONDITION_MESSAGE: &str = "must not be an Office file";
const GOOGLE_DOC_MIME_TYPE: &str = "application/vnd.google-apps.document";
const TEMPORARY_CONVERSION_NAME: &str = "goog temporary Docs conversion";

pub type Document = Value;
pub type BatchUpdateResponse = Value;

/// Accepts either a bare Google Docs Document ID or a Google Docs/Drive URL
/// pointing at one, and returns the extracted Document ID.
pub fn extract_document_id(input: &str) -> String {
    let trimmed = input.trim();
    match Url::parse(trimmed)
        .ok()
        .and_then(|url| extract_id_from_url(&url))
    {
        Some(id) => id,
        None => trimmed.to_string(),
    }
}

fn extract_id_from_url(url: &Url) -> Option<String> {
    if let Some(segments) = url.path_segments() {
        let segments: Vec<&str> = segments.collect();
        if let Some(pos) = segments.iter().position(|segment| *segment == "d") {
            if let Some(id) = segments.get(pos + 1).filter(|id| !id.is_empty()) {
                return Some((*id).to_string());
            }
        }
    }

    url.query_pairs()
        .find(|(key, _)| key == "id")
        .map(|(_, value)| value.into_owned())
}

#[derive(Debug, Clone)]
pub struct GetDocumentOptions {
    pub document_id: String,
    pub fields: Option<String>,
    pub include_tabs_content: bool,
    documents_url: String,
    drive_files_url: String,
}

#[derive(Debug, Clone)]
pub struct CopyDocumentOptions {
    pub source_document_id: String,
    pub title: String,
    drive_files_url: String,
}

impl CopyDocumentOptions {
    pub fn new(source_document_id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            source_document_id: source_document_id.into(),
            title: title.into(),
            drive_files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub(super) fn with_drive_files_url(mut self, drive_files_url: impl Into<String>) -> Self {
        self.drive_files_url = drive_files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DocsError> {
        let mut url = drive_file_url(
            &self.drive_files_url,
            &self.source_document_id,
            Some("copy"),
        )?;
        url.query_pairs_mut()
            .append_pair("fields", "id,name,mimeType,webViewLink")
            .append_pair("supportsAllDrives", "true");
        Ok(url)
    }
}

impl GetDocumentOptions {
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
            fields: None,
            include_tabs_content: false,
            documents_url: DOCS_DOCUMENTS_URL.to_string(),
            drive_files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn with_fields(mut self, fields: impl Into<String>) -> Self {
        self.fields = Some(fields.into());
        self
    }

    pub fn with_include_tabs_content(mut self, include_tabs_content: bool) -> Self {
        self.include_tabs_content = include_tabs_content;
        self
    }

    pub(super) fn with_documents_url(mut self, documents_url: impl Into<String>) -> Self {
        self.documents_url = documents_url.into();
        self
    }

    pub(super) fn with_drive_files_url(mut self, drive_files_url: impl Into<String>) -> Self {
        self.drive_files_url = drive_files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DocsError> {
        let mut url = document_url(&self.documents_url, &self.document_id)?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(fields) = &self.fields {
                query.append_pair("fields", fields);
            }
            if self.include_tabs_content {
                query.append_pair("includeTabsContent", "true");
            }
        }
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct BatchUpdateDocumentOptions {
    pub document_id: String,
    pub request_body: Value,
    documents_url: String,
}

impl BatchUpdateDocumentOptions {
    pub fn new(document_id: impl Into<String>, request_body: Value) -> Self {
        Self {
            document_id: document_id.into(),
            request_body,
            documents_url: DOCS_DOCUMENTS_URL.to_string(),
        }
    }

    pub(super) fn with_documents_url(mut self, documents_url: impl Into<String>) -> Self {
        self.documents_url = documents_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DocsError> {
        document_url(
            &self.documents_url,
            &format!("{}:batchUpdate", self.document_id),
        )
    }
}

#[derive(Debug, Clone)]
pub struct CreateDocumentOptions {
    pub title: String,
    documents_url: String,
}

impl CreateDocumentOptions {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            documents_url: DOCS_DOCUMENTS_URL.to_string(),
        }
    }

    pub(super) fn with_documents_url(mut self, documents_url: impl Into<String>) -> Self {
        self.documents_url = documents_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DocsError> {
        Ok(Url::parse(&self.documents_url)?)
    }
}

fn document_url(documents_url: &str, document_path: &str) -> Result<Url, DocsError> {
    let mut url = Url::parse(documents_url)?;
    url.path_segments_mut()
        .map_err(|_| DocsError::InvalidResponse("Google Docs API URL cannot be a base".into()))?
        .push(document_path);
    Ok(url)
}

/// Fetches a Document through the Google Docs API. Google Docs cannot read
/// Office files (for example a .docx file stored on Drive) directly; when the
/// API reports that precondition, this transparently makes a temporary
/// native Google Docs copy through Drive, reads that instead, and deletes it
/// again before returning.
pub async fn get_document<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetDocumentOptions,
) -> Result<Document, DocsError> {
    send_json_request_with_office_file_fallback(
        client,
        client.get(options.request_url()?),
        DOCS_SCOPES,
        || get_document_via_temporary_conversion(client, options),
    )
    .await
}

/// Creates a new, blank Google Docs Document via the Docs API and returns
/// the created Document. The Document is always created at the root of the
/// active account's My Drive; move it into a folder afterward with Drive if
/// needed.
pub async fn create_document<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &CreateDocumentOptions,
) -> Result<Document, DocsError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .json(&serde_json::json!({ "title": options.title })),
        DOCS_SCOPES,
    )
    .await
}

/// Copies an existing native Google Doc through Drive so editor-only
/// components are preserved in the new document.
pub async fn copy_document<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &CopyDocumentOptions,
) -> Result<Document, DocsError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .json(&serde_json::json!({ "name": options.title })),
        DRIVE_SCOPES,
    )
    .await
}

pub async fn batch_update_document<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchUpdateDocumentOptions,
) -> Result<BatchUpdateResponse, DocsError> {
    let response = client
        .send_with_scopes(
            client
                .post(options.request_url()?)
                .json(&options.request_body),
            DOCS_SCOPES,
        )
        .await
        .map_err(DocsError::Auth)?;

    parse_json_response(response).await
}

async fn get_document_via_temporary_conversion<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetDocumentOptions,
) -> Result<Document, DocsError> {
    let temporary_id =
        create_temporary_google_doc(client, &options.drive_files_url, &options.document_id).await?;

    let mut temporary_options = GetDocumentOptions::new(temporary_id.clone())
        .with_include_tabs_content(options.include_tabs_content)
        .with_documents_url(options.documents_url.clone())
        .with_drive_files_url(options.drive_files_url.clone());
    if let Some(fields) = &options.fields {
        temporary_options = temporary_options.with_fields(fields.clone());
    }

    let response = send_json_request(
        client,
        client.get(temporary_options.request_url()?),
        DOCS_SCOPES,
    )
    .await;

    finish_temporary_conversion(client, &options.drive_files_url, &temporary_id, response).await
}

async fn finish_temporary_conversion<S: AccountStore>(
    client: &AuthClient<'_, S>,
    drive_files_url: &str,
    temporary_id: &str,
    response: Result<Value, DocsError>,
) -> Result<Value, DocsError> {
    let delete_result = delete_temporary_google_doc(client, drive_files_url, temporary_id).await;

    match (response, delete_result) {
        (Ok(response), Ok(())) => Ok(response),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

async fn create_temporary_google_doc<S: AccountStore>(
    client: &AuthClient<'_, S>,
    drive_files_url: &str,
    source_file_id: &str,
) -> Result<String, DocsError> {
    let mut url = drive_file_url(drive_files_url, source_file_id, Some("copy"))?;
    url.query_pairs_mut()
        .append_pair("fields", "id")
        .append_pair("supportsAllDrives", "true");
    let response = send_json_request(
        client,
        client.post(url).json(&serde_json::json!({
            "mimeType": GOOGLE_DOC_MIME_TYPE,
            "name": TEMPORARY_CONVERSION_NAME
        })),
        DRIVE_SCOPES,
    )
    .await?;

    response
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            DocsError::InvalidResponse("Google Drive copy response did not include an id".into())
        })
}

async fn delete_temporary_google_doc<S: AccountStore>(
    client: &AuthClient<'_, S>,
    drive_files_url: &str,
    file_id: &str,
) -> Result<(), DocsError> {
    let mut url = drive_file_url(drive_files_url, file_id, None)?;
    url.query_pairs_mut()
        .append_pair("supportsAllDrives", "true");
    send_empty_request(client, client.delete(url), DRIVE_SCOPES).await
}

fn drive_file_url(
    drive_files_url: &str,
    file_id: &str,
    suffix: Option<&str>,
) -> Result<Url, DocsError> {
    let mut url = Url::parse(drive_files_url)?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            DocsError::InvalidResponse("Google Drive API URL cannot be a base".into())
        })?;
        segments.push(file_id);
        if let Some(suffix) = suffix {
            segments.push(suffix);
        }
    }
    Ok(url)
}

async fn send_json_request<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: RequestBuilder,
    scopes: &[&str],
) -> Result<Value, DocsError> {
    let response = client
        .send_with_scopes(request, scopes)
        .await
        .map_err(DocsError::Auth)?;
    parse_json_response(response).await
}

async fn send_empty_request<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: RequestBuilder,
    scopes: &[&str],
) -> Result<(), DocsError> {
    let response = client
        .send_with_scopes(request, scopes)
        .await
        .map_err(DocsError::Auth)?;
    ensure_success_response(response).await?;
    Ok(())
}

async fn send_json_request_with_office_file_fallback<S, F, Fut>(
    client: &AuthClient<'_, S>,
    request: RequestBuilder,
    scopes: &[&str],
    fallback: F,
) -> Result<Value, DocsError>
where
    S: AccountStore,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Value, DocsError>>,
{
    let response = client
        .send_with_scopes(request, scopes)
        .await
        .map_err(DocsError::Auth)?;
    let status = response.status();

    if status.is_success() {
        return response
            .json::<Value>()
            .await
            .map_err(|e| DocsError::InvalidResponse(e.to_string()));
    }

    match status {
        StatusCode::NOT_FOUND => Err(DocsError::NotFound),
        StatusCode::FORBIDDEN => Err(DocsError::PermissionDenied),
        status => {
            let body = response.text().await.unwrap_or_default();
            if is_office_file_precondition_error(status, &body) {
                fallback().await
            } else {
                Err(DocsError::Api { status, body })
            }
        }
    }
}

async fn parse_json_response(response: Response) -> Result<Value, DocsError> {
    let response = ensure_success_response(response).await?;
    response
        .json::<Value>()
        .await
        .map_err(|e| DocsError::InvalidResponse(e.to_string()))
}

async fn ensure_success_response(response: Response) -> Result<Response, DocsError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    match status {
        StatusCode::NOT_FOUND => Err(DocsError::NotFound),
        StatusCode::FORBIDDEN => Err(DocsError::PermissionDenied),
        status => {
            let body = response.text().await.unwrap_or_default();
            if is_office_file_precondition_error(status, &body) {
                return Err(DocsError::UnsupportedOfficeFile);
            }
            Err(DocsError::Api { status, body })
        }
    }
}

fn is_office_file_precondition_error(status: StatusCode, body: &str) -> bool {
    if status != StatusCode::BAD_REQUEST {
        return false;
    }

    let Ok(response) = serde_json::from_str::<GoogleApiErrorResponse>(body) else {
        return false;
    };

    response.error.is_office_file_precondition()
}

#[derive(Debug, Deserialize)]
struct GoogleApiErrorResponse {
    error: GoogleApiError,
}

#[derive(Debug, Deserialize)]
struct GoogleApiError {
    message: String,
    status: String,
}

impl GoogleApiError {
    fn is_office_file_precondition(&self) -> bool {
        self.status == OFFICE_FILE_PRECONDITION_STATUS
            && self.message.contains(OFFICE_FILE_PRECONDITION_MESSAGE)
    }
}
