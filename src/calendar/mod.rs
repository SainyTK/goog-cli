pub mod error;

pub use error::CalendarError;

use reqwest::{header::CONTENT_LENGTH, Method, Response, StatusCode};
use serde_json::Value;
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const CALENDAR_SCOPE: &str = "https://www.googleapis.com/auth/calendar";
pub const CALENDAR_SCOPES: &[&str] = &[CALENDAR_SCOPE];
const CALENDAR_BASE_URL: &str = "https://www.googleapis.com/calendar/v3";

pub type Calendar = Value;
pub type Acl = Value;
pub type AclRule = Value;
pub type CalendarList = Value;
pub type Event = Value;
pub type Events = Value;
pub type FreeBusy = Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendUpdates {
    All,
    ExternalOnly,
    None,
}

impl SendUpdates {
    fn api_value(self) -> &'static str {
        match self {
            SendUpdates::All => "all",
            SendUpdates::ExternalOnly => "externalOnly",
            SendUpdates::None => "none",
        }
    }
}

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
pub struct InsertCalendarOptions {
    pub request_body: Value,
    base_url: String,
}

impl InsertCalendarOptions {
    pub fn new(request_body: Value) -> Self {
        Self {
            request_body,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        calendar_url(&self.base_url, &["calendars"])
    }
}

#[derive(Debug, Clone)]
pub struct UpdateCalendarOptions {
    pub calendar_id: String,
    pub request_body: Value,
    base_url: String,
}

impl UpdateCalendarOptions {
    pub fn new(calendar_id: impl Into<String>, request_body: Value) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            request_body,
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
pub struct DeleteCalendarOptions {
    pub calendar_id: String,
    base_url: String,
}

impl DeleteCalendarOptions {
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
pub struct ListAclOptions {
    pub calendar_id: String,
    pub max_results: u32,
    pub page_token: Option<String>,
    base_url: String,
}

impl ListAclOptions {
    pub fn new(calendar_id: impl Into<String>, max_results: u32) -> Self {
        Self {
            calendar_id: calendar_id.into(),
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
        let mut url = calendar_url(&self.base_url, &["calendars", &self.calendar_id, "acl"])?;
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
pub struct GetAclOptions {
    pub calendar_id: String,
    pub rule_id: String,
    base_url: String,
}

impl GetAclOptions {
    pub fn new(calendar_id: impl Into<String>, rule_id: impl Into<String>) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            rule_id: rule_id.into(),
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
            &["calendars", &self.calendar_id, "acl", &self.rule_id],
        )
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
    pub send_updates: Option<SendUpdates>,
    base_url: String,
}

impl WriteEventOptions {
    pub fn insert(calendar_id: impl Into<String>, request_body: Value) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            event_id: None,
            request_body,
            send_updates: None,
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
            send_updates: None,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub fn patch(
        calendar_id: impl Into<String>,
        event_id: impl Into<String>,
        request_body: Value,
    ) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            event_id: Some(event_id.into()),
            request_body,
            send_updates: None,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub fn with_send_updates(mut self, send_updates: SendUpdates) -> Self {
        self.send_updates = Some(send_updates);
        self
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn insert_url(&self) -> Result<Url, CalendarError> {
        let mut url = calendar_url(&self.base_url, &["calendars", &self.calendar_id, "events"])?;
        append_send_updates(&mut url, self.send_updates);
        Ok(url)
    }

    fn update_url(&self) -> Result<Url, CalendarError> {
        let event_id = self
            .event_id
            .as_deref()
            .ok_or_else(|| CalendarError::InvalidResponse("event_id was missing".into()))?;
        let mut url = calendar_url(
            &self.base_url,
            &["calendars", &self.calendar_id, "events", event_id],
        )?;
        append_send_updates(&mut url, self.send_updates);
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct DeleteEventOptions {
    pub calendar_id: String,
    pub event_id: String,
    pub send_updates: Option<SendUpdates>,
    base_url: String,
}

impl DeleteEventOptions {
    pub fn new(calendar_id: impl Into<String>, event_id: impl Into<String>) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            event_id: event_id.into(),
            send_updates: None,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub fn with_send_updates(mut self, send_updates: SendUpdates) -> Self {
        self.send_updates = Some(send_updates);
        self
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        let mut url = calendar_url(
            &self.base_url,
            &["calendars", &self.calendar_id, "events", &self.event_id],
        )?;
        append_send_updates(&mut url, self.send_updates);
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct MoveEventOptions {
    pub source_calendar_id: String,
    pub event_id: String,
    pub destination_calendar_id: String,
    base_url: String,
}

impl MoveEventOptions {
    pub fn new(
        source_calendar_id: impl Into<String>,
        event_id: impl Into<String>,
        destination_calendar_id: impl Into<String>,
    ) -> Self {
        Self {
            source_calendar_id: source_calendar_id.into(),
            event_id: event_id.into(),
            destination_calendar_id: destination_calendar_id.into(),
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        let mut url = calendar_url(
            &self.base_url,
            &[
                "calendars",
                &self.source_calendar_id,
                "events",
                &self.event_id,
                "move",
            ],
        )?;
        url.query_pairs_mut()
            .append_pair("destination", &self.destination_calendar_id);
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct QuickAddEventOptions {
    pub calendar_id: String,
    pub text: String,
    pub send_updates: Option<SendUpdates>,
    base_url: String,
}

impl QuickAddEventOptions {
    pub fn new(calendar_id: impl Into<String>, text: impl Into<String>) -> Self {
        Self {
            calendar_id: calendar_id.into(),
            text: text.into(),
            send_updates: None,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub fn with_send_updates(mut self, send_updates: SendUpdates) -> Self {
        self.send_updates = Some(send_updates);
        self
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        let mut url = calendar_url(
            &self.base_url,
            &["calendars", &self.calendar_id, "events", "quickAdd"],
        )?;
        {
            let mut query = url.query_pairs_mut();
            query.append_pair("text", &self.text);
            if let Some(send_updates) = self.send_updates {
                query.append_pair("sendUpdates", send_updates.api_value());
            }
        }
        Ok(url)
    }
}

#[derive(Debug, Clone)]
pub struct FreeBusyOptions {
    pub request_body: Value,
    base_url: String,
}

impl FreeBusyOptions {
    pub fn new(request_body: Value) -> Self {
        Self {
            request_body,
            base_url: CALENDAR_BASE_URL.to_string(),
        }
    }

    pub(super) fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, CalendarError> {
        calendar_url(&self.base_url, &["freeBusy"])
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

fn append_send_updates(url: &mut Url, send_updates: Option<SendUpdates>) {
    if let Some(send_updates) = send_updates {
        url.query_pairs_mut()
            .append_pair("sendUpdates", send_updates.api_value());
    }
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

pub async fn insert_calendar<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &InsertCalendarOptions,
) -> Result<Calendar, CalendarError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .json(&options.request_body),
    )
    .await
}

pub async fn update_calendar<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &UpdateCalendarOptions,
) -> Result<Calendar, CalendarError> {
    send_json_request(
        client,
        client
            .put(options.request_url()?)
            .json(&options.request_body),
    )
    .await
}

pub async fn patch_calendar<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &UpdateCalendarOptions,
) -> Result<Calendar, CalendarError> {
    send_json_request(
        client,
        client
            .request(Method::PATCH, options.request_url()?)
            .json(&options.request_body),
    )
    .await
}

pub async fn delete_calendar<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &DeleteCalendarOptions,
) -> Result<(), CalendarError> {
    let response = client
        .send_with_scopes(client.delete(options.request_url()?), CALENDAR_SCOPES)
        .await
        .map_err(CalendarError::Auth)?;
    parse_empty_response(response).await
}

pub async fn list_acl<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListAclOptions,
) -> Result<Acl, CalendarError> {
    send_json_request(client, client.get(options.request_url()?)).await
}

pub async fn get_acl<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetAclOptions,
) -> Result<AclRule, CalendarError> {
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

pub async fn patch_event<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &WriteEventOptions,
) -> Result<Event, CalendarError> {
    send_json_request(
        client,
        client
            .request(Method::PATCH, options.update_url()?)
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

pub async fn move_event<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &MoveEventOptions,
) -> Result<Event, CalendarError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .header(CONTENT_LENGTH, "0")
            .body(Vec::new()),
    )
    .await
}

pub async fn quick_add_event<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &QuickAddEventOptions,
) -> Result<Event, CalendarError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .header(CONTENT_LENGTH, "0")
            .body(Vec::new()),
    )
    .await
}

pub async fn query_freebusy<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &FreeBusyOptions,
) -> Result<FreeBusy, CalendarError> {
    send_json_request(
        client,
        client
            .post(options.request_url()?)
            .json(&options.request_body),
    )
    .await
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
