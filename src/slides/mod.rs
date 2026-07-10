pub mod error;

#[cfg(test)]
mod tests;

pub use error::SlidesError;

use reqwest::{Response, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const SLIDES_SCOPE: &str = "https://www.googleapis.com/auth/presentations";
pub const SLIDES_SCOPES: &[&str] = &[SLIDES_SCOPE];
const SLIDES_PRESENTATIONS_URL: &str = "https://slides.googleapis.com/v1/presentations";
const DEFAULT_MAX_THUMBNAIL_BYTES: usize = 16 * 1024 * 1024;
const DEFAULT_MAX_THUMBNAIL_PIXELS: u64 = 16_000_000;

pub type Presentation = Value;
pub type CreatePresentationResponse = Value;
pub type BatchUpdatePresentationResponse = Value;

#[derive(Debug, Clone)]
pub struct GetPageThumbnailOptions {
    pub presentation_id: String,
    pub page_object_id: String,
    max_download_bytes: usize,
    max_pixel_count: u64,
    presentations_url: String,
    #[cfg(test)]
    allow_insecure_content_url: bool,
}

impl GetPageThumbnailOptions {
    pub fn new(presentation_id: impl Into<String>, page_object_id: impl Into<String>) -> Self {
        Self {
            presentation_id: presentation_id.into(),
            page_object_id: page_object_id.into(),
            max_download_bytes: DEFAULT_MAX_THUMBNAIL_BYTES,
            max_pixel_count: DEFAULT_MAX_THUMBNAIL_PIXELS,
            presentations_url: SLIDES_PRESENTATIONS_URL.to_string(),
            #[cfg(test)]
            allow_insecure_content_url: false,
        }
    }

    pub fn with_max_download_bytes(mut self, max_download_bytes: usize) -> Self {
        self.max_download_bytes = max_download_bytes;
        self
    }

    pub fn with_max_pixel_count(mut self, max_pixel_count: u64) -> Self {
        self.max_pixel_count = max_pixel_count;
        self
    }

    #[cfg(test)]
    fn with_presentations_url(mut self, presentations_url: impl Into<String>) -> Self {
        self.presentations_url = presentations_url.into();
        self
    }

    #[cfg(test)]
    fn allow_insecure_content_url_for_tests(mut self) -> Self {
        self.allow_insecure_content_url = true;
        self
    }

    fn request_url(&self) -> Result<Url, SlidesError> {
        let mut url = Url::parse(&self.presentations_url)?;
        url.path_segments_mut()
            .map_err(|_| {
                SlidesError::InvalidResponse("Google Slides API URL cannot be a base".into())
            })?
            .push(&self.presentation_id)
            .push("pages")
            .push(&self.page_object_id)
            .push("thumbnail");
        url.query_pairs_mut()
            .append_pair("thumbnailProperties.mimeType", "PNG")
            .append_pair("thumbnailProperties.thumbnailSize", "LARGE");
        Ok(url)
    }

    fn content_url(&self, content_url: &str) -> Result<Url, SlidesError> {
        let url = Url::parse(content_url)?;
        if url.scheme() == "https" {
            return Ok(url);
        }
        #[cfg(test)]
        if self.allow_insecure_content_url {
            return Ok(url);
        }
        Err(SlidesError::InvalidResponse(
            "Google Slides thumbnail content URL must use HTTPS".into(),
        ))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageThumbnail {
    pub width: u32,
    pub height: u32,
    pub bytes: bytes::Bytes,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PageThumbnailMetadata {
    width: u32,
    height: u32,
    content_url: String,
}

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

/// Fetches one LARGE PNG thumbnail attempt.
///
/// Deck inspection owns retry timing because it coordinates the expected slide
/// set and the presentation-wide consistency deadline.
pub async fn fetch_page_thumbnail_once<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetPageThumbnailOptions,
) -> Result<PageThumbnail, SlidesError> {
    let metadata_response = client
        .send_with_scopes(client.get(options.request_url()?), SLIDES_SCOPES)
        .await
        .map_err(SlidesError::Auth)?;
    let metadata = parse_thumbnail_metadata(metadata_response).await?;
    validate_thumbnail_metadata(&metadata, options.max_pixel_count)?;
    let content_url = options.content_url(&metadata.content_url)?;

    let image_response = client
        .get(content_url)
        .send()
        .await
        .map_err(|error| SlidesError::Network(error.to_string()))?;
    let image_response = ensure_success_response(image_response).await?;
    let bytes = read_bounded_bytes(image_response, options.max_download_bytes).await?;
    if bytes.is_empty() {
        return Err(SlidesError::InvalidResponse(
            "Google Slides returned an empty page thumbnail".into(),
        ));
    }

    Ok(PageThumbnail {
        width: metadata.width,
        height: metadata.height,
        bytes,
    })
}

fn validate_thumbnail_metadata(
    metadata: &PageThumbnailMetadata,
    max_pixel_count: u64,
) -> Result<(), SlidesError> {
    let pixel_count = u64::from(metadata.width) * u64::from(metadata.height);
    if metadata.width == 0 || metadata.height == 0 || pixel_count > max_pixel_count {
        return Err(SlidesError::InvalidResponse(format!(
            "Google Slides returned unsupported thumbnail dimensions {}x{}",
            metadata.width, metadata.height
        )));
    }
    Ok(())
}

async fn read_bounded_bytes(
    mut response: Response,
    max_download_bytes: usize,
) -> Result<bytes::Bytes, SlidesError> {
    if response
        .content_length()
        .is_some_and(|length| length > max_download_bytes as u64)
    {
        return Err(SlidesError::InvalidResponse(format!(
            "Google Slides thumbnail exceeds the {max_download_bytes}-byte download limit"
        )));
    }

    let mut bytes = bytes::BytesMut::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| SlidesError::Network(error.to_string()))?
    {
        let next_length = bytes
            .len()
            .checked_add(chunk.len())
            .ok_or_else(|| SlidesError::InvalidResponse("thumbnail size overflow".into()))?;
        if next_length > max_download_bytes {
            return Err(SlidesError::InvalidResponse(format!(
                "Google Slides thumbnail exceeds the {max_download_bytes}-byte download limit"
            )));
        }
        bytes.extend_from_slice(&chunk);
    }
    Ok(bytes.freeze())
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
    let response = ensure_success_response(response).await?;
    response
        .json()
        .await
        .map_err(|e| SlidesError::InvalidResponse(e.to_string()))
}

async fn parse_thumbnail_metadata(
    response: Response,
) -> Result<PageThumbnailMetadata, SlidesError> {
    let response = ensure_success_response(response).await?;
    response
        .json()
        .await
        .map_err(|error| SlidesError::InvalidResponse(error.to_string()))
}

async fn ensure_success_response(response: Response) -> Result<Response, SlidesError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    let body = response.text().await.unwrap_or_default();
    match status {
        StatusCode::NOT_FOUND => Err(SlidesError::NotFound),
        StatusCode::FORBIDDEN => Err(SlidesError::PermissionDenied),
        _ => Err(SlidesError::Api { status, body }),
    }
}
