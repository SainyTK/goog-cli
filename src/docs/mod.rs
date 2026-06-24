pub mod error;

pub use error::DocsError;

use reqwest::{Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const DOCS_SCOPE: &str = "https://www.googleapis.com/auth/documents";
pub const DOCS_SCOPES: &[&str] = &[DOCS_SCOPE];
pub const DOCS_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/documents.readonly";
pub const DOCS_READONLY_SCOPES: &[&str] = &[DOCS_READONLY_SCOPE];
const DOCS_DOCUMENTS_URL: &str = "https://docs.googleapis.com/v1/documents";

pub type Document = Value;
pub type BatchUpdateResponse = Value;

#[derive(Debug, Clone)]
pub struct GetDocumentOptions {
    pub document_id: String,
    pub fields: Option<String>,
    pub include_tabs_content: bool,
    documents_url: String,
}

impl GetDocumentOptions {
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
            fields: None,
            include_tabs_content: false,
            documents_url: DOCS_DOCUMENTS_URL.to_string(),
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

fn document_url(documents_url: &str, document_path: &str) -> Result<Url, DocsError> {
    let mut url = Url::parse(documents_url)?;
    url.path_segments_mut()
        .map_err(|_| DocsError::InvalidResponse("Google Docs API URL cannot be a base".into()))?
        .push(document_path);
    Ok(url)
}

pub async fn get_document<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetDocumentOptions,
) -> Result<Document, DocsError> {
    let response = client
        .send_with_scopes(client.get(options.request_url()?), DOCS_READONLY_SCOPES)
        .await
        .map_err(DocsError::Auth)?;

    parse_document_response(response).await
}

pub async fn batch_update_document<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchUpdateDocumentOptions,
) -> Result<BatchUpdateResponse, DocsError> {
    let response = client
        .send_with_scopes(
            client.post(options.request_url()?).json(&options.request_body),
            DOCS_SCOPES,
        )
        .await
        .map_err(DocsError::Auth)?;

    parse_json_response(response).await
}

async fn parse_document_response(response: Response) -> Result<Document, DocsError> {
    parse_json_response(response).await
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
            Err(DocsError::Api { status, body })
        }
    }
}
