use std::future::Future;
use std::io::{Read, Write};
use std::path::Path;

use anyhow::{Context, Result};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::calendar::{
    delete_event, get_calendar, get_event, insert_event, list_calendars, list_events, update_event,
    CalendarError, DeleteEventOptions, GetCalendarOptions, GetEventOptions, ListCalendarsOptions,
    ListEventsOptions, WriteEventOptions,
};
use crate::cli::{CalendarCalendarsCommand, CalendarCommand, CalendarEventsCommand};

const DEFAULT_LIST_LIMIT: u32 = 50;
const ALL_PAGE_SIZE: u32 = 250;

pub fn run<S: AccountStore>(
    cmd: CalendarCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    output_json_by_default: bool,
) -> Result<()> {
    match cmd {
        CalendarCommand::Calendars { command } => run_with_runtime(run_calendars_command_to(
            config,
            store,
            account_override,
            command,
            output_json_by_default,
            &mut std::io::stdout(),
            None,
            None,
        )),
        CalendarCommand::Events { command } => {
            let mut stdin = std::io::stdin();
            run_with_runtime(run_events_command_to(
                config,
                store,
                account_override,
                command,
                &mut stdin,
                output_json_by_default,
                &mut std::io::stdout(),
                None,
                None,
            ))
        }
    }
}

fn run_with_runtime(future: impl Future<Output = Result<()>>) -> Result<()> {
    let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
    runtime.block_on(future)
}

pub(super) async fn run_calendars_command_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CalendarCalendarsCommand,
    output_json_by_default: bool,
    out: &mut impl Write,
    base_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    match command {
        CalendarCalendarsCommand::List { limit, all, json } => {
            let json = json || output_json_by_default;
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            let calendars = collect_calendars(&client, limit, all, base_url)
                .await
                .context("failed to list Google Calendars")?;
            if json {
                write_ndjson(out, &calendars)
            } else {
                write_calendars_table(out, &calendars)
            }
        }
        CalendarCalendarsCommand::Get { calendar_id } => {
            let options = calendar_get_options(calendar_id.clone(), base_url);
            let target_resource_key = resource_key("calendar", &calendar_id);
            let calendar = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::GetCalendar(&options),
                state_path,
            )
            .await
            .context("failed to read Google Calendar")?;
            write_json_line(out, &calendar, "failed to serialize Calendar")
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_events_command_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CalendarEventsCommand,
    input: &mut impl Read,
    output_json_by_default: bool,
    out: &mut impl Write,
    base_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    match command {
        CalendarEventsCommand::List {
            calendar_id,
            limit,
            all,
            time_min,
            time_max,
            query,
            single_events,
            json,
        } => {
            let json = json || output_json_by_default;
            let options = list_events_options(
                calendar_id.clone(),
                requested_result_count(limit, all).unwrap_or(DEFAULT_LIST_LIMIT),
                time_min,
                time_max,
                query,
                single_events,
                None,
                base_url,
            );
            let target_resource_key = resource_key("calendar", &calendar_id);
            let events = collect_events_unified(
                config,
                store,
                account_override,
                &target_resource_key,
                options,
                limit,
                all,
                base_url,
                state_path,
            )
            .await
            .context("failed to list Google Calendar events")?;
            if json {
                write_ndjson(out, &events)
            } else {
                write_events_table(out, &events)
            }
        }
        CalendarEventsCommand::Get {
            calendar_id,
            event_id,
        } => {
            let options = get_event_options(calendar_id.clone(), event_id.clone(), base_url);
            let target_resource_key = calendar_event_resource_key(&calendar_id, &event_id);
            let event = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::GetEvent(&options),
                state_path,
            )
            .await
            .context("failed to read Google Calendar event")?;
            write_json_line(out, &event, "failed to serialize Calendar event")
        }
        CalendarEventsCommand::Create {
            calendar_id,
            event,
            summary,
            start,
            end,
            time_zone,
            all_day,
            location,
            description,
            attendee,
        } => {
            let request_body = match event {
                Some(event) => read_request_body(&event, input, "Google Calendar event")?,
                None => build_event_request_body(
                    summary,
                    start,
                    end,
                    time_zone,
                    all_day,
                    location,
                    description,
                    attendee,
                )?,
            };
            let options = write_event_options_insert(calendar_id.clone(), request_body, base_url);
            let target_resource_key = resource_key("calendar", &calendar_id);
            let event = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::InsertEvent(&options),
                state_path,
            )
            .await
            .context("failed to create Google Calendar event")?;
            write_json_line(out, &event, "failed to serialize Calendar event")
        }
        CalendarEventsCommand::Update {
            calendar_id,
            event_id,
            event,
        } => {
            let request_body = read_request_body(&event, input, "Google Calendar event")?;
            let options = write_event_options_update(
                calendar_id.clone(),
                event_id.clone(),
                request_body,
                base_url,
            );
            let target_resource_key = calendar_event_resource_key(&calendar_id, &event_id);
            let event = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::UpdateEvent(&options),
                state_path,
            )
            .await
            .context("failed to update Google Calendar event")?;
            write_json_line(out, &event, "failed to serialize Calendar event")
        }
        CalendarEventsCommand::Delete {
            calendar_id,
            event_id,
        } => {
            let options = delete_event_options(calendar_id.clone(), event_id.clone(), base_url);
            let target_resource_key = calendar_event_resource_key(&calendar_id, &event_id);
            run_with_calendar_delete_access(
                config,
                store,
                account_override,
                &target_resource_key,
                &options,
                state_path,
            )
            .await
            .context("failed to delete Google Calendar event")?;
            writeln!(out, "deleted\t{calendar_id}\t{event_id}").context("failed to write output")
        }
    }
}

async fn collect_calendars<S: AccountStore>(
    client: &AuthClient<'_, S>,
    limit: Option<u32>,
    all: bool,
    base_url: Option<&str>,
) -> Result<Vec<serde_json::Value>, CalendarError> {
    let mut remaining = requested_result_count(limit, all);
    let mut page_token = None;
    let mut items = Vec::new();

    loop {
        let Some(page_size) = next_page_size(remaining) else {
            break;
        };
        let options = list_calendars_options(page_size, page_token.take(), base_url);
        let page = list_calendars(client, &options).await?;
        let page_items = page
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let page_count = page_items.len() as u32;
        if let Some(left) = remaining.as_mut() {
            *left = left.saturating_sub(page_count);
        }
        items.extend(page_items);

        match page
            .get("nextPageToken")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        {
            Some(token) if should_fetch_next_page(remaining, all) => page_token = Some(token),
            _ => break,
        }
    }

    Ok(items)
}

#[allow(clippy::too_many_arguments)]
async fn collect_events_unified<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    first_options: ListEventsOptions,
    limit: Option<u32>,
    all: bool,
    base_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<Vec<serde_json::Value>, CalendarError> {
    let mut remaining = requested_result_count(limit, all);
    let mut page_token = None;
    let mut items = Vec::new();

    loop {
        let Some(page_size) = next_page_size(remaining) else {
            break;
        };
        let options = list_events_options(
            first_options.calendar_id.clone(),
            page_size,
            first_options.time_min.clone(),
            first_options.time_max.clone(),
            first_options.query.clone(),
            first_options.single_events,
            page_token.take(),
            base_url,
        );
        let page = run_with_calendar_unified_access(
            config,
            store,
            account_override,
            target_resource_key,
            CalendarAccessAttempt::ListEvents(&options),
            state_path,
        )
        .await?;
        let page_items = page
            .get("items")
            .and_then(serde_json::Value::as_array)
            .cloned()
            .unwrap_or_default();
        let page_count = page_items.len() as u32;
        if let Some(left) = remaining.as_mut() {
            *left = left.saturating_sub(page_count);
        }
        items.extend(page_items);

        match page
            .get("nextPageToken")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string)
        {
            Some(token) if should_fetch_next_page(remaining, all) => page_token = Some(token),
            _ => break,
        }
    }

    Ok(items)
}

enum CalendarAccessAttempt<'a> {
    GetCalendar(&'a GetCalendarOptions),
    ListEvents(&'a ListEventsOptions),
    GetEvent(&'a GetEventOptions),
    InsertEvent(&'a WriteEventOptions),
    UpdateEvent(&'a WriteEventOptions),
}

async fn run_with_calendar_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    attempt: CalendarAccessAttempt<'_>,
    state_path: Option<&Path>,
) -> Result<serde_json::Value, CalendarError> {
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, serde_json::Value, CalendarError> {
            Box::pin(run_calendar_access_as_account(
                config, store, &attempt, account,
            ))
        },
        is_target_access_failure,
    )
    .await
}

async fn run_calendar_access_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    attempt: &CalendarAccessAttempt<'_>,
    account: String,
) -> Result<serde_json::Value, CalendarError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))
        .map_err(CalendarError::Auth)?;
    match attempt {
        CalendarAccessAttempt::GetCalendar(options) => get_calendar(&client, options).await,
        CalendarAccessAttempt::ListEvents(options) => list_events(&client, options).await,
        CalendarAccessAttempt::GetEvent(options) => get_event(&client, options).await,
        CalendarAccessAttempt::InsertEvent(options) => insert_event(&client, options).await,
        CalendarAccessAttempt::UpdateEvent(options) => update_event(&client, options).await,
    }
}

async fn run_with_calendar_delete_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    options: &DeleteEventOptions,
    state_path: Option<&Path>,
) -> Result<(), CalendarError> {
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, (), CalendarError> {
            Box::pin(delete_event_as_account(config, store, options, account))
        },
        is_target_access_failure,
    )
    .await
}

async fn delete_event_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    options: &DeleteEventOptions,
    account: String,
) -> Result<(), CalendarError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))
        .map_err(CalendarError::Auth)?;
    delete_event(&client, options).await
}

fn is_target_access_failure(err: &CalendarError) -> bool {
    matches!(
        err,
        CalendarError::NotFound | CalendarError::PermissionDenied
    )
}

fn requested_result_count(limit: Option<u32>, all: bool) -> Option<u32> {
    if all {
        limit
    } else {
        Some(limit.unwrap_or(DEFAULT_LIST_LIMIT))
    }
}

fn next_page_size(remaining: Option<u32>) -> Option<u32> {
    let page_size = remaining.unwrap_or(ALL_PAGE_SIZE).min(ALL_PAGE_SIZE);
    (page_size > 0).then_some(page_size)
}

fn should_fetch_next_page(remaining: Option<u32>, all: bool) -> bool {
    remaining.map_or(all, |left| left > 0)
}

fn list_calendars_options(
    page_size: u32,
    page_token: Option<String>,
    base_url: Option<&str>,
) -> ListCalendarsOptions {
    let mut options = ListCalendarsOptions::new(page_size);
    if let Some(page_token) = page_token {
        options = options.with_page_token(page_token);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn calendar_get_options(calendar_id: String, base_url: Option<&str>) -> GetCalendarOptions {
    let mut options = GetCalendarOptions::new(calendar_id);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

#[allow(clippy::too_many_arguments)]
fn list_events_options(
    calendar_id: String,
    page_size: u32,
    time_min: Option<String>,
    time_max: Option<String>,
    query: Option<String>,
    single_events: bool,
    page_token: Option<String>,
    base_url: Option<&str>,
) -> ListEventsOptions {
    let mut options =
        ListEventsOptions::new(calendar_id, page_size).with_single_events(single_events);
    if let Some(time_min) = time_min {
        options = options.with_time_min(time_min);
    }
    if let Some(time_max) = time_max {
        options = options.with_time_max(time_max);
    }
    if let Some(query) = query {
        options = options.with_query(query);
    }
    if let Some(page_token) = page_token {
        options = options.with_page_token(page_token);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn get_event_options(
    calendar_id: String,
    event_id: String,
    base_url: Option<&str>,
) -> GetEventOptions {
    let mut options = GetEventOptions::new(calendar_id, event_id);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn write_event_options_insert(
    calendar_id: String,
    request_body: serde_json::Value,
    base_url: Option<&str>,
) -> WriteEventOptions {
    let mut options = WriteEventOptions::insert(calendar_id, request_body);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn write_event_options_update(
    calendar_id: String,
    event_id: String,
    request_body: serde_json::Value,
    base_url: Option<&str>,
) -> WriteEventOptions {
    let mut options = WriteEventOptions::update(calendar_id, event_id, request_body);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn delete_event_options(
    calendar_id: String,
    event_id: String,
    base_url: Option<&str>,
) -> DeleteEventOptions {
    let mut options = DeleteEventOptions::new(calendar_id, event_id);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn calendar_event_resource_key(calendar_id: &str, event_id: &str) -> String {
    resource_key("calendar", &format!("{calendar_id}/{event_id}"))
}

fn read_request_body(
    path_or_stdin: &str,
    input: &mut impl Read,
    request_name: &str,
) -> Result<serde_json::Value> {
    let (body, request_source) = if path_or_stdin == "-" {
        let mut body = String::new();
        input
            .read_to_string(&mut body)
            .with_context(|| format!("failed to read {request_name} from stdin"))?;
        (body, "stdin".to_string())
    } else {
        let body = std::fs::read_to_string(path_or_stdin)
            .with_context(|| format!("failed to read {request_name}: {path_or_stdin}"))?;
        (body, path_or_stdin.to_string())
    };

    serde_json::from_str(&body)
        .with_context(|| format!("failed to parse {request_name} from {request_source}"))
}

#[allow(clippy::too_many_arguments)]
fn build_event_request_body(
    summary: Option<String>,
    start: Option<String>,
    end: Option<String>,
    time_zone: Option<String>,
    all_day: bool,
    location: Option<String>,
    description: Option<String>,
    attendees: Vec<String>,
) -> Result<serde_json::Value> {
    let summary = summary.context("--summary is required unless --event is used")?;
    let start = start.context("--start is required unless --event is used")?;
    let end = end.context("--end is required unless --event is used")?;

    let mut body = serde_json::Map::from_iter([
        ("summary".to_string(), serde_json::Value::String(summary)),
        (
            "start".to_string(),
            event_time_body("start", start, time_zone.as_deref(), all_day)?,
        ),
        (
            "end".to_string(),
            event_time_body("end", end, time_zone.as_deref(), all_day)?,
        ),
    ]);

    if let Some(location) = location {
        body.insert("location".into(), serde_json::Value::String(location));
    }
    if let Some(description) = description {
        body.insert("description".into(), serde_json::Value::String(description));
    }
    if !attendees.is_empty() {
        body.insert(
            "attendees".into(),
            serde_json::Value::Array(
                attendees
                    .into_iter()
                    .map(|email| serde_json::json!({ "email": email }))
                    .collect(),
            ),
        );
    }

    Ok(serde_json::Value::Object(body))
}

fn event_time_body(
    field_name: &str,
    value: String,
    time_zone: Option<&str>,
    all_day: bool,
) -> Result<serde_json::Value> {
    if all_day {
        if time_zone.is_some() {
            anyhow::bail!("--time-zone cannot be used with --all-day");
        }
        validate_calendar_date(field_name, &value)?;
        return Ok(serde_json::json!({ "date": value }));
    }

    validate_calendar_date_time(field_name, &value)?;
    let mut body = serde_json::json!({ "dateTime": value });
    if let Some(time_zone) = time_zone {
        body["timeZone"] = serde_json::Value::String(time_zone.to_string());
    }
    Ok(body)
}

fn validate_calendar_date(field_name: &str, value: &str) -> Result<()> {
    chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
        .with_context(|| format!("--{field_name} must be YYYY-MM-DD when --all-day is used"))?;
    Ok(())
}

fn validate_calendar_date_time(field_name: &str, value: &str) -> Result<()> {
    chrono::DateTime::parse_from_rfc3339(value)
        .with_context(|| format!("--{field_name} must be an RFC3339 date-time"))?;
    Ok(())
}

fn write_calendars_table(out: &mut impl Write, calendars: &[serde_json::Value]) -> Result<()> {
    writeln!(out, "SUMMARY\tCALENDAR ID\tACCESS ROLE\tTIME ZONE")
        .context("failed to write output")?;
    for calendar in calendars {
        writeln!(
            out,
            "{}\t{}\t{}\t{}",
            string_field(calendar, "summary"),
            string_field(calendar, "id"),
            string_field(calendar, "accessRole"),
            string_field(calendar, "timeZone"),
        )
        .context("failed to write output")?;
    }
    Ok(())
}

fn write_events_table(out: &mut impl Write, events: &[serde_json::Value]) -> Result<()> {
    writeln!(out, "SUMMARY\tEVENT ID\tSTART\tEND\tSTATUS").context("failed to write output")?;
    for event in events {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            string_field(event, "summary"),
            string_field(event, "id"),
            event_time(event, "start"),
            event_time(event, "end"),
            string_field(event, "status"),
        )
        .context("failed to write output")?;
    }
    Ok(())
}

fn event_time(event: &serde_json::Value, field: &str) -> String {
    event
        .get(field)
        .and_then(|value| {
            value
                .get("dateTime")
                .or_else(|| value.get("date"))
                .and_then(serde_json::Value::as_str)
        })
        .unwrap_or("")
        .to_string()
}

fn string_field(value: &serde_json::Value, field: &str) -> String {
    value
        .get(field)
        .and_then(serde_json::Value::as_str)
        .unwrap_or("")
        .replace('\t', " ")
        .replace('\n', " ")
}

fn write_ndjson(out: &mut impl Write, values: &[serde_json::Value]) -> Result<()> {
    for value in values {
        write_json_line(out, value, "failed to serialize Calendar row")?;
    }
    Ok(())
}

fn write_json_line(out: &mut impl Write, value: &serde_json::Value, context: &str) -> Result<()> {
    serde_json::to_writer(&mut *out, value).with_context(|| context.to_string())?;
    writeln!(out).context("failed to write output")
}
