//! Draft references: parsing a Draft ID or Gmail browser URL into something
//! the Gmail API can resolve, and building the Gmail browser link for a Draft.

use url::Url;

use super::{
    decode_gmail_restricted_thread_token, decode_gmail_restricted_token,
    encode_gmail_restricted_token,
};

/// Gmail browser compose tokens encode the Draft ID behind this prefix.
const COMPOSE_TOKEN_PREFIX: &str = "msg-a:";
/// Gmail browser compose tokens encode a server Thread ID behind this prefix.
const THREAD_TOKEN_PREFIX: &str = "f:";
const GMAIL_WEB_URL: &str = "https://mail.google.com/mail/";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DraftReference {
    DraftId(String),
    /// A hex Message ID or Thread ID from a legacy Gmail URL. The Draft is
    /// found by listing Drafts and matching either ID.
    MessageOrThreadId(String),
    /// A Thread from a Gmail conversation URL. The Draft is the Thread's
    /// Message labeled DRAFT.
    Thread(String),
}

/// Accepts a bare Draft ID, a Gmail `#drafts?compose=` URL, or a Gmail
/// conversation URL for a Thread that holds a reply Draft.
pub fn parse_draft_reference(input: &str) -> DraftReference {
    let trimmed = input.trim();
    match Url::parse(trimmed)
        .ok()
        .and_then(|url| parse_draft_reference_from_url(&url))
    {
        Some(reference) => reference,
        None => DraftReference::DraftId(trimmed.to_string()),
    }
}

fn parse_draft_reference_from_url(url: &Url) -> Option<DraftReference> {
    if url.host_str()? != "mail.google.com" {
        return None;
    }
    let fragment = url.fragment()?;
    let (path, query) = fragment.split_once('?').unwrap_or((fragment, ""));

    if let Some(token) = query
        .split('&')
        .find_map(|pair| pair.strip_prefix("compose="))
        .filter(|token| !token.is_empty())
    {
        return Some(compose_token_reference(token));
    }

    let token = path.rsplit('/').next()?.trim();
    if token.is_empty() || !path.contains('/') {
        return None;
    }
    Some(match decode_gmail_restricted_thread_token(token) {
        Some(thread_id) => DraftReference::Thread(thread_id),
        None => DraftReference::MessageOrThreadId(token.to_string()),
    })
}

/// Gmail compose tokens decode to `thread-a:<thread>+msg-a:<draft>`, with one
/// comma-separated entry per open compose window. The first window wins.
fn compose_token_reference(token: &str) -> DraftReference {
    let decoded = decode_gmail_restricted_token(token);
    let draft_id = decoded.as_deref().and_then(|decoded| {
        decoded
            .split(',')
            .next()?
            .split('+')
            .find_map(|component| component.strip_prefix(COMPOSE_TOKEN_PREFIX))
    });
    match draft_id {
        Some(draft_id) => DraftReference::DraftId(draft_id.to_string()),
        None => DraftReference::MessageOrThreadId(token.to_string()),
    }
}

/// The Gmail browser link that opens a Draft in the compose window. Gmail's
/// compose token names the Draft's Thread as `f:<decimal Thread ID>` and the
/// Draft as `msg-a:<Draft ID>`. Edits keep both IDs, so the link stays the
/// same. Returns `None` when the Thread ID is not the API's hex form.
pub fn draft_web_link(account_email: &str, thread_id: &str, draft_id: &str) -> Option<String> {
    let thread_number = u64::from_str_radix(thread_id, 16).ok()?;
    let token = encode_gmail_restricted_token(&format!(
        "{THREAD_TOKEN_PREFIX}{thread_number}+{COMPOSE_TOKEN_PREFIX}{draft_id}"
    ));
    Some(format!(
        "{GMAIL_WEB_URL}?authuser={account_email}#drafts?compose={token}"
    ))
}
