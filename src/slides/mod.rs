pub mod error;

pub use error::SlidesError;

use reqwest::{Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const SLIDES_SCOPE: &str = "https://www.googleapis.com/auth/presentations";
pub const SLIDES_SCOPES: &[&str] = &[SLIDES_SCOPE];
const SLIDES_PRESENTATIONS_URL: &str = "https://slides.googleapis.com/v1/presentations";

pub type Presentation = Value;
pub type CreatePresentationResponse = Value;
pub type BatchUpdatePresentationResponse = Value;

/// Accepts either a bare Google Slides Presentation ID or a Google
/// Slides/Drive URL pointing at one, and returns the extracted Presentation ID.
pub fn extract_presentation_id(input: &str) -> String {
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
pub struct CreatePresentationOptions {
    pub title: String,
    presentations_url: String,
}

impl CreatePresentationOptions {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            presentations_url: SLIDES_PRESENTATIONS_URL.to_string(),
        }
    }

    pub(super) fn with_presentations_url(mut self, presentations_url: impl Into<String>) -> Self {
        self.presentations_url = presentations_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SlidesError> {
        Ok(Url::parse(&self.presentations_url)?)
    }
}

#[derive(Debug, Clone)]
pub struct GetPresentationOptions {
    pub presentation_id: String,
    pub fields: Option<String>,
    presentations_url: String,
}

impl GetPresentationOptions {
    pub fn new(presentation_id: impl Into<String>) -> Self {
        Self {
            presentation_id: presentation_id.into(),
            fields: None,
            presentations_url: SLIDES_PRESENTATIONS_URL.to_string(),
        }
    }

    pub fn with_fields(mut self, fields: impl Into<String>) -> Self {
        self.fields = Some(fields.into());
        self
    }

    pub(super) fn with_presentations_url(mut self, presentations_url: impl Into<String>) -> Self {
        self.presentations_url = presentations_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SlidesError> {
        let mut url = presentation_url(&self.presentations_url, &self.presentation_id)?;
        if let Some(fields) = &self.fields {
            url.query_pairs_mut().append_pair("fields", fields);
        }
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct BatchUpdatePresentationOptions {
    pub presentation_id: String,
    pub request_body: Value,
    presentations_url: String,
}

impl BatchUpdatePresentationOptions {
    pub fn new(presentation_id: impl Into<String>, request_body: Value) -> Self {
        Self {
            presentation_id: presentation_id.into(),
            request_body,
            presentations_url: SLIDES_PRESENTATIONS_URL.to_string(),
        }
    }

    pub(super) fn with_presentations_url(mut self, presentations_url: impl Into<String>) -> Self {
        self.presentations_url = presentations_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SlidesError> {
        presentation_url(
            &self.presentations_url,
            &format!("{}:batchUpdate", self.presentation_id),
        )
    }
}

fn presentation_url(presentations_url: &str, presentation_path: &str) -> Result<Url, SlidesError> {
    let mut url = Url::parse(presentations_url)?;
    url.path_segments_mut()
        .map_err(|_| SlidesError::InvalidResponse("Google Slides API URL cannot be a base".into()))?
        .push(presentation_path);
    Ok(url)
}

pub async fn create_presentation<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &CreatePresentationOptions,
) -> Result<CreatePresentationResponse, SlidesError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .json(&serde_json::json!({ "title": options.title })),
    )
    .await
}

pub async fn get_presentation<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetPresentationOptions,
) -> Result<Presentation, SlidesError> {
    send_json_request(client, client.get(options.request_url()?)).await
}

pub async fn batch_update_presentation<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchUpdatePresentationOptions,
) -> Result<BatchUpdatePresentationResponse, SlidesError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .json(&options.request_body),
    )
    .await
}

async fn send_json_request<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: reqwest::RequestBuilder,
) -> Result<Value, SlidesError> {
    let response = client
        .send_with_scopes(request, SLIDES_SCOPES)
        .await
        .map_err(SlidesError::Auth)?;

    parse_json_response(response).await
}

async fn parse_json_response(response: Response) -> Result<Value, SlidesError> {
    let status = response.status();
    if status.is_success() {
        return response
            .json()
            .await
            .map_err(|e| SlidesError::InvalidResponse(e.to_string()));
    }

    let body = response.text().await.unwrap_or_default();
    match status {
        StatusCode::NOT_FOUND => Err(SlidesError::NotFound),
        StatusCode::FORBIDDEN => Err(SlidesError::PermissionDenied),
        _ => Err(SlidesError::Api { status, body }),
    }
}
