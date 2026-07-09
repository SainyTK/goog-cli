use thiserror::Error;

#[derive(Debug, Error)]
pub enum MailError {
    #[error("Gmail message was not found")]
    NotFound,

    #[error("Gmail permission denied")]
    PermissionDenied,

    #[error("Gmail API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("invalid Gmail API response: {0}")]
    InvalidResponse(String),

    #[error("invalid Gmail input: {0}")]
    InvalidInput(String),

    #[error("Gmail attachment filename was not found; pass --output")]
    MissingAttachmentFilename,

    #[error("invalid Gmail API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),
}
