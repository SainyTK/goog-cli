pub mod error;

pub use error::SheetsError;

use reqwest::{Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const SHEETS_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets.readonly";
pub const SHEETS_READONLY_SCOPES: &[&str] = &[SHEETS_READONLY_SCOPE];
const SHEETS_SPREADSHEETS_URL: &str = "https://sheets.googleapis.com/v4/spreadsheets";

pub type Spreadsheet = Value;

#[derive(Debug, Clone)]
pub struct GetSpreadsheetOptions {
    pub spreadsheet_id: String,
    pub fields: Option<String>,
    pub include_grid_data: bool,
    pub ranges: Vec<String>,
    spreadsheets_url: String,
}

impl GetSpreadsheetOptions {
    pub fn new(spreadsheet_id: impl Into<String>) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            fields: None,
            include_grid_data: false,
            ranges: Vec::new(),
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub fn with_fields(mut self, fields: impl Into<String>) -> Self {
        self.fields = Some(fields.into());
        self
    }

    pub fn with_include_grid_data(mut self, include_grid_data: bool) -> Self {
        self.include_grid_data = include_grid_data;
        self
    }

    pub fn with_ranges(mut self, ranges: Vec<String>) -> Self {
        self.ranges = ranges;
        self
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        let mut url = spreadsheet_url(&self.spreadsheets_url, &self.spreadsheet_id)?;
        {
            let mut query = url.query_pairs_mut();
            if let Some(fields) = &self.fields {
                query.append_pair("fields", fields);
            }
            if self.include_grid_data {
                query.append_pair("includeGridData", "true");
            }
            for range in &self.ranges {
                query.append_pair("ranges", range);
            }
        }
        Ok(url)
    }
}

fn spreadsheet_url(spreadsheets_url: &str, spreadsheet_id: &str) -> Result<Url, SheetsError> {
    let mut url = Url::parse(spreadsheets_url)?;
    url.path_segments_mut()
        .map_err(|_| SheetsError::InvalidResponse("Google Sheets API URL cannot be a base".into()))?
        .push(spreadsheet_id);
    Ok(url)
}

pub async fn get_spreadsheet<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetSpreadsheetOptions,
) -> Result<Spreadsheet, SheetsError> {
    let response = client
        .send_with_scopes(client.get(options.request_url()?), SHEETS_READONLY_SCOPES)
        .await
        .map_err(SheetsError::Auth)?;

    parse_json_response(response).await
}

async fn parse_json_response(response: Response) -> Result<Value, SheetsError> {
    let response = ensure_success_response(response).await?;
    response
        .json::<Value>()
        .await
        .map_err(|e| SheetsError::InvalidResponse(e.to_string()))
}

async fn ensure_success_response(response: Response) -> Result<Response, SheetsError> {
    let status = response.status();
    if status.is_success() {
        return Ok(response);
    }

    match status {
        StatusCode::NOT_FOUND => Err(SheetsError::NotFound),
        StatusCode::FORBIDDEN => Err(SheetsError::PermissionDenied),
        status => {
            let body = response.text().await.unwrap_or_default();
            if is_office_file_precondition_error(status, &body) {
                return Err(SheetsError::UnsupportedOfficeFile);
            }
            Err(SheetsError::Api { status, body })
        }
    }
}

fn is_office_file_precondition_error(status: StatusCode, body: &str) -> bool {
    if status != StatusCode::BAD_REQUEST {
        return false;
    }

    let Ok(value) = serde_json::from_str::<Value>(body) else {
        return false;
    };
    let error = &value["error"];
    error["status"].as_str() == Some("FAILED_PRECONDITION")
        && error["message"]
            .as_str()
            .map_or(false, |message| message.contains("must not be an Office file"))
}
