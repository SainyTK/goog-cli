pub mod error;

pub use error::CalendarError;

use reqwest::{Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar";
pub const CALENDAR_SCOPES: &[&str] = &[CALENDAR_SCOPE];
const CALENDAR_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

pub type Calendar = Value;
pub type CalendarList = Value;
pub type Event = Value;
pub type Events = Value;

#[derive(Debug, Clone)]
pub struct ListCalendarsOptions {
    pub max_results: u32,
    pub page_token: Option<String>,
    base_url: String,
}

impl ListCalendarsOptions {
    pub fn new(max_results: u32) -> Self {
        Self {
            max_results,
            page_token: None,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub fn with_page_token(mut self, page_token: impl Into<String>) -> Self {
        self.page_token = Some(page_token.into());
        self
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        let mut url = calendar_url(&self.base_url, &["users", "me", "calendarList"])?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("maxResults", &self.max_results.to_string());
            if let Some(page_token) = &self.page_token {
                query.append_pair("pageToken", page_token);
            }
        }
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct GetCalendarOptions {
    pub calendar_id: String,
    base_url: String,
}

impl GetCalendarOptions {
    pub fn new(calendar_id: impl Into<String>) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        calendar_url(&self.base_url, &["calendars", &self.calendar_id])
    }
}

#[derive(Debug, Clone)]
pub struct ListEventsOptions {
    pub calendar_id: String,
    pub max_results: u32,
    pub page_token: Option<String>,
    pub time_min: Option<String>,
    pub time_max: Option<String>,
    pub query: Option<String>,
    pub single_events: bool,
    base_url: String,
}

impl ListEventsOptions {
    pub fn new(calendar_id: impl Into<String>, max_results: u32) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            max_results,
            page_token: None,
            time_min: None,
            time_max: None,
            query: None,
            single_events: false,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub fn with_page_token(mut self, page_token: impl Into<String>) -> Self {
        self.page_token = Some(page_token.into());
        self
    }

    pub fn with_time_min(mut self, time_min: impl Into<String>) -> Self {
        self.time_min = Some(time_min.into());
        self
    }

    pub fn with_time_max(mut self, time_max: impl Into<String>) -> Self {
        self.time_max = Some(time_max.into());
        self
    }

    pub fn with_query(mut self, query: impl Into<String>) -> Self {
        self.query = Some(query.into());
        self
    }

    pub fn with_single_events(mut self, single_events: bool) -> Self {
        self.single_events = single_events;
        self
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        let mut url = calendar_url(&self.base_url, &["calendars", &self.calendar_id, "events"])?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("maxResults", &self.max_results.to_string());
            if let Some(page_token) = &self.page_token {
                query.append_pair("pageToken", page_token);
            }
            if let Some(time_min) = &self.time_min {
                query.append_pair("timeMin", time_min);
            }
            if let Some(time_max) = &self.time_max {
                query.append_pair("timeMax", time_max);
            }
            if let Some(query_text) = &self.query {
                query.append_pair("q", query_text);
            }
            if self.single_events {
                query.append_pair("singleEvents", "true");
            }
        }
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct GetEventOptions {
    pub calendar_id: String,
    pub event_id: String,
    base_url: String,
}

impl GetEventOptions {
    pub fn new(calendar_id: impl Into<String>, event_id: impl Into<String>) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            event_id: event_id.into(),
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        calendar_url(
            &self.base_url,
            &["calendars", &self.calendar_id, "events", &self.event_id],
        )
    }
}

#[derive(Debug, Clone)]
pub struct WriteEventOptions {
    pub calendar_id: String,
    pub event_id: Option<String>,
    pub request_body: Value,
    base_url: String,
}

impl WriteEventOptions {
    pub fn insert(calendar_id: impl Into<String>, request_body: Value) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            event_id: None,
            request_body,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub fn update(
        calendar_id: impl Into<String>,
        event_id: impl Into<String>,
        request_body: Value,
    ) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            event_id: Some(event_id.into()),
            request_body,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn insert_url(&self) -> Result<Url, CalendarError> {
        calendar_url(&self.base_url, &["calendars", &self.calendar_id, "events"])
    }

    fn update_url(&self) -> Result<Url, CalendarError> {
        let event_id = self
            .event_id
            .as_deref()
            .ok_or_else(|| CalendarError::InvalidResponse("event_id was missing".into()))?;
        calendar_url(
            &self.base_url,
            &["calendars", &self.calendar_id, "events", event_id],
        )
    }
}

#[derive(Debug, Clone)]
pub struct DeleteEventOptions {
    pub calendar_id: String,
    pub event_id: String,
    base_url: String,
}

impl DeleteEventOptions {
    pub fn new(calendar_id: impl Into<String>, event_id: impl Into<String>) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            event_id: event_id.into(),
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        calendar_url(
            &self.base_url,
            &["calendars", &self.calendar_id, "events", &self.event_id],
        )
    }
}

fn calendar_url(base_url: &str, path_segments: &[&str]) -> Result<Url, CalendarError> {
    let mut url = Url::parse(base_url)?;
    {
        let mut segments = url.path_segments_mut().map_err(|_| {
            CalendarError::InvalidResponse("Google Calendar API URL cannot be a base".into())
        })?;
        for segment in path_segments {
            segments.push(segment);
        }
    }
    Ok(url)
}

pub async fn list_calendars<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListCalendarsOptions,
) -> Result<CalendarList, CalendarError> {
    send_json_request(client, client.get(options.request_url()?)).await
}

pub async fn get_calendar<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetCalendarOptions,
) -> Result<Calendar, CalendarError> {
    send_json_request(client, client.get(options.request_url()?)).await
}

pub async fn list_events<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListEventsOptions,
) -> Result<Events, CalendarError> {
    send_json_request(client, client.get(options.request_url()?)).await
}

pub async fn get_event<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetEventOptions,
) -> Result<Event, CalendarError> {
    send_json_request(client, client.get(options.request_url()?)).await
}

pub async fn insert_event<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &WriteEventOptions,
) -> Result<Event, CalendarError> {
    send_json_request(
        client,
        client
            .post(options.insert_url()?)
            .json(&options.request_body),
    )
    .await
}

pub async fn update_event<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &WriteEventOptions,
) -> Result<Event, CalendarError> {
    send_json_request(
        client,
        client
            .put(options.update_url()?)
            .json(&options.request_body),
    )
    .await
}

pub async fn delete_event<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DeleteEventOptions,
) -> Result<(), CalendarError> {
    let response = client
        .send_with_scopes(client.delete(options.request_url()?), CALENDAR_SCOPES)
        .await
        .map_err(CalendarError::Auth)?;
    parse_empty_response(response).await
}

async fn send_json_request<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: reqwest::RequestBuilder,
) -> Result<Value, CalendarError> {
    let response = client
        .send_with_scopes(request, CALENDAR_SCOPES)
        .await
        .map_err(CalendarError::Auth)?;

    parse_json_response(response).await
}

async fn parse_json_response(response: Response) -> Result<Value, CalendarError> {
    let status = response.status();
    if status.is_success() {
        return response
            .json()
            .await
            .map_err(|e| CalendarError::InvalidResponse(e.to_string()));
    }

    let body = response.text().await.unwrap_or_default();
    map_error_response(status, body)
}

async fn parse_empty_response(response: Response) -> Result<(), CalendarError> {
    let status = response.status();
    if status.is_success() {
        return Ok(());
    }

    let body = response.text().await.unwrap_or_default();
    map_error_response(status, body)
}

fn map_error_response<T>(status: StatusCode, body: String) -> Result<T, CalendarError> {
    match status {
        StatusCode::NOT_FOUND => Err(CalendarError::NotFound),
        StatusCode::FORBIDDEN => Err(CalendarError::PermissionDenied),
        _ => Err(CalendarError::Api { status, body }),
    }
}
