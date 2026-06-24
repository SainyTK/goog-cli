pub mod error;

pub use error::MailError;

use reqwest::{Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const GMAIL_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";
pub const GMAIL_READONLY_SCOPES: &[&str] = &[GMAIL_READONLY_SCOPE];
const GMAIL_MESSAGES_URL: &str = "https://gmail.googleapis.com/gmail/v1/users/me/messages";

pub type Message = Value;

#[derive(Debug, Clone)]
pub struct GetMessageOptions {
    pub message_id: String,
    messages_url: String,
}

impl GetMessageOptions {
    pub fn new(message_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            messages_url: GMAIL_MESSAGES_URL.to_string(),
        }
    }

    pub(super) fn with_messages_url(mut self, messages_url: impl Into<String>) -> Self {
        self.messages_url = messages_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, MailError> {
        let mut url = Url::parse(&self.messages_url)?;
        url.path_segments_mut()
            .map_err(|_| MailError::InvalidResponse("GoogleMail API URL cannot be a base".into()))?
            .push(&self.message_id);
        Ok(url)
    }
}

pub async fn get_message<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetMessageOptions,
) -> Result<Message, MailError> {
    let response = client
        .send_with_scopes(client.get(options.request_url()?), GMAIL_READONLY_SCOPES)
        .await
        .map_err(MailError::Auth)?;

    parse_message_response(response).await
}

async fn parse_message_response(response: Response) -> Result<Message, MailError> {
    let response = ensure_success_response(response).await?;
    response
        .json::<Value>()
        .await
        .map_err(|e| MailError::InvalidResponse(e.to_string()))
}

async fn ensure_success_response(response: Response) -> Result<Response, MailError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    match status {
        StatusCode::NOT_FOUND => Err(MailError::NotFound),
        StatusCode::FORBIDDEN => Err(MailError::PermissionDenied),
        status => {
            let body = response.text().await.unwrap_or_default();
            Err(MailError::Api { status, body })
        }
    }
}
