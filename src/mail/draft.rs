//! Draft content: the editable message model, its RFC 2822 rendering, and
//! reading an existing Gmail Draft payload back into the model so edits can
//! change only the fields they name.

use base64::Engine;
use serde::{Deserialize, Serialize};

use super::MailError;

const MIXED_BOUNDARY: &str = "goog-cli-draft-boundary";
const RELATED_BOUNDARY: &str = "goog-cli-draft-related";
const ALTERNATIVE_BOUNDARY: &str = "goog-cli-draft-alternative";
const UNTITLED_ATTACHMENT: &str = "untitled";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DraftMessage {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub subject: String,
    pub body: DraftBody,
    pub attachments: Vec<DraftAttachment>,
    pub thread: Option<DraftThread>,
}

/// The Gmail Thread a Draft belongs to. Reply Drafts also carry the
/// In-Reply-To and References headers Gmail needs to keep them in the Thread.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DraftThread {
    pub thread_id: String,
    pub in_reply_to: Option<String>,
    pub references: Option<String>,
}

/// A Draft body holds a plain text part, an HTML part, or both as
/// multipart/alternative, which is how Gmail stores Drafts written in its editor.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DraftBody {
    pub text: Option<String>,
    pub html: Option<String>,
}

impl DraftBody {
    pub fn new(content: impl Into<String>, format: DraftBodyFormat) -> Self {
        match format {
            DraftBodyFormat::PlainText => Self {
                text: Some(content.into()),
                html: None,
            },
            DraftBodyFormat::Html => Self {
                text: None,
                html: Some(content.into()),
            },
        }
    }
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

/// A file carried by a Draft. Attachments with a Content-ID are inline
/// images referenced from the HTML body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DraftAttachment {
    pub filename: String,
    pub content_type: String,
    pub data: Vec<u8>,
    pub content_id: Option<String>,
}

/// Changes to apply to an existing Draft. Empty recipient lists and `None`
/// fields leave the existing value unchanged.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DraftEdit {
    pub to: Vec<String>,
    pub cc: Vec<String>,
    pub bcc: Vec<String>,
    pub add_to: Vec<String>,
    pub add_cc: Vec<String>,
    pub add_bcc: Vec<String>,
    pub subject: Option<String>,
    pub body: Option<DraftBody>,
    pub attachments: Vec<DraftAttachment>,
    pub remove_attachments: Vec<String>,
}

impl DraftEdit {
    pub fn apply(self, mut message: DraftMessage) -> Result<DraftMessage, MailError> {
        replace_unless_empty(&mut message.to, self.to);
        replace_unless_empty(&mut message.cc, self.cc);
        replace_unless_empty(&mut message.bcc, self.bcc);
        message.to.extend(self.add_to);
        message.cc.extend(self.add_cc);
        message.bcc.extend(self.add_bcc);
        if let Some(subject) = self.subject {
            message.subject = subject;
        }
        if let Some(body) = self.body {
            message.body = body;
        }
        for filename in &self.remove_attachments {
            let before = message.attachments.len();
            message
                .attachments
                .retain(|attachment| attachment.filename != *filename);
            if message.attachments.len() == before {
                return Err(MailError::InvalidInput(format!(
                    "draft has no attachment named {filename}"
                )));
            }
        }
        message.attachments.extend(self.attachments);
        Ok(message)
    }
}

fn replace_unless_empty(current: &mut Vec<String>, replacement: Vec<String>) {
    if !replacement.is_empty() {
        *current = replacement;
    }
}

/// Headers of the Message a reply Draft answers.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RepliedMessage {
    pub thread_id: String,
    pub message_id_header: Option<String>,
    pub references: Option<String>,
    pub subject: String,
    pub from: Option<String>,
    pub reply_to: Option<String>,
}

impl RepliedMessage {
    /// Places a new Draft in this Message's Thread, defaulting the recipient
    /// and subject when the Draft does not set them.
    pub fn address_reply(&self, mut message: DraftMessage) -> DraftMessage {
        if message.to.is_empty() {
            message
                .to
                .extend(self.reply_to.clone().or_else(|| self.from.clone()));
        }
        if message.subject.is_empty() {
            message.subject = self.reply_subject();
        }
        message.thread = Some(self.draft_thread());
        message
    }

    fn reply_subject(&self) -> String {
        let subject = self.subject.trim();
        if subject.len() >= 3 && subject[..3].eq_ignore_ascii_case("re:") {
            subject.to_string()
        } else {
            format!("Re: {subject}")
        }
    }

    fn draft_thread(&self) -> DraftThread {
        let references = match (&self.references, &self.message_id_header) {
            (Some(references), Some(message_id)) => Some(format!("{references} {message_id}")),
            (None, Some(message_id)) => Some(message_id.clone()),
            (references, None) => references.clone(),
        };
        DraftThread {
            thread_id: self.thread_id.clone(),
            in_reply_to: self.message_id_header.clone(),
            references,
        }
    }
}

/// The parts of a Gmail `format=full` Message payload a Draft edit reads.
/// Attachment data must be hydrated into `body.data` before conversion.
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayloadPart {
    #[serde(default, rename = "mimeType")]
    pub mime_type: String,
    #[serde(default)]
    pub filename: String,
    #[serde(default)]
    pub headers: Vec<PayloadHeader>,
    #[serde(default)]
    pub body: PayloadBody,
    #[serde(default)]
    pub parts: Vec<PayloadPart>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayloadHeader {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct PayloadBody {
    #[serde(
        default,
        rename = "attachmentId",
        skip_serializing_if = "Option::is_none"
    )]
    pub attachment_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub data: Option<String>,
}

impl PayloadPart {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|header| header.name.eq_ignore_ascii_case(name))
            .map(|header| header.value.as_str())
            .filter(|value| !value.trim().is_empty())
    }

    /// Every part whose data Gmail stores separately behind an attachment ID.
    pub fn parts_missing_data(&mut self) -> Vec<&mut PayloadPart> {
        let mut missing = Vec::new();
        collect_parts_missing_data(self, &mut missing);
        missing
    }

    fn disposition(&self) -> Option<String> {
        self.header("Content-Disposition").map(|value| {
            value
                .split(';')
                .next()
                .unwrap_or(value)
                .trim()
                .to_ascii_lowercase()
        })
    }

    fn data(&self) -> Result<Vec<u8>, MailError> {
        match &self.body.data {
            Some(data) => super::decode_base64url(data),
            None => Ok(Vec::new()),
        }
    }
}

fn collect_parts_missing_data<'a>(
    part: &'a mut PayloadPart,
    missing: &mut Vec<&'a mut PayloadPart>,
) {
    if part.parts.is_empty() {
        if part.body.data.is_none() && part.body.attachment_id.is_some() {
            missing.push(part);
        }
        return;
    }
    for child in &mut part.parts {
        collect_parts_missing_data(child, missing);
    }
}

/// Converts an existing Draft payload into the editable model, keeping
/// recipients, subject, both body formats, attachments, inline images and the
/// Thread so an edit can change only what it names.
pub fn draft_message_from_payload(
    payload: &PayloadPart,
    thread_id: Option<String>,
) -> Result<DraftMessage, MailError> {
    let mut message = DraftMessage {
        to: address_header(payload, "To"),
        cc: address_header(payload, "Cc"),
        bcc: address_header(payload, "Bcc"),
        subject: payload.header("Subject").unwrap_or_default().to_string(),
        thread: thread_id.map(|thread_id| DraftThread {
            thread_id,
            in_reply_to: payload.header("In-Reply-To").map(str::to_string),
            references: payload.header("References").map(str::to_string),
        }),
        ..DraftMessage::default()
    };
    collect_content(payload, &mut message)?;
    Ok(message)
}

fn address_header(payload: &PayloadPart, name: &str) -> Vec<String> {
    payload
        .header(name)
        .map(|value| vec![value.trim().to_string()])
        .unwrap_or_default()
}

fn collect_content(part: &PayloadPart, message: &mut DraftMessage) -> Result<(), MailError> {
    if part.mime_type.starts_with("multipart/") {
        for child in &part.parts {
            collect_content(child, message)?;
        }
        return Ok(());
    }

    let disposition = part.disposition();
    let is_body_text = part.filename.is_empty() && disposition.as_deref() != Some("attachment");
    let body_slot = match part.mime_type.as_str() {
        "text/plain" if is_body_text => Some(&mut message.body.text),
        "text/html" if is_body_text => Some(&mut message.body.html),
        _ => None,
    };
    if let Some(slot) = body_slot.filter(|slot| slot.is_none()) {
        *slot = Some(String::from_utf8_lossy(&part.data()?).into_owned());
        return Ok(());
    }

    let content_id = part
        .header("Content-ID")
        .filter(|_| disposition.as_deref() == Some("inline"))
        .map(str::to_string);
    message.attachments.push(DraftAttachment {
        filename: attachment_filename(part),
        content_type: if part.mime_type.is_empty() {
            "application/octet-stream".to_string()
        } else {
            part.mime_type.clone()
        },
        data: part.data()?,
        content_id,
    });
    Ok(())
}

fn attachment_filename(part: &PayloadPart) -> String {
    if !part.filename.is_empty() {
        return part.filename.clone();
    }
    part.header("Content-Type")
        .and_then(|value| super::header_parameter(value, "name"))
        .unwrap_or_else(|| UNTITLED_ATTACHMENT.to_string())
}

/// Renders a Draft as an RFC 2822 message. Recipients may be empty because
/// Gmail keeps unaddressed Drafts; new Drafts require To at the command line.
pub fn build_draft_raw_message(message: &DraftMessage) -> Result<String, MailError> {
    let mut raw = draft_headers(message)?;
    raw.push_str("MIME-Version: 1.0\r\n");
    write_entity(&mut raw, &message_entity(message))?;
    Ok(raw)
}

enum Entity<'a> {
    Text {
        format: DraftBodyFormat,
        content: &'a str,
    },
    Attachment(&'a DraftAttachment),
    Multipart {
        subtype: &'static str,
        boundary: &'static str,
        parts: Vec<Entity<'a>>,
    },
}

/// Nests the body as multipart/mixed( multipart/related( body, inline images ), attachments ),
/// omitting each wrapper the Draft does not need.
fn message_entity(message: &DraftMessage) -> Entity<'_> {
    let (inline, attached): (Vec<_>, Vec<_>) = message
        .attachments
        .iter()
        .partition(|attachment| attachment.content_id.is_some());
    let body = body_entity(&message.body);
    let related = wrap_multipart("related", RELATED_BOUNDARY, body, inline);
    wrap_multipart("mixed", MIXED_BOUNDARY, related, attached)
}

fn wrap_multipart<'a>(
    subtype: &'static str,
    boundary: &'static str,
    first: Entity<'a>,
    attachments: Vec<&'a DraftAttachment>,
) -> Entity<'a> {
    if attachments.is_empty() {
        return first;
    }
    let mut parts = vec![first];
    parts.extend(attachments.into_iter().map(Entity::Attachment));
    Entity::Multipart {
        subtype,
        boundary,
        parts,
    }
}

fn body_entity(body: &DraftBody) -> Entity<'_> {
    match (&body.text, &body.html) {
        (Some(text), Some(html)) => Entity::Multipart {
            subtype: "alternative",
            boundary: ALTERNATIVE_BOUNDARY,
            parts: vec![
                Entity::Text {
                    format: DraftBodyFormat::PlainText,
                    content: text,
                },
                Entity::Text {
                    format: DraftBodyFormat::Html,
                    content: html,
                },
            ],
        },
        (None, Some(html)) => Entity::Text {
            format: DraftBodyFormat::Html,
            content: html,
        },
        (text, None) => Entity::Text {
            format: DraftBodyFormat::PlainText,
            content: text.as_deref().unwrap_or_default(),
        },
    }
}

fn write_entity(raw: &mut String, entity: &Entity<'_>) -> Result<(), MailError> {
    match entity {
        Entity::Text { format, content } => {
            raw.push_str(&format!(
                "Content-Type: {}; charset=UTF-8\r\n",
                format.mime_type()
            ));
            raw.push_str("Content-Transfer-Encoding: 8bit\r\n");
            raw.push_str("\r\n");
            raw.push_str(&normalize_body_newlines(content));
        }
        Entity::Attachment(attachment) => raw.push_str(&draft_attachment_part(attachment)?),
        Entity::Multipart {
            subtype,
            boundary,
            parts,
        } => {
            raw.push_str(&format!(
                "Content-Type: multipart/{subtype}; boundary=\"{boundary}\"\r\n"
            ));
            raw.push_str("\r\n");
            for part in parts {
                raw.push_str(&format!("--{boundary}\r\n"));
                write_entity(raw, part)?;
            }
            raw.push_str(&format!("--{boundary}--\r\n"));
        }
    }
    Ok(())
}

fn draft_headers(message: &DraftMessage) -> Result<String, MailError> {
    let mut headers = String::new();
    push_address_header(&mut headers, "To", &message.to)?;
    push_address_header(&mut headers, "Cc", &message.cc)?;
    push_address_header(&mut headers, "Bcc", &message.bcc)?;
    push_single_header(&mut headers, "Subject", &message.subject)?;
    if let Some(thread) = &message.thread {
        if let Some(in_reply_to) = &thread.in_reply_to {
            push_single_header(&mut headers, "In-Reply-To", in_reply_to)?;
        }
        if let Some(references) = &thread.references {
            push_single_header(&mut headers, "References", references)?;
        }
    }
    Ok(headers)
}

fn draft_attachment_part(attachment: &DraftAttachment) -> Result<String, MailError> {
    reject_header_newlines("Attachment filename", &attachment.filename)?;
    reject_header_newlines("Attachment content type", &attachment.content_type)?;
    let filename = quote_header_parameter(&attachment.filename);
    let disposition = if attachment.content_id.is_some() {
        "inline"
    } else {
        "attachment"
    };
    let mut part = String::new();
    part.push_str(&format!(
        "Content-Type: {}; name=\"{}\"\r\n",
        attachment.content_type, filename
    ));
    part.push_str(&format!(
        "Content-Disposition: {disposition}; filename=\"{filename}\"\r\n"
    ));
    if let Some(content_id) = &attachment.content_id {
        push_single_header(&mut part, "Content-ID", content_id)?;
    }
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
