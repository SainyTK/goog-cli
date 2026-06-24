pub mod error;

pub use error::DocsError;

use reqwest::{Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const DOCS_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/documents.readonly";
pub const DOCS_READONLY_SCOPES: &[&str] = &[DOCS_READONLY_SCOPE];
const DOCS_DOCUMENTS_URL: &str = "https://docs.googleapis.com/v1/documents";

pub type Document = Value;

#[derive(Debug, Clone)]
pub struct GetDocumentOptions {
    pub document_id: String,
    documents_url: String,
}

impl GetDocumentOptions {
    pub fn new(document_id: impl Into<String>) -> Self {
        Self {
            document_id: document_id.into(),
            documents_url: DOCS_DOCUMENTS_URL.to_string(),
        }
    }

    pub(super) fn with_documents_url(mut self, documents_url: impl Into<String>) -> Self {
        self.documents_url = documents_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DocsError> {
        let mut url = Url::parse(&self.documents_url)?;
        url.path_segments_mut()
            .map_err(|_| {
                DocsError::InvalidResponse("Google Docs API URL cannot be a base".into())
            })?
            .push(&self.document_id);
        Ok(url)
    }
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

async fn parse_document_response(response: Response) -> Result<Document, DocsError> {
    let response = ensure_success_response(response).await?;
    response
        .json::<Document>()
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
