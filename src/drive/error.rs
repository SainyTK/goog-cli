use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriveError {
    #[error("Google Drive resource was not found")]
    NotFound,

    #[error("Google Drive permission denied")]
    PermissionDenied,

    #[error("Google Drive API error ({status}): {body}")]
    Api { status: reqwest::StatusCode, body: String },

    #[error("invalid Google Drive API response: {0}")]
    InvalidResponse(String),

    #[error("invalid Google Drive API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Google Drive network error: {0}")]
    Network(reqwest::Error),

    #[error("Google Drive file I/O error: {0}")]
    Io(std::io::Error),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),
}
