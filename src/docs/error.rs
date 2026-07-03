use thiserror::Error;

#[derive(Debug, Error)]
pub enum DocsError {
    #[error("Google Docs Document was not found")]
    NotFound,

    #[error("Google Docs permission denied")]
    PermissionDenied,

    #[error(
        "Google Docs cannot write to Office documents; convert to a native Google Docs Document and retry"
    )]
    UnsupportedOfficeFile,

    #[error("Google Docs API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("invalid Google Docs API response: {0}")]
    InvalidResponse(String),

    #[error("invalid Google Docs API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),
}
