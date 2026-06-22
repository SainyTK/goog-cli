use thiserror::Error;

#[derive(Debug, Error)]
pub enum DriveError {
    #[error("file not found: {id}")]
    NotFound { id: String },

    #[error("permission denied for file: {id}")]
    PermissionDenied { id: String },

    #[error("auth error: {0}")]
    Auth(#[from] goog_auth::error::AuthError),
}
