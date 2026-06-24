use thiserror::Error;

#[derive(Debug, Error)]
pub enum MailError {
    #[error("GoogleMail Message was not found")]
    NotFound,

    #[error("GoogleMail permission denied")]
    PermissionDenied,

    #[error("GoogleMail API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("invalid GoogleMail API response: {0}")]
    InvalidResponse(String),

    #[error("invalid GoogleMail API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),
}
