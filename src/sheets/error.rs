use thiserror::Error;

#[derive(Debug, Error)]
pub enum SheetsError {
    #[error("Google Sheets Spreadsheet was not found")]
    NotFound,

    #[error("Google Sheets permission denied")]
    PermissionDenied,

    #[error(
        "Editing Excel-format files (.xlsx) via the Sheets API is not supported by Google; convert this file to a native Google Sheet (Drive UI: File > Save as Google Sheets), or edit it locally and re-upload"
    )]
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
