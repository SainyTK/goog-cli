use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriveError {
    #[error("Google Drive resource was not found")]
    NotFound,

    #[error("Google Drive permission denied")]
    PermissionDenied,

    #[error("Google Drive API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("Google Drive file is a native Google file ({0}); only uploaded files such as Office documents or PDFs can take new content")]
    NativeGoogleFile(String),

    #[error("Google Drive file changed since the expected version: {0}")]
    VersionMismatch(String),

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
