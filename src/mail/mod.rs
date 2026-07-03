pub mod error;

pub use error::MailError;

use base64::Engine;
use reqwest::{Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const GMAIL_SCOPE: &str = "https://www.googleapis.com/auth/gmail.modify";
pub const GMAIL_SCOPES: &[&str] = &[GMAIL_SCOPE];
const GMAIL_MESSAGES_URL: &str = "https://gmail.googleapis.com/gmail/v1/users/me/messages";
const MESSAGE_LIST_FIELDS: &str = "messages(id),nextPageToken";
const MESSAGE_METADATA_FIELDS: &str = "id,payload(headers(name,value))";

pub type Message = Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MessageSummary {
    #[serde(rename = "messageId")]
    pub id: String,
    pub date: String,
    pub from: String,
    pub subject: String,
}

#[derive(Debug, Default, Deserialize)]
struct MessagesPage {
    #[serde(default, deserialize_with = "deserialize_null_vec_as_empty")]
    messages: Vec<ListedMessage>,
}

#[derive(Debug, Deserialize)]
struct ListedMessage {
    id: String,
}

fn deserialize_null_vec_as_empty<'de, D, T>(deserializer: D) -> Result<Vec<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
struct MetadataMessage {
    #[serde(default)]
    id: String,
    #[serde(default)]
    payload: MessagePayload,
}

#[derive(Debug, Default, Deserialize)]
struct MessagePayload {
    #[serde(default)]
    headers: Vec<MessageHeader>,
    #[serde(default)]
    parts: Vec<MessagePart>,
    #[serde(default)]
    filename: String,
    #[serde(default)]
    body: Option<MessagePartBody>,
}

#[derive(Debug, Deserialize)]
struct MessageHeader {
    name: String,
    value: String,
}

#[derive(Debug, Default, Deserialize)]
struct MessagePart {
    #[serde(default)]
    filename: String,
    #[serde(default)]
    headers: Vec<MessageHeader>,
    #[serde(default)]
    parts: Vec<MessagePart>,
    body: Option<MessagePartBody>,
}

#[derive(Debug, Deserialize)]
struct MessagePartBody {
    #[serde(rename = "attachmentId")]
    attachment_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DownloadedAttachment {
    pub path: PathBuf,
    pub bytes: u64,
}

#[derive(Debug, Deserialize)]
struct AttachmentPayload {
    data: String,
}

#[derive(Debug, Clone)]
pub struct DownloadAttachmentOptions {
    pub message_id: String,
    pub attachment_id: String,
    pub output: Option<PathBuf>,
    messages_url: String,
}

impl DownloadAttachmentOptions {
    pub fn new(message_id: impl Into<String>, attachment_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            attachment_id: attachment_id.into(),
            output: None,
            messages_url: GMAIL_MESSAGES_URL.to_string(),
        }
    }

    pub fn with_output(mut self, output: impl Into<PathBuf>) -> Self {
        self.output = Some(output.into());
        self
    }

    pub(super) fn with_messages_url(mut self, messages_url: impl Into<String>) -> Self {
        self.messages_url = messages_url.into();
        self
    }

    fn attachment_url(&self) -> Result<Url, MailError> {
        let mut url = message_url(&self.messages_url, &self.message_id)?;
        url.path_segments_mut()
            .map_err(|_| MailError::InvalidResponse("GoogleMail API URL cannot be a base".into()))?
            .push("attachments")
            .push(&self.attachment_id);
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct ListMessagesOptions {
    pub page_size: u32,
    pub query: Option<String>,
    pub inbox_only: bool,
    messages_url: String,
}

impl ListMessagesOptions {
    pub fn inbox(page_size: u32) -> Self {
        Self {
            page_size,
            query: None,
            inbox_only: true,
            messages_url: GMAIL_MESSAGES_URL.to_string(),
        }
    }

    pub fn search(query: impl Into<String>, page_size: u32) -> Self {
        Self {
            page_size,
            query: Some(query.into()),
            inbox_only: false,
            messages_url: GMAIL_MESSAGES_URL.to_string(),
        }
    }

    pub(super) fn with_messages_url(mut self, messages_url: impl Into<String>) -> Self {
        self.messages_url = messages_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, MailError> {
        let mut url = Url::parse(&self.messages_url)?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("maxResults", &self.page_size.to_string())
                .append_pair("fields", MESSAGE_LIST_FIELDS);
            if self.inbox_only {
                query.append_pair("labelIds", "INBOX");
            }
            if let Some(mailbox_query) = &self.query {
                query.append_pair("q", mailbox_query);
            }
        }
        Ok(url)
    }
}

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
        message_url(&self.messages_url, &self.message_id)
    }
}

fn message_url(messages_url: &str, message_id: &str) -> Result<Url, MailError> {
    let mut url = Url::parse(messages_url)?;
    url.path_segments_mut()
        .map_err(|_| MailError::InvalidResponse("GoogleMail API URL cannot be a base".into()))?
        .push(message_id);
    Ok(url)
}

pub async fn get_message<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetMessageOptions,
) -> Result<Message, MailError> {
    let response = client
        .send_with_scopes(client.get(options.request_url()?), GMAIL_SCOPES)
        .await
        .map_err(MailError::Auth)?;

    parse_message_response(response).await
}

pub async fn list_messages<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListMessagesOptions,
) -> Result<Vec<MessageSummary>, MailError> {
    let response = client
        .send_with_scopes(client.get(options.request_url()?), GMAIL_SCOPES)
        .await
        .map_err(MailError::Auth)?;
    let page = parse_messages_page_response(response).await?;
    let mut summaries = Vec::with_capacity(page.messages.len());

    for message in page.messages {
        let metadata = fetch_message_metadata(client, &options.messages_url, &message.id).await?;
        summaries.push(summary_from_metadata(metadata));
    }

    Ok(summaries)
}

pub async fn download_attachment<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DownloadAttachmentOptions,
) -> Result<DownloadedAttachment, MailError> {
    let path = match &options.output {
        Some(output) => output.clone(),
        None => attachment_filename_path(client, options).await?,
    };
    ensure_destination_available(&path).await?;

    let response = client
        .send_with_scopes(client.get(options.attachment_url()?), GMAIL_SCOPES)
        .await
        .map_err(MailError::Auth)?;
    let payload: AttachmentPayload = parse_json_response(response).await?;
    let bytes = decode_base64url(&payload.data)?;

    let mut file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .await
        .map_err(MailError::Io)?;
    file.write_all(&bytes).await.map_err(MailError::Io)?;
    file.flush().await.map_err(MailError::Io)?;

    Ok(DownloadedAttachment {
        path,
        bytes: bytes.len() as u64,
    })
}

async fn ensure_destination_available(path: &Path) -> Result<(), MailError> {
    if tokio::fs::try_exists(path).await.map_err(MailError::Io)? {
        return Err(MailError::Io(std::io::Error::new(
            std::io::ErrorKind::AlreadyExists,
            format!("destination already exists: {}", path.display()),
        )));
    }
    Ok(())
}

async fn attachment_filename_path<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DownloadAttachmentOptions,
) -> Result<PathBuf, MailError> {
    let metadata = fetch_attachment_filename_metadata(client, options).await?;
    let filename = find_attachment_filename(&metadata.payload, &options.attachment_id)
        .ok_or(MailError::MissingAttachmentFilename)?;
    Ok(std::env::current_dir()
        .map_err(MailError::Io)?
        .join(filename))
}

async fn fetch_attachment_filename_metadata<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DownloadAttachmentOptions,
) -> Result<MetadataMessage, MailError> {
    let response = client
        .send_with_scopes(
            client.get(message_url(&options.messages_url, &options.message_id)?),
            GMAIL_SCOPES,
        )
        .await
        .map_err(MailError::Auth)?;
    parse_json_response(response).await
}

fn find_attachment_filename(payload: &MessagePayload, attachment_id: &str) -> Option<String> {
    let attachments = attachment_filenames(payload);

    filename_for_attachment(&attachments, attachment_id)
        .or_else(|| filename_from_single_attachment(&attachments))
}

fn filename_for_attachment(
    attachments: &[AttachmentFilename],
    attachment_id: &str,
) -> Option<String> {
    attachments.iter().find_map(|attachment| {
        if attachment.id == attachment_id {
            attachment.filename.clone()
        } else {
            None
        }
    })
}

fn attachment_filename_from_headers(headers: &[MessageHeader]) -> Option<String> {
    headers
        .iter()
        .filter(|header| {
            header.name.eq_ignore_ascii_case("content-disposition")
                || header.name.eq_ignore_ascii_case("content-type")
        })
        .find_map(|header| {
            header_parameter(&header.value, "filename")
                .or_else(|| header_parameter(&header.value, "name"))
        })
}

fn header_parameter(value: &str, parameter_name: &str) -> Option<String> {
    value
        .split(';')
        .skip(1)
        .filter_map(|parameter| parameter.split_once('='))
        .find_map(|(name, value)| {
            if name.trim().eq_ignore_ascii_case(parameter_name) {
                let value = value.trim().trim_matches('"').trim();
                (!value.is_empty()).then(|| value.to_string())
            } else {
                None
            }
        })
}

fn attachment_filenames(payload: &MessagePayload) -> Vec<AttachmentFilename> {
    let mut attachments = Vec::new();
    collect_attachment_filenames_from_node(
        &payload.filename,
        &payload.headers,
        payload.body.as_ref(),
        &payload.parts,
        &mut attachments,
    );
    attachments
}

fn filename_from_single_attachment(attachments: &[AttachmentFilename]) -> Option<String> {
    let [attachment] = attachments else {
        return None;
    };

    attachment.filename.clone()
}

struct AttachmentFilename {
    id: String,
    filename: Option<String>,
}

fn collect_attachment_filenames_from_part(
    part: &MessagePart,
    attachments: &mut Vec<AttachmentFilename>,
) {
    collect_attachment_filenames_from_node(
        &part.filename,
        &part.headers,
        part.body.as_ref(),
        &part.parts,
        attachments,
    );
}

fn collect_attachment_filenames_from_node(
    filename: &str,
    headers: &[MessageHeader],
    body: Option<&MessagePartBody>,
    parts: &[MessagePart],
    attachments: &mut Vec<AttachmentFilename>,
) {
    if let Some(attachment_id) = body.and_then(|body| body.attachment_id.as_deref()) {
        attachments.push(AttachmentFilename {
            id: attachment_id.to_string(),
            filename: part_filename(filename, headers),
        });
    }

    for child in parts {
        collect_attachment_filenames_from_part(child, attachments);
    }
}

fn part_filename(filename: &str, headers: &[MessageHeader]) -> Option<String> {
    if !filename.is_empty() {
        return Some(filename.to_string());
    }

    attachment_filename_from_headers(headers)
}

pub(crate) fn decode_base64url(data: &str) -> Result<Vec<u8>, MailError> {
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(data)
        .or_else(|_| base64::engine::general_purpose::URL_SAFE.decode(data))
        .map_err(|e| MailError::InvalidResponse(e.to_string()))
}

async fn fetch_message_metadata<S: AccountStore>(
    client: &AuthClient<'_, S>,
    messages_url: &str,
    message_id: &str,
) -> Result<MetadataMessage, MailError> {
    let mut url = message_url(messages_url, message_id)?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("format", "metadata")
            .append_pair("metadataHeaders", "Date")
            .append_pair("metadataHeaders", "From")
            .append_pair("metadataHeaders", "Subject")
            .append_pair("fields", MESSAGE_METADATA_FIELDS);
    }

    let response = client
        .send_with_scopes(client.get(url), GMAIL_SCOPES)
        .await
        .map_err(MailError::Auth)?;
    parse_json_response(response).await
}

fn summary_from_metadata(metadata: MetadataMessage) -> MessageSummary {
    MessageSummary {
        id: metadata.id,
        date: header_value(&metadata.payload.headers, "date").unwrap_or_default(),
        from: header_value(&metadata.payload.headers, "from").unwrap_or_default(),
        subject: header_value(&metadata.payload.headers, "subject").unwrap_or_default(),
    }
}

fn header_value(headers: &[MessageHeader], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|header| header.name.eq_ignore_ascii_case(name))
        .map(|header| header.value.clone())
}

async fn parse_messages_page_response(response: Response) -> Result<MessagesPage, MailError> {
    let response = ensure_success_response(response).await?;
    let body = response
        .text()
        .await
        .map_err(|e| MailError::InvalidResponse(e.to_string()))?;
    if body.trim().is_empty() {
        return Ok(MessagesPage::default());
    }
    serde_json::from_str(&body).map_err(|e| MailError::InvalidResponse(e.to_string()))
}

async fn parse_message_response(response: Response) -> Result<Message, MailError> {
    parse_json_response(response).await
}

async fn parse_json_response<T: DeserializeOwned>(response: Response) -> Result<T, MailError> {
    let response = ensure_success_response(response).await?;
    response
        .json::<T>()
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
