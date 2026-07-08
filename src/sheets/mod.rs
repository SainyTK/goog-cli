pub mod error;

pub use error::SheetsError;

use std::future::Future;

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::Deserialize;
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::drive::DRIVE_SCOPES;

pub const SHEETS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";
pub const SHEETS_SCOPES: &[&str] = &[SHEETS_SCOPE];
const SHEETS_SPREADSHEETS_URL: &str = "https://sheets.googleapis.com/v4/spreadsheets";
const OFFICE_FILE_PRECONDITION_STATUS: &str = "FAILED_PRECONDITION";
const OFFICE_FILE_PRECONDITION_MESSAGE: &str = "must not be an Office file";
const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const GOOGLE_SHEETS_MIME_TYPE: &str = "application/vnd.google-apps.spreadsheet";
const TEMPORARY_CONVERSION_NAME: &str = "goog temporary Sheets conversion";

pub type Spreadsheet = Value;
pub type CreateSpreadsheetResponse = Value;
pub type ValueRange = Value;
pub type BatchGetValuesResponse = Value;
pub type UpdateValuesResponse = Value;
pub type BatchUpdateValuesResponse = Value;
pub type AppendValuesResponse = Value;
pub type ClearValuesResponse = Value;
pub type BatchClearValuesResponse = Value;
pub type BatchUpdateSpreadsheetResponse = Value;

pub enum SheetsOperation<'a> {
    GetSpreadsheet(&'a GetSpreadsheetOptions),
    GetValues(&'a GetValuesOptions),
    BatchGetValues(&'a BatchGetValuesOptions),
    UpdateValues(&'a UpdateValuesOptions),
    BatchUpdateValues(&'a BatchUpdateValuesOptions),
    AppendValues(&'a AppendValuesOptions),
    ClearValues(&'a ClearValuesOptions),
    BatchClearValues(&'a BatchClearValuesOptions),
    BatchUpdateSpreadsheet(&'a BatchUpdateSpreadsheetOptions),
}

#[derive(Debug, Clone)]
pub struct CreateSpreadsheetOptions {
    pub title: String,
    spreadsheets_url: String,
}

impl CreateSpreadsheetOptions {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        Ok(Url::parse(&self.spreadsheets_url)?)
    }
}

impl SheetsOperation<'_> {
    pub fn spreadsheet_id(&self) -> &str {
        match self {
            Self::GetSpreadsheet(options) => &options.spreadsheet_id,
            Self::GetValues(options) => &options.spreadsheet_id,
            Self::BatchGetValues(options) => &options.spreadsheet_id,
            Self::UpdateValues(options) => &options.spreadsheet_id,
            Self::BatchUpdateValues(options) => &options.spreadsheet_id,
            Self::AppendValues(options) => &options.spreadsheet_id,
            Self::ClearValues(options) => &options.spreadsheet_id,
            Self::BatchClearValues(options) => &options.spreadsheet_id,
            Self::BatchUpdateSpreadsheet(options) => &options.spreadsheet_id,
        }
    }

    pub async fn execute<S: AccountStore>(
        &self,
        client: &AuthClient<'_, S>,
    ) -> Result<Value, SheetsError> {
        match self {
            Self::GetSpreadsheet(options) => {
                send_json_request_with_office_file_fallback(
                    client,
                    client.get(options.request_url()?),
                    SHEETS_SCOPES,
                    || get_spreadsheet_via_temporary_conversion(client, options),
                )
                .await
            }
            Self::GetValues(options) => {
                send_json_request_with_office_file_fallback(
                    client,
                    client.get(options.request_url()?),
                    SHEETS_SCOPES,
                    || get_values_via_temporary_conversion(client, options),
                )
                .await
            }
            Self::BatchGetValues(options) => {
                send_json_request_with_office_file_fallback(
                    client,
                    client.get(options.request_url()?),
                    SHEETS_SCOPES,
                    || batch_get_values_via_temporary_conversion(client, options),
                )
                .await
            }
            Self::UpdateValues(options) => {
                send_json_request(
                    client,
                    client
                        .put(options.request_url()?)
                        .json(&options.request_body),
                    SHEETS_SCOPES,
                )
                .await
            }
            Self::BatchUpdateValues(options) => {
                send_json_request(
                    client,
                    client
                        .post(options.request_url()?)
                        .json(&options.request_body),
                    SHEETS_SCOPES,
                )
                .await
            }
            Self::AppendValues(options) => {
                send_json_request(
                    client,
                    client
                        .post(options.request_url()?)
                        .json(&options.request_body),
                    SHEETS_SCOPES,
                )
                .await
            }
            Self::ClearValues(options) => {
                send_json_request(
                    client,
                    client
                        .post(options.request_url()?)
                        .json(&serde_json::json!({})),
                    SHEETS_SCOPES,
                )
                .await
            }
            Self::BatchClearValues(options) => {
                send_json_request(
                    client,
                    client
                        .post(options.request_url()?)
                        .json(&serde_json::json!({ "ranges": &options.ranges })),
                    SHEETS_SCOPES,
                )
                .await
            }
            Self::BatchUpdateSpreadsheet(options) => {
                send_json_request(
                    client,
                    client
                        .post(options.request_url()?)
                        .json(&options.request_body),
                    SHEETS_SCOPES,
                )
                .await
            }
        }
    }
}

pub async fn create_spreadsheet<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &CreateSpreadsheetOptions,
) -> Result<CreateSpreadsheetResponse, SheetsError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .json(&serde_json::json!({ "properties": { "title": options.title } })),
        SHEETS_SCOPES,
    )
    .await
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueRenderOption {
    FormattedValue,
    UnformattedValue,
    Formula,
}

impl ValueRenderOption {
    fn as_google_value(self) -> &'static str {
        match self {
            Self::FormattedValue => "FORMATTED_VALUE",
            Self::UnformattedValue => "UNFORMATTED_VALUE",
            Self::Formula => "FORMULA",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueInputOption {
    Raw,
    UserEntered,
}

impl ValueInputOption {
    fn as_google_value(self) -> &'static str {
        match self {
            Self::Raw => "RAW",
            Self::UserEntered => "USER_ENTERED",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsertDataOption {
    InsertRows,
    Overwrite,
}

impl InsertDataOption {
    fn as_google_value(self) -> &'static str {
        match self {
            Self::InsertRows => "INSERT_ROWS",
            Self::Overwrite => "OVERWRITE",
        }
    }
}

#[derive(Debug, Clone)]
pub struct GetSpreadsheetOptions {
    pub spreadsheet_id: String,
    pub fields: Option<String>,
    pub include_grid_data: bool,
    pub ranges: Vec<String>,
    spreadsheets_url: String,
    drive_files_url: String,
}

impl GetSpreadsheetOptions {
    pub fn new(spreadsheet_id: impl Into<String>) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            fields: None,
            include_grid_data: false,
            ranges: Vec::new(),
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
            drive_files_url: DRIVE_FILES_URL.to_string(),
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

    pub(super) fn with_drive_files_url(mut self, drive_files_url: impl Into<String>) -> Self {
        self.drive_files_url = drive_files_url.into();
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

#[derive(Debug, Clone)]
pub struct GetValuesOptions {
    pub spreadsheet_id: String,
    pub range: String,
    pub value_render_option: ValueRenderOption,
    spreadsheets_url: String,
    drive_files_url: String,
}

impl GetValuesOptions {
    pub fn new(spreadsheet_id: impl Into<String>, range: impl Into<String>) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            range: range.into(),
            value_render_option: ValueRenderOption::FormattedValue,
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
            drive_files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn with_value_render_option(mut self, value_render_option: ValueRenderOption) -> Self {
        self.value_render_option = value_render_option;
        self
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    pub(super) fn with_drive_files_url(mut self, drive_files_url: impl Into<String>) -> Self {
        self.drive_files_url = drive_files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        let mut url =
            spreadsheet_values_url(&self.spreadsheets_url, &self.spreadsheet_id, &[&self.range])?;
        url.query_pairs_mut().append_pair(
            "valueRenderOption",
            self.value_render_option.as_google_value(),
        );
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct BatchGetValuesOptions {
    pub spreadsheet_id: String,
    pub ranges: Vec<String>,
    pub value_render_option: ValueRenderOption,
    spreadsheets_url: String,
    drive_files_url: String,
}

impl BatchGetValuesOptions {
    pub fn new(spreadsheet_id: impl Into<String>, ranges: Vec<String>) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            ranges,
            value_render_option: ValueRenderOption::FormattedValue,
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
            drive_files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn with_value_render_option(mut self, value_render_option: ValueRenderOption) -> Self {
        self.value_render_option = value_render_option;
        self
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    pub(super) fn with_drive_files_url(mut self, drive_files_url: impl Into<String>) -> Self {
        self.drive_files_url = drive_files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        let mut url =
            spreadsheet_values_url(&self.spreadsheets_url, &self.spreadsheet_id, &[":batchGet"])?;
        {
            let mut query = url.query_pairs_mut();
            for range in &self.ranges {
                query.append_pair("ranges", range);
            }
            query.append_pair(
                "valueRenderOption",
                self.value_render_option.as_google_value(),
            );
        }
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct UpdateValuesOptions {
    pub spreadsheet_id: String,
    pub range: String,
    pub request_body: Value,
    pub value_input_option: ValueInputOption,
    spreadsheets_url: String,
}

impl UpdateValuesOptions {
    pub fn new(
        spreadsheet_id: impl Into<String>,
        range: impl Into<String>,
        request_body: Value,
    ) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            range: range.into(),
            request_body,
            value_input_option: ValueInputOption::UserEntered,
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub fn with_value_input_option(mut self, value_input_option: ValueInputOption) -> Self {
        self.value_input_option = value_input_option;
        self
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        let mut url =
            spreadsheet_values_url(&self.spreadsheets_url, &self.spreadsheet_id, &[&self.range])?;
        url.query_pairs_mut().append_pair(
            "valueInputOption",
            self.value_input_option.as_google_value(),
        );
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct BatchUpdateValuesOptions {
    pub spreadsheet_id: String,
    pub request_body: Value,
    spreadsheets_url: String,
}

impl BatchUpdateValuesOptions {
    pub fn new(spreadsheet_id: impl Into<String>, request_body: Value) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            request_body,
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        spreadsheet_values_url(
            &self.spreadsheets_url,
            &self.spreadsheet_id,
            &[":batchUpdate"],
        )
    }
}

#[derive(Debug, Clone)]
pub struct AppendValuesOptions {
    pub spreadsheet_id: String,
    pub range: String,
    pub request_body: Value,
    pub value_input_option: ValueInputOption,
    pub insert_data_option: InsertDataOption,
    spreadsheets_url: String,
}

impl AppendValuesOptions {
    pub fn new(
        spreadsheet_id: impl Into<String>,
        range: impl Into<String>,
        request_body: Value,
    ) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            range: range.into(),
            request_body,
            value_input_option: ValueInputOption::UserEntered,
            insert_data_option: InsertDataOption::InsertRows,
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub fn with_value_input_option(mut self, value_input_option: ValueInputOption) -> Self {
        self.value_input_option = value_input_option;
        self
    }

    pub fn with_insert_data_option(mut self, insert_data_option: InsertDataOption) -> Self {
        self.insert_data_option = insert_data_option;
        self
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        let append_range = format!("{}:append", self.range);
        let mut url = spreadsheet_values_url(
            &self.spreadsheets_url,
            &self.spreadsheet_id,
            &[&append_range],
        )?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair(
                "valueInputOption",
                self.value_input_option.as_google_value(),
            );
            query.append_pair(
                "insertDataOption",
                self.insert_data_option.as_google_value(),
            );
        }
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct ClearValuesOptions {
    pub spreadsheet_id: String,
    pub range: String,
    spreadsheets_url: String,
}

impl ClearValuesOptions {
    pub fn new(spreadsheet_id: impl Into<String>, range: impl Into<String>) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            range: range.into(),
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        let clear_range = format!("{}:clear", self.range);
        spreadsheet_values_url(
            &self.spreadsheets_url,
            &self.spreadsheet_id,
            &[&clear_range],
        )
    }
}

#[derive(Debug, Clone)]
pub struct BatchClearValuesOptions {
    pub spreadsheet_id: String,
    pub ranges: Vec<String>,
    spreadsheets_url: String,
}

impl BatchClearValuesOptions {
    pub fn new(spreadsheet_id: impl Into<String>, ranges: Vec<String>) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            ranges,
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        spreadsheet_values_url(
            &self.spreadsheets_url,
            &self.spreadsheet_id,
            &[":batchClear"],
        )
    }
}

#[derive(Debug, Clone)]
pub struct BatchUpdateSpreadsheetOptions {
    pub spreadsheet_id: String,
    pub request_body: Value,
    spreadsheets_url: String,
}

impl BatchUpdateSpreadsheetOptions {
    pub fn new(spreadsheet_id: impl Into<String>, request_body: Value) -> Self {
        Self {
            spreadsheet_id: spreadsheet_id.into(),
            request_body,
            spreadsheets_url: SHEETS_SPREADSHEETS_URL.to_string(),
        }
    }

    pub(super) fn with_spreadsheets_url(mut self, spreadsheets_url: impl Into<String>) -> Self {
        self.spreadsheets_url = spreadsheets_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, SheetsError> {
        spreadsheet_url(
            &self.spreadsheets_url,
            &format!("{}:batchUpdate", self.spreadsheet_id),
        )
    }
}

fn spreadsheet_url(spreadsheets_url: &str, spreadsheet_id: &str) -> Result<Url, SheetsError> {
    let mut url = Url::parse(spreadsheets_url)?;
    url.path_segments_mut()
        .map_err(|_| SheetsError::InvalidResponse("Google Sheets API URL cannot be a base".into()))?
        .push(spreadsheet_id);
    Ok(url)
}

fn spreadsheet_values_url(
    spreadsheets_url: &str,
    spreadsheet_id: &str,
    values_path: &[&str],
) -> Result<Url, SheetsError> {
    let mut url = spreadsheet_url(spreadsheets_url, spreadsheet_id)?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            SheetsError::InvalidResponse("Google Sheets API URL cannot be a base".into())
        })?;
        segments.push("values");
        for segment in values_path {
            segments.push(segment);
        }
    }
    Ok(url)
}

fn drive_file_url(
    files_url: &str,
    file_id: &str,
    suffix: Option<&str>,
) -> Result<Url, SheetsError> {
    let mut url = Url::parse(files_url)?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            SheetsError::InvalidResponse("Google Drive API URL cannot be a base".into())
        })?;
        segments.push(file_id);
        if let Some(suffix) = suffix {
            segments.push(suffix);
        }
    }
    url.query_pairs_mut()
        .append_pair("supportsAllDrives", "true");
    Ok(url)
}

pub async fn get_spreadsheet<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetSpreadsheetOptions,
) -> Result<Spreadsheet, SheetsError> {
    SheetsOperation::GetSpreadsheet(options)
        .execute(client)
        .await
}

pub async fn get_values<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetValuesOptions,
) -> Result<ValueRange, SheetsError> {
    SheetsOperation::GetValues(options).execute(client).await
}

pub async fn batch_get_values<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchGetValuesOptions,
) -> Result<BatchGetValuesResponse, SheetsError> {
    SheetsOperation::BatchGetValues(options)
        .execute(client)
        .await
}

pub async fn update_values<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &UpdateValuesOptions,
) -> Result<UpdateValuesResponse, SheetsError> {
    SheetsOperation::UpdateValues(options).execute(client).await
}

pub async fn batch_update_values<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchUpdateValuesOptions,
) -> Result<BatchUpdateValuesResponse, SheetsError> {
    SheetsOperation::BatchUpdateValues(options)
        .execute(client)
        .await
}

pub async fn append_values<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &AppendValuesOptions,
) -> Result<AppendValuesResponse, SheetsError> {
    SheetsOperation::AppendValues(options).execute(client).await
}

pub async fn clear_values<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ClearValuesOptions,
) -> Result<ClearValuesResponse, SheetsError> {
    SheetsOperation::ClearValues(options).execute(client).await
}

pub async fn batch_clear_values<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchClearValuesOptions,
) -> Result<BatchClearValuesResponse, SheetsError> {
    SheetsOperation::BatchClearValues(options)
        .execute(client)
        .await
}

pub async fn batch_update_spreadsheet<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchUpdateSpreadsheetOptions,
) -> Result<BatchUpdateSpreadsheetResponse, SheetsError> {
    SheetsOperation::BatchUpdateSpreadsheet(options)
        .execute(client)
        .await
}

async fn send_json_request<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: RequestBuilder,
    scopes: &[&str],
) -> Result<Value, SheetsError> {
    let response = client
        .send_with_scopes(request, scopes)
        .await
        .map_err(SheetsError::Auth)?;

    parse_json_response(response).await
}

async fn send_json_request_with_office_file_fallback<S, F, Fut>(
    client: &AuthClient<'_, S>,
    request: RequestBuilder,
    scopes: &[&str],
    fallback: F,
) -> Result<Value, SheetsError>
where
    S: AccountStore,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<Value, SheetsError>>,
{
    let response = client
        .send_with_scopes(request, scopes)
        .await
        .map_err(SheetsError::Auth)?;
    let status = response.status();

    if status.is_success() {
        return response
            .json::<Value>()
            .await
            .map_err(|e| SheetsError::InvalidResponse(e.to_string()));
    }

    match status {
        StatusCode::NOT_FOUND => Err(SheetsError::NotFound),
        StatusCode::FORBIDDEN => Err(SheetsError::PermissionDenied),
        status => {
            let body = response.text().await.unwrap_or_default();
            if is_office_file_precondition_error(status, &body) {
                fallback().await
            } else {
                Err(SheetsError::Api { status, body })
            }
        }
    }
}

async fn send_empty_request<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: RequestBuilder,
    scopes: &[&str],
) -> Result<(), SheetsError> {
    let response = client
        .send_with_scopes(request, scopes)
        .await
        .map_err(SheetsError::Auth)?;

    ensure_success_response(response).await?;
    Ok(())
}

async fn get_spreadsheet_via_temporary_conversion<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetSpreadsheetOptions,
) -> Result<Spreadsheet, SheetsError> {
    read_values_via_temporary_conversion(
        client,
        &options.drive_files_url,
        &options.spreadsheet_id,
        |temporary_id| {
            let mut temporary_options = GetSpreadsheetOptions::new(temporary_id)
                .with_include_grid_data(options.include_grid_data)
                .with_ranges(options.ranges.clone())
                .with_spreadsheets_url(&options.spreadsheets_url)
                .with_drive_files_url(&options.drive_files_url);
            if let Some(fields) = &options.fields {
                temporary_options = temporary_options.with_fields(fields.clone());
            }
            temporary_options.request_url()
        },
    )
    .await
}

async fn get_values_via_temporary_conversion<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetValuesOptions,
) -> Result<ValueRange, SheetsError> {
    read_values_via_temporary_conversion(
        client,
        &options.drive_files_url,
        &options.spreadsheet_id,
        |temporary_id| {
            GetValuesOptions::new(temporary_id, options.range.clone())
                .with_value_render_option(options.value_render_option)
                .with_spreadsheets_url(&options.spreadsheets_url)
                .with_drive_files_url(&options.drive_files_url)
                .request_url()
        },
    )
    .await
}

async fn batch_get_values_via_temporary_conversion<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &BatchGetValuesOptions,
) -> Result<BatchGetValuesResponse, SheetsError> {
    read_values_via_temporary_conversion(
        client,
        &options.drive_files_url,
        &options.spreadsheet_id,
        |temporary_id| {
            BatchGetValuesOptions::new(temporary_id, options.ranges.clone())
                .with_value_render_option(options.value_render_option)
                .with_spreadsheets_url(&options.spreadsheets_url)
                .with_drive_files_url(&options.drive_files_url)
                .request_url()
        },
    )
    .await
}

async fn read_values_via_temporary_conversion<S: AccountStore>(
    client: &AuthClient<'_, S>,
    drive_files_url: &str,
    source_file_id: &str,
    converted_request_url: impl FnOnce(&str) -> Result<Url, SheetsError>,
) -> Result<Value, SheetsError> {
    let temporary_id =
        create_temporary_google_sheet(client, drive_files_url, source_file_id).await?;
    let request_url = converted_request_url(&temporary_id)?;
    let response = send_json_request(client, client.get(request_url), SHEETS_SCOPES).await;

    finish_temporary_conversion(client, drive_files_url, &temporary_id, response).await
}

async fn finish_temporary_conversion<S: AccountStore>(
    client: &AuthClient<'_, S>,
    drive_files_url: &str,
    temporary_id: &str,
    response: Result<Value, SheetsError>,
) -> Result<Value, SheetsError> {
    let delete_result = delete_temporary_google_sheet(client, drive_files_url, temporary_id).await;

    match (response, delete_result) {
        (Ok(response), Ok(())) => Ok(response),
        (Err(error), _) => Err(error),
        (Ok(_), Err(error)) => Err(error),
    }
}

async fn create_temporary_google_sheet<S: AccountStore>(
    client: &AuthClient<'_, S>,
    drive_files_url: &str,
    source_file_id: &str,
) -> Result<String, SheetsError> {
    let mut url = drive_file_url(drive_files_url, source_file_id, Some("copy"))?;
    url.query_pairs_mut().append_pair("fields", "id");
    let response = send_json_request(
        client,
        client.post(url).json(&serde_json::json!({
            "mimeType": GOOGLE_SHEETS_MIME_TYPE,
            "name": TEMPORARY_CONVERSION_NAME
        })),
        DRIVE_SCOPES,
    )
    .await?;

    response
        .get("id")
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| {
            SheetsError::InvalidResponse("Google Drive copy response did not include an id".into())
        })
}

async fn delete_temporary_google_sheet<S: AccountStore>(
    client: &AuthClient<'_, S>,
    drive_files_url: &str,
    file_id: &str,
) -> Result<(), SheetsError> {
    send_empty_request(
        client,
        client.delete(drive_file_url(drive_files_url, file_id, None)?),
        DRIVE_SCOPES,
    )
    .await
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

    let Ok(response) = serde_json::from_str::<GoogleApiErrorResponse>(body) else {
        return false;
    };

    response.error.is_office_file_precondition()
}

#[derive(Debug, Deserialize)]
struct GoogleApiErrorResponse {
    error: GoogleApiError,
}

#[derive(Debug, Deserialize)]
struct GoogleApiError {
    message: String,
    status: String,
}

impl GoogleApiError {
    fn is_office_file_precondition(&self) -> bool {
        self.status == OFFICE_FILE_PRECONDITION_STATUS
            && self.message.contains(OFFICE_FILE_PRECONDITION_MESSAGE)
    }
}
