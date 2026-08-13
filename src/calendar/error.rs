use thiserror::Error;

#[derive(Debug, Error)]
pub enum CalendarError {
    #[error("Google Calendar resource was not found")]
    NotFound,

    #[error("Google Calendar permission denied")]
    PermissionDenied,

    #[error("Google Calendar API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("invalid Google Calendar API response: {0}")]
    InvalidResponse(String),

    #[error("invalid Google Calendar API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),
}
