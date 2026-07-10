use thiserror::Error;

#[derive(Debug, Error)]
pub enum SlidesError {
    #[error("Google Slides presentation was not found")]
    NotFound,

    #[error("Google Slides permission denied")]
    PermissionDenied,

    #[error("Google Slides API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("invalid Google Slides API response: {0}")]
    InvalidResponse(String),

    #[error("invalid Google Slides API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("Google Slides network error: {0}")]
    Network(String),

    #[error("Google Slides artifact error: {0}")]
    Artifact(String),

    #[error("Google Slides artifact I/O error: {0}")]
    ArtifactIo(#[source] std::io::Error),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),
}
