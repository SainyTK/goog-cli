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
const GMAIL_DRAFTS_URL: &str = "https://gmail.googleapis.com/gmail/v1/users/me/drafts";
const GMAIL_RESTRICTED_ALPHABET: &str = "BCDFGHJKLMNPQRSTVWXZbcdfghjklmnpqrstvwxz";
const BASE64_ALPHABET: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const MESSAGE_LIST_FIELDS: &str = "messages(id),nextPageToken";
const MESSAGE_METADATA_FIELDS: &str = "id,payload(headers(name,value))";

pub type Message = Value;
pub type Draft = Value;
const DRAFT_BOUNDARY: &str = "goog-cli-draft-boundary";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageReference {
    MessageId(String),
    Thread {
        thread_id: String,
        preferred_label: Option<String>,
    },
}

/// Accepts either a bare Gmail message ID or a Gmail browser URL.
/// Browser URLs may identify a thread token instead of a REST Message ID.
pub fn parse_message_reference(input: &str) -> MessageReference {
    let trimmed = input.trim();
    match Url::parse(trimmed)
        .ok()
        .and_then(|url| parse_message_reference_from_url(&url))
    {
        Some(reference) => reference,
        None => MessageReference::MessageId(trimmed.to_string()),
    }
}

fn parse_message_reference_from_url(url: &Url) -> Option<MessageReference> {
    let host = url.host_str()?;
    if host != "mail.google.com" {
        return None;
    }

    if let Some(id) = url.query_pairs().find_map(|(key, value)| {
        if key == "th" && !value.is_empty() {
            Some(value.into_owned())
        } else {
            None
        }
    }) {
        return Some(MessageReference::MessageId(id));
    }

    let fragment = url.fragment()?;
    let mut segments = fragment.split('/').filter(|segment| !segment.is_empty());
    let preferred_label = segments.next().map(gmail_label_id);
    let token = fragment.split('/').next_back()?.trim();
    if token.is_empty() {
        return None;
    }

    match decode_gmail_restricted_thread_token(token) {
        Some(thread_id) => Some(MessageReference::Thread {
            thread_id,
            preferred_label,
        }),
        None => Some(MessageReference::MessageId(token.to_string())),
    }
}

fn gmail_label_id(label: &str) -> String {
    label
        .trim_matches(|ch: char| ch == '^' || ch.is_whitespace())
        .to_ascii_uppercase()
}

fn decode_gmail_restricted_thread_token(token: &str) -> Option<String> {
    if token.is_empty()
        || token
            .chars()
            .any(|ch| !GMAIL_RESTRICTED_ALPHABET.contains(ch))
    {
        return None;
    }

    let encoded = transliterate(token, GMAIL_RESTRICTED_ALPHABET, BASE64_ALPHABET)?;
    let decoded = base64::engine::general_purpose::STANDARD_NO_PAD
        .decode(encoded)
        .ok()?;
    let decoded = String::from_utf8(decoded).ok()?;
    if decoded.starts_with("thread-") {
        Some(decoded)
    } else {
        Some(format!("thread-{decoded}"))
    }
}

fn transliterate(subject: &str, input_alphabet: &str, output_alphabet: &str) -> Option<String> {
    let input_chars: Vec<char> = input_alphabet.chars().collect();
    let output_chars: Vec<char> = output_alphabet.chars().collect();
    if subject.chars().all(|ch| ch == input_chars[0]) {
        return Some(output_chars[0].to_string());
    }

    let input_indices: Option<Vec<usize>> = subject
        .chars()
        .rev()
        .map(|ch| input_chars.iter().position(|candidate| *candidate == ch))
        .collect();
    let input_indices = input_indices?;
    let input_base = input_chars.len();
    let output_base = output_chars.len();
    let mut output_indices = Vec::<usize>::new();

    for input_index in input_indices.iter().rev() {
        let mut offset = 0usize;
        for output_index in &mut output_indices {
            let mut index = *output_index * input_base + offset;
            if index >= output_base {
                offset = index / output_base;
                index %= output_base;
            } else {
                offset = 0;
            }
            *output_index = index;
        }
        while offset > 0 {
            output_indices.push(offset % output_base);
            offset /= output_base;
        }

        offset = *input_index;
        let mut output_position = 0usize;
        while offset > 0 {
            if output_position >= output_indices.len() {
                output_indices.push(0);
            }
            let mut index = output_indices[output_position] + offset;
            if index >= output_base {
                offset = index / output_base;
                index %= output_base;
            } else {
                offset = 0;
            }
            output_indices[output_position] = index;
            output_position += 1;
        }
    }

    output_indices
        .iter()
        .rev()
        .map(|index| output_chars.get(*index).copied())
        .collect()
}

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
struct ThreadMessagePage {
    #[serde(default, deserialize_with = "deserialize_null_vec_as_empty")]
    messages: Vec<ThreadMessage>,
}

#[derive(Debug, Deserialize)]
struct ThreadMessage {
    id: String,
    #[serde(default, rename = "labelIds")]
    label_ids: Vec<String>,
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
    pub attachment_id: Option<String>,
    pub output: Option<PathBuf>,
    messages_url: String,
}

impl DownloadAttachmentOptions {
    pub fn new(message_id: impl Into<String>, attachment_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            attachment_id: Some(attachment_id.into()),
            output: None,
            messages_url: GMAIL_MESSAGES_URL.to_string(),
        }
    }

    pub fn without_attachment_id(message_id: impl Into<String>) -> Self {
        Self {
            message_id: message_id.into(),
            attachment_id: None,
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

    fn attachment_url(&self, attachment_id: &str) -> Result<Url, MailError> {
        let mut url = message_url(&self.messages_url, &self.message_id)?;
        url.path_segments_mut()
            .map_err(|_| MailError::InvalidResponse("Gmail API URL cannot be a base".into()))?
            .push("attachments")
            .push(attachment_id);
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct CreateDraftOptions {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub body_format: DraftBodyFormat,
    pub attachments: Vec<DraftAttachment>,
    drafts_url: String,
}

#[derive(Debug, Clone)]
pub struct UpdateDraftOptions {
    pub draft_id: String,
    pub message: DraftMessage,
    drafts_url: String,
}

#[derive(Debug, Clone)]
pub struct DraftMessage {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: String,
    pub body_format: DraftBodyFormat,
    pub attachments: Vec<DraftAttachment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DraftBodyFormat {
    PlainText,
    Html,
}

impl DraftBodyFormat {
    fn mime_type(self) -> &'static str {
        match self {
            Self::PlainText => "text/plain",
            Self::Html => "text/html",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
}

impl CreateDraftOptions {
    pub fn new(
        to: Vec<String>,
        cc: Vec<String>,
        bcc: Vec<String>,
        subject: impl Into<String>,
        body: impl Into<String>,
    ) -> Self {
        Self {
            to,
            cc,
            bcc,
            subject: subject.into(),
            body: body.into(),
            body_format: DraftBodyFormat::PlainText,
            attachments: Vec::new(),
            drafts_url: GMAIL_DRAFTS_URL.to_string(),
        }
    }

    pub fn with_attachments(mut self, attachments: Vec<DraftAttachment>) -> Self {
        self.attachments = attachments;
        self
    }

    pub fn with_body_format(mut self, body_format: DraftBodyFormat) -> Self {
        self.body_format = body_format;
        self
    }

    pub(super) fn with_drafts_url(mut self, drafts_url: impl Into<String>) -> Self {
        self.drafts_url = drafts_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, MailError> {
        Ok(Url::parse(&self.drafts_url)?)
    }

    fn request_body(&self) -> Result<Value, MailError> {
        let raw = build_draft_raw_message(&DraftMessage {
            to: self.to.clone(),
            cc: self.cc.clone(),
            bcc: self.bcc.clone(),
            subject: self.subject.clone(),
            body: self.body.clone(),
            body_format: self.body_format,
            attachments: self.attachments.clone(),
        })?;
        Ok(serde_json::json!({
            "message": {
                "raw": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
            }
        }))
    }
}

impl UpdateDraftOptions {
    pub fn new(draft_id: impl Into<String>, message: DraftMessage) -> Self {
        Self {
            draft_id: draft_id.into(),
            message,
            drafts_url: GMAIL_DRAFTS_URL.to_string(),
        }
    }

    pub(super) fn with_drafts_url(mut self, drafts_url: impl Into<String>) -> Self {
        self.drafts_url = drafts_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, MailError> {
        draft_url(&self.drafts_url, &self.draft_id)
    }

    fn request_body(&self) -> Result<Value, MailError> {
        let raw = build_draft_raw_message(&self.message)?;
        Ok(serde_json::json!({
            "id": self.draft_id,
            "message": {
                "raw": base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
            }
        }))
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
        .map_err(|_| MailError::InvalidResponse("Gmail API URL cannot be a base".into()))?
        .push(message_id);
    Ok(url)
}

fn draft_url(drafts_url: &str, draft_id: &str) -> Result<Url, MailError> {
    let mut url = Url::parse(drafts_url)?;
    url.path_segments_mut()
        .map_err(|_| MailError::InvalidResponse("Gmail API URL cannot be a base".into()))?
        .push(draft_id);
    Ok(url)
}

fn thread_url(messages_url: &str, thread_id: &str) -> Result<Url, MailError> {
    let mut url = Url::parse(messages_url)?;
    {
        let mut segments = url
            .path_segments_mut()
            .map_err(|_| MailError::InvalidResponse("Gmail API URL cannot be a base".into()))?;
        segments.pop();
        segments.push("threads").push(thread_id);
    }
    url.query_pairs_mut()
        .append_pair("fields", "messages(id,labelIds)");
    Ok(url)
}

pub async fn resolve_message_reference<S: AccountStore>(
    client: &AuthClient<'_, S>,
    reference: &MessageReference,
    messages_url: Option<&str>,
) -> Result<String, MailError> {
    match reference {
        MessageReference::MessageId(message_id) => Ok(message_id.clone()),
        MessageReference::Thread {
            thread_id,
            preferred_label,
        } => {
            let messages_url = messages_url.unwrap_or(GMAIL_MESSAGES_URL);
            let response = client
                .send_with_scopes(
                    client.get(thread_url(messages_url, thread_id)?),
                    GMAIL_SCOPES,
                )
                .await
                .map_err(MailError::Auth)?;
            let thread: ThreadMessagePage = parse_json_response(response).await?;
            select_thread_message_id(&thread.messages, preferred_label.as_deref())
                .ok_or(MailError::NotFound)
        }
    }
}

fn select_thread_message_id(
    messages: &[ThreadMessage],
    preferred_label: Option<&str>,
) -> Option<String> {
    if let Some(preferred_label) = preferred_label {
        if preferred_label != "ALL" {
            if let Some(message) = messages.iter().rev().find(|message| {
                message
                    .label_ids
                    .iter()
                    .any(|label| label == preferred_label)
            }) {
                return Some(message.id.clone());
            }
        }
    }

    messages.last().map(|message| message.id.clone())
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

pub async fn create_draft<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &CreateDraftOptions,
) -> Result<Draft, MailError> {
    let response = client
        .send_with_scopes(
            client
                .post(options.request_url()?)
                .json(&options.request_body()?),
            GMAIL_SCOPES,
        )
        .await
        .map_err(MailError::Auth)?;

    parse_json_response(response).await
}

pub async fn update_draft<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &UpdateDraftOptions,
) -> Result<Draft, MailError> {
    let response = client
        .send_with_scopes(
            client
                .put(options.request_url()?)
                .json(&options.request_body()?),
            GMAIL_SCOPES,
        )
        .await
        .map_err(MailError::Auth)?;

    parse_json_response(response).await
}

pub async fn download_attachment<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DownloadAttachmentOptions,
) -> Result<DownloadedAttachment, MailError> {
    let resolved = resolve_attachment_download(client, options).await?;
    let path = resolved.path;
    ensure_destination_available(&path).await?;

    let response = client
        .send_with_scopes(
            client.get(options.attachment_url(&resolved.attachment_id)?),
            GMAIL_SCOPES,
        )
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

fn build_draft_raw_message(message: &DraftMessage) -> Result<String, MailError> {
    if message.to.is_empty() {
        return Err(MailError::InvalidInput(
            "at least one To recipient is required".into(),
        ));
    }

    let mut raw = draft_headers(message)?;
    raw.push_str("MIME-Version: 1.0\r\n");

    if message.attachments.is_empty() {
        raw.push_str(&format!(
            "Content-Type: {}; charset=UTF-8\r\n",
            message.body_format.mime_type()
        ));
        raw.push_str("Content-Transfer-Encoding: 8bit\r\n");
        raw.push_str("\r\n");
        raw.push_str(&normalize_body_newlines(&message.body));
        return Ok(raw);
    }

    raw.push_str(&format!(
        "Content-Type: multipart/mixed; boundary=\"{DRAFT_BOUNDARY}\"\r\n"
    ));
    raw.push_str("\r\n");
    raw.push_str(&format!("--{DRAFT_BOUNDARY}\r\n"));
    raw.push_str(&format!(
        "Content-Type: {}; charset=UTF-8\r\n",
        message.body_format.mime_type()
    ));
    raw.push_str("Content-Transfer-Encoding: 8bit\r\n");
    raw.push_str("\r\n");
    raw.push_str(&normalize_body_newlines(&message.body));
    for attachment in &message.attachments {
        raw.push_str(&draft_attachment_part(attachment)?);
    }
    raw.push_str(&format!("--{DRAFT_BOUNDARY}--\r\n"));
    Ok(raw)
}

fn draft_headers(options: &DraftMessage) -> Result<String, MailError> {
    let mut message = String::new();
    push_address_header(&mut message, "To", &options.to)?;
    push_address_header(&mut message, "Cc", &options.cc)?;
    push_address_header(&mut message, "Bcc", &options.bcc)?;
    push_single_header(&mut message, "Subject", &options.subject)?;
    Ok(message)
}

fn draft_attachment_part(attachment: &DraftAttachment) -> Result<String, MailError> {
    reject_header_newlines("Attachment filename", &attachment.filename)?;
    reject_header_newlines("Attachment content type", &attachment.content_type)?;
    let filename = quote_header_parameter(&attachment.filename);
    let mut part = String::new();
    part.push_str(&format!("--{DRAFT_BOUNDARY}\r\n"));
    part.push_str(&format!(
        "Content-Type: {}; name=\"{}\"\r\n",
        attachment.content_type, filename
    ));
    part.push_str(&format!(
        "Content-Disposition: attachment; filename=\"{}\"\r\n",
        filename
    ));
    part.push_str("Content-Transfer-Encoding: base64\r\n");
    part.push_str("\r\n");
    part.push_str(&wrap_base64(
        &base64::engine::general_purpose::STANDARD.encode(&attachment.data),
    ));
    Ok(part)
}

fn quote_header_parameter(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
}

fn wrap_base64(data: &str) -> String {
    if data.is_empty() {
        return "\r\n".to_string();
    }

    let mut wrapped = String::new();
    for chunk in data.as_bytes().chunks(76) {
        wrapped.push_str(std::str::from_utf8(chunk).expect("base64 is utf-8"));
        wrapped.push_str("\r\n");
    }
    wrapped
}

fn push_address_header(
    message: &mut String,
    name: &str,
    values: &[String],
) -> Result<(), MailError> {
    if values.is_empty() {
        return Ok(());
    }
    push_single_header(message, name, &values.join(", "))
}

fn push_single_header(message: &mut String, name: &str, value: &str) -> Result<(), MailError> {
    reject_header_newlines(name, value)?;
    message.push_str(name);
    message.push_str(": ");
    message.push_str(value);
    message.push_str("\r\n");
    Ok(())
}

fn reject_header_newlines(name: &str, value: &str) -> Result<(), MailError> {
    if value.contains('\n') || value.contains('\r') {
        return Err(MailError::InvalidInput(format!(
            "{name} header cannot contain newlines"
        )));
    }
    Ok(())
}

fn normalize_body_newlines(body: &str) -> String {
    let mut normalized = body.replace("\r\n", "\n").replace('\r', "\n");
    normalized = normalized.replace('\n', "\r\n");
    if !normalized.ends_with("\r\n") {
        normalized.push_str("\r\n");
    }
    normalized
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

struct ResolvedAttachmentDownload {
    attachment_id: String,
    path: PathBuf,
}

async fn resolve_attachment_download<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DownloadAttachmentOptions,
) -> Result<ResolvedAttachmentDownload, MailError> {
    match (&options.attachment_id, &options.output) {
        (Some(attachment_id), Some(output)) => Ok(ResolvedAttachmentDownload {
            attachment_id: attachment_id.clone(),
            path: output.clone(),
        }),
        _ => {
            let metadata = fetch_attachment_filename_metadata(client, options).await?;
            let attachments = attachment_filenames(&metadata.payload);
            let attachment_id =
                resolve_attachment_id(options.attachment_id.as_deref(), &attachments)?;
            let path = match &options.output {
                Some(output) => output.clone(),
                None => {
                    let filename = filename_for_attachment(&attachments, &attachment_id)
                        .or_else(|| filename_from_single_attachment(&attachments))
                        .ok_or(MailError::MissingAttachmentFilename)?;
                    std::env::current_dir()
                        .map_err(MailError::Io)?
                        .join(filename)
                }
            };

            Ok(ResolvedAttachmentDownload {
                attachment_id,
                path,
            })
        }
    }
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

fn resolve_attachment_id(
    attachment_id: Option<&str>,
    attachments: &[AttachmentFilename],
) -> Result<String, MailError> {
    if let Some(attachment_id) = attachment_id {
        return Ok(attachment_id.to_string());
    }

    let [attachment] = attachments else {
        return Err(MailError::InvalidInput(
            "attachment ID is required when the message does not have exactly one attachment"
                .into(),
        ));
    };

    Ok(attachment.id.clone())
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
