use thiserror::Error;

#[derive(Debug, Error)]
pub enum SheetsError {
    #[error("Google Sheets Spreadsheet was not found")]
    NotFound,

    #[error("Google Sheets permission denied")]
    PermissionDenied,

    #[error("Google Sheets cannot read Office spreadsheets; convert to a native Google Sheets Spreadsheet and retry")]
    UnsupportedOfficeFile,

    #[error("Google Sheets API error ({status}): {body}")]
    Api {
        status: reqwest::StatusCode,
        body: String,
    },

    #[error("invalid Google Sheets API response: {0}")]
    InvalidResponse(String),

    #[error("invalid Google Sheets API URL: {0}")]
    InvalidUrl(#[from] url::ParseError),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),
}
