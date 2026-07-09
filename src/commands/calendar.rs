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
    delete_calendar, delete_event, get_acl, get_calendar, get_event, insert_calendar, insert_event,
    list_acl, list_calendars, list_events, move_event, patch_calendar, patch_event, query_freebusy,
    quick_add_event, update_calendar, update_event, CalendarError, DeleteCalendarOptions,
    DeleteEventOptions, FreeBusyOptions, GetAclOptions, GetCalendarOptions, GetEventOptions,
    InsertCalendarOptions, ListAclOptions, ListCalendarsOptions, ListEventsOptions,
    MoveEventOptions, QuickAddEventOptions, SendUpdates, UpdateCalendarOptions, WriteEventOptions,
};
use crate::cli::{
    CalendarAclCommand, CalendarCalendarsCommand, CalendarCommand, CalendarEventsCommand,
    CalendarSendUpdates,
};

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
        CalendarCommand::Acl { command } => run_with_runtime(run_acl_command_to(
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
        CalendarCommand::Freebusy {
            time_min,
            time_max,
            calendars,
            time_zone,
            group_expansion_max,
            calendar_expansion_max,
            json,
        } => run_with_runtime(run_freebusy_command_to(
            config,
            store,
            account_override,
            FreeBusyCommand {
                time_min,
                time_max,
                calendars,
                time_zone,
                group_expansion_max,
                calendar_expansion_max,
                json,
            },
            output_json_by_default,
            &mut std::io::stdout(),
            None,
            None,
        )),
    }
}

pub(super) struct FreeBusyCommand {
    pub time_min: String,
    pub time_max: String,
    pub calendars: Vec<String>,
    pub time_zone: Option<String>,
    pub group_expansion_max: Option<u32>,
    pub calendar_expansion_max: Option<u32>,
    pub json: bool,
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
        CalendarCalendarsCommand::Create {
            summary,
            description,
            location,
            time_zone,
        } => {
            let options = insert_calendar_options(
                build_calendar_request_body(summary, description, location, time_zone),
                base_url,
            );
            let client = AuthClient::from_config(config.clone(), store, account_override)?;
            let calendar = insert_calendar(&client, &options)
                .await
                .context("failed to create Google Calendar")?;
            write_json_line(out, &calendar, "failed to serialize Calendar")
        }
        CalendarCalendarsCommand::Update {
            calendar_id,
            summary,
            description,
            location,
            time_zone,
        } => {
            let options = update_calendar_options(
                calendar_id.clone(),
                build_calendar_request_body(summary, description, location, time_zone),
                base_url,
            );
            let target_resource_key = resource_key("calendar", &calendar_id);
            let calendar = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::UpdateCalendar(&options),
                state_path,
            )
            .await
            .context("failed to update Google Calendar")?;
            write_json_line(out, &calendar, "failed to serialize Calendar")
        }
        CalendarCalendarsCommand::Patch {
            calendar_id,
            summary,
            description,
            location,
            time_zone,
        } => {
            let options = update_calendar_options(
                calendar_id.clone(),
                build_calendar_patch_request_body(summary, description, location, time_zone)?,
                base_url,
            );
            let target_resource_key = resource_key("calendar", &calendar_id);
            let calendar = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::PatchCalendar(&options),
                state_path,
            )
            .await
            .context("failed to patch Google Calendar")?;
            write_json_line(out, &calendar, "failed to serialize Calendar")
        }
        CalendarCalendarsCommand::Delete { calendar_id } => {
            let options = delete_calendar_options(calendar_id.clone(), base_url);
            let target_resource_key = resource_key("calendar", &calendar_id);
            run_with_calendar_delete_calendar_access(
                config,
                store,
                account_override,
                &target_resource_key,
                &options,
                state_path,
            )
            .await
            .context("failed to delete Google Calendar")?;
            writeln!(out, "deleted\t{calendar_id}").context("failed to write output")
        }
    }
}

pub(super) async fn run_acl_command_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: CalendarAclCommand,
    output_json_by_default: bool,
    out: &mut impl Write,
    base_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    match command {
        CalendarAclCommand::List {
            calendar_id,
            limit,
            all,
            json,
        } => {
            let json = json || output_json_by_default;
            let target_resource_key = resource_key("calendar-acl", &calendar_id);
            let rules = collect_acl(
                config,
                store,
                account_override,
                &target_resource_key,
                calendar_id,
                limit,
                all,
                base_url,
                state_path,
            )
            .await
            .context("failed to list Google Calendar ACL rules")?;
            if json {
                write_ndjson(out, &rules)
            } else {
                write_acl_table(out, &rules)
            }
        }
        CalendarAclCommand::Get {
            calendar_id,
            rule_id,
            json,
        } => {
            let json = json || output_json_by_default;
            let target_resource_key = resource_key("calendar-acl", &calendar_id);
            let options = get_acl_options(calendar_id, rule_id, base_url);
            let rule = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::GetAcl(&options),
                state_path,
            )
            .await
            .context("failed to get Google Calendar ACL rule")?;
            if json {
                write_json_line(out, &rule, "failed to serialize Calendar ACL rule")
            } else {
                write_acl_rule_table(out, &rule)
            }
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
            recurrence,
            reminder,
            no_reminders,
            send_updates,
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
                    recurrence,
                    reminder,
                    no_reminders,
                )?,
            };
            let options = write_event_options_insert(
                calendar_id.clone(),
                request_body,
                send_updates.map(SendUpdates::from),
                base_url,
            );
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
            summary,
            start,
            end,
            time_zone,
            all_day,
            location,
            description,
            attendee,
            recurrence,
            reminder,
            no_reminders,
            send_updates,
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
                    recurrence,
                    reminder,
                    no_reminders,
                )?,
            };
            let options = write_event_options_update(
                calendar_id.clone(),
                event_id.clone(),
                request_body,
                send_updates.map(SendUpdates::from),
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
        CalendarEventsCommand::Patch {
            calendar_id,
            event_id,
            event,
            summary,
            start,
            end,
            time_zone,
            all_day,
            location,
            description,
            attendee,
            recurrence,
            reminder,
            no_reminders,
            send_updates,
        } => {
            let request_body = match event {
                Some(event) => read_request_body(&event, input, "Google Calendar event patch")?,
                None => build_event_patch_body(
                    summary,
                    start,
                    end,
                    time_zone,
                    all_day,
                    location,
                    description,
                    attendee,
                    recurrence,
                    reminder,
                    no_reminders,
                )?,
            };
            let options = write_event_options_patch(
                calendar_id.clone(),
                event_id.clone(),
                request_body,
                send_updates.map(SendUpdates::from),
                base_url,
            );
            let target_resource_key = calendar_event_resource_key(&calendar_id, &event_id);
            let event = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::PatchEvent(&options),
                state_path,
            )
            .await
            .context("failed to patch Google Calendar event")?;
            write_json_line(out, &event, "failed to serialize Calendar event")
        }
        CalendarEventsCommand::Move {
            source_calendar_id,
            event_id,
            destination,
        } => {
            let options = move_event_options(
                source_calendar_id.clone(),
                event_id.clone(),
                destination.clone(),
                base_url,
            );
            let target_resource_key = calendar_event_resource_key(&source_calendar_id, &event_id);
            let event = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::MoveEvent(&options),
                state_path,
            )
            .await
            .context("failed to move Google Calendar event")?;
            write_json_line(out, &event, "failed to serialize Calendar event")
        }
        CalendarEventsCommand::QuickAdd {
            calendar_id,
            text,
            send_updates,
        } => {
            let options = quick_add_event_options(
                calendar_id.clone(),
                text,
                send_updates.map(SendUpdates::from),
                base_url,
            );
            let target_resource_key = resource_key("calendar", &calendar_id);
            let event = run_with_calendar_unified_access(
                config,
                store,
                account_override,
                &target_resource_key,
                CalendarAccessAttempt::QuickAddEvent(&options),
                state_path,
            )
            .await
            .context("failed to quick-add Google Calendar event")?;
            write_json_line(out, &event, "failed to serialize Calendar event")
        }
        CalendarEventsCommand::Delete {
            calendar_id,
            event_id,
            send_updates,
        } => {
            let options = delete_event_options(
                calendar_id.clone(),
                event_id.clone(),
                send_updates.map(SendUpdates::from),
                base_url,
            );
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

pub(super) async fn run_freebusy_command_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    command: FreeBusyCommand,
    output_json_by_default: bool,
    out: &mut impl Write,
    base_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let json = command.json || output_json_by_default;
    let options = freebusy_options(&command, base_url)?;
    let target_resource_key = freebusy_resource_key(&command.calendars);
    let response = run_with_calendar_unified_access(
        config,
        store,
        account_override,
        &target_resource_key,
        CalendarAccessAttempt::FreeBusy(&options),
        state_path,
    )
    .await
    .context("failed to query Google Calendar free/busy")?;

    if json {
        write_json_line(
            out,
            &response,
            "failed to serialize Calendar free/busy response",
        )
    } else {
        write_freebusy_table(out, &response)
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

#[allow(clippy::too_many_arguments)]
async fn collect_acl<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    calendar_id: String,
    limit: Option<u32>,
    all: bool,
    base_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<Vec<serde_json::Value>, CalendarError> {
    let mut items = Vec::new();
    let mut remaining = requested_result_count(limit, all);
    let mut page_token = None;

    while let Some(page_size) = next_page_size(remaining) {
        let options = list_acl_options(calendar_id.clone(), page_size, page_token.take(), base_url);
        let page = run_with_calendar_unified_access(
            config,
            store,
            account_override,
            target_resource_key,
            CalendarAccessAttempt::ListAcl(&options),
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
    UpdateCalendar(&'a UpdateCalendarOptions),
    PatchCalendar(&'a UpdateCalendarOptions),
    ListAcl(&'a ListAclOptions),
    GetAcl(&'a GetAclOptions),
    ListEvents(&'a ListEventsOptions),
    GetEvent(&'a GetEventOptions),
    InsertEvent(&'a WriteEventOptions),
    UpdateEvent(&'a WriteEventOptions),
    PatchEvent(&'a WriteEventOptions),
    MoveEvent(&'a MoveEventOptions),
    QuickAddEvent(&'a QuickAddEventOptions),
    FreeBusy(&'a FreeBusyOptions),
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
        CalendarAccessAttempt::UpdateCalendar(options) => update_calendar(&client, options).await,
        CalendarAccessAttempt::PatchCalendar(options) => patch_calendar(&client, options).await,
        CalendarAccessAttempt::ListAcl(options) => list_acl(&client, options).await,
        CalendarAccessAttempt::GetAcl(options) => get_acl(&client, options).await,
        CalendarAccessAttempt::ListEvents(options) => list_events(&client, options).await,
        CalendarAccessAttempt::GetEvent(options) => get_event(&client, options).await,
        CalendarAccessAttempt::InsertEvent(options) => insert_event(&client, options).await,
        CalendarAccessAttempt::UpdateEvent(options) => update_event(&client, options).await,
        CalendarAccessAttempt::PatchEvent(options) => patch_event(&client, options).await,
        CalendarAccessAttempt::MoveEvent(options) => move_event(&client, options).await,
        CalendarAccessAttempt::QuickAddEvent(options) => quick_add_event(&client, options).await,
        CalendarAccessAttempt::FreeBusy(options) => query_freebusy(&client, options).await,
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

async fn run_with_calendar_delete_calendar_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    options: &DeleteCalendarOptions,
    state_path: Option<&Path>,
) -> Result<(), CalendarError> {
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, (), CalendarError> {
            Box::pin(delete_calendar_as_account(config, store, options, account))
        },
        is_target_access_failure,
    )
    .await
}

async fn delete_calendar_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    options: &DeleteCalendarOptions,
    account: String,
) -> Result<(), CalendarError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))
        .map_err(CalendarError::Auth)?;
    delete_calendar(&client, options).await
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

fn insert_calendar_options(
    request_body: serde_json::Value,
    base_url: Option<&str>,
) -> InsertCalendarOptions {
    let mut options = InsertCalendarOptions::new(request_body);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn update_calendar_options(
    calendar_id: String,
    request_body: serde_json::Value,
    base_url: Option<&str>,
) -> UpdateCalendarOptions {
    let mut options = UpdateCalendarOptions::new(calendar_id, request_body);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn delete_calendar_options(calendar_id: String, base_url: Option<&str>) -> DeleteCalendarOptions {
    let mut options = DeleteCalendarOptions::new(calendar_id);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn list_acl_options(
    calendar_id: String,
    page_size: u32,
    page_token: Option<String>,
    base_url: Option<&str>,
) -> ListAclOptions {
    let mut options = ListAclOptions::new(calendar_id, page_size);
    if let Some(page_token) = page_token {
        options = options.with_page_token(page_token);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn get_acl_options(calendar_id: String, rule_id: String, base_url: Option<&str>) -> GetAclOptions {
    let mut options = GetAclOptions::new(calendar_id, rule_id);
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
    send_updates: Option<SendUpdates>,
    base_url: Option<&str>,
) -> WriteEventOptions {
    let mut options = WriteEventOptions::insert(calendar_id, request_body);
    if let Some(send_updates) = send_updates {
        options = options.with_send_updates(send_updates);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn write_event_options_update(
    calendar_id: String,
    event_id: String,
    request_body: serde_json::Value,
    send_updates: Option<SendUpdates>,
    base_url: Option<&str>,
) -> WriteEventOptions {
    let mut options = WriteEventOptions::update(calendar_id, event_id, request_body);
    if let Some(send_updates) = send_updates {
        options = options.with_send_updates(send_updates);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn write_event_options_patch(
    calendar_id: String,
    event_id: String,
    request_body: serde_json::Value,
    send_updates: Option<SendUpdates>,
    base_url: Option<&str>,
) -> WriteEventOptions {
    let mut options = WriteEventOptions::patch(calendar_id, event_id, request_body);
    if let Some(send_updates) = send_updates {
        options = options.with_send_updates(send_updates);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn delete_event_options(
    calendar_id: String,
    event_id: String,
    send_updates: Option<SendUpdates>,
    base_url: Option<&str>,
) -> DeleteEventOptions {
    let mut options = DeleteEventOptions::new(calendar_id, event_id);
    if let Some(send_updates) = send_updates {
        options = options.with_send_updates(send_updates);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn move_event_options(
    source_calendar_id: String,
    event_id: String,
    destination_calendar_id: String,
    base_url: Option<&str>,
) -> MoveEventOptions {
    let mut options = MoveEventOptions::new(source_calendar_id, event_id, destination_calendar_id);
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn quick_add_event_options(
    calendar_id: String,
    text: String,
    send_updates: Option<SendUpdates>,
    base_url: Option<&str>,
) -> QuickAddEventOptions {
    let mut options = QuickAddEventOptions::new(calendar_id, text);
    if let Some(send_updates) = send_updates {
        options = options.with_send_updates(send_updates);
    }
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    options
}

fn freebusy_options(command: &FreeBusyCommand, base_url: Option<&str>) -> Result<FreeBusyOptions> {
    validate_calendar_date_time("time-min", &command.time_min)?;
    validate_calendar_date_time("time-max", &command.time_max)?;

    let mut body = serde_json::Map::from_iter([
        (
            "timeMin".to_string(),
            serde_json::Value::String(command.time_min.clone()),
        ),
        (
            "timeMax".to_string(),
            serde_json::Value::String(command.time_max.clone()),
        ),
        (
            "items".to_string(),
            serde_json::Value::Array(
                command
                    .calendars
                    .iter()
                    .map(|id| serde_json::json!({ "id": id }))
                    .collect(),
            ),
        ),
    ]);

    if let Some(time_zone) = &command.time_zone {
        body.insert(
            "timeZone".into(),
            serde_json::Value::String(time_zone.clone()),
        );
    }
    if let Some(group_expansion_max) = command.group_expansion_max {
        body.insert(
            "groupExpansionMax".into(),
            serde_json::Value::Number(group_expansion_max.into()),
        );
    }
    if let Some(calendar_expansion_max) = command.calendar_expansion_max {
        body.insert(
            "calendarExpansionMax".into(),
            serde_json::Value::Number(calendar_expansion_max.into()),
        );
    }

    let mut options = FreeBusyOptions::new(serde_json::Value::Object(body));
    if let Some(base_url) = base_url {
        options = options.with_base_url(base_url);
    }
    Ok(options)
}

fn calendar_event_resource_key(calendar_id: &str, event_id: &str) -> String {
    resource_key("calendar", &format!("{calendar_id}/{event_id}"))
}

fn freebusy_resource_key(calendars: &[String]) -> String {
    resource_key("calendar-freebusy", &calendars.join(","))
}

impl From<CalendarSendUpdates> for SendUpdates {
    fn from(value: CalendarSendUpdates) -> Self {
        match value {
            CalendarSendUpdates::All => SendUpdates::All,
            CalendarSendUpdates::ExternalOnly => SendUpdates::ExternalOnly,
            CalendarSendUpdates::None => SendUpdates::None,
        }
    }
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

fn build_calendar_request_body(
    summary: String,
    description: Option<String>,
    location: Option<String>,
    time_zone: Option<String>,
) -> serde_json::Value {
    let mut body = serde_json::Map::new();
    body.insert("summary".into(), serde_json::Value::String(summary));
    if let Some(description) = description {
        body.insert("description".into(), serde_json::Value::String(description));
    }
    if let Some(location) = location {
        body.insert("location".into(), serde_json::Value::String(location));
    }
    if let Some(time_zone) = time_zone {
        body.insert("timeZone".into(), serde_json::Value::String(time_zone));
    }
    serde_json::Value::Object(body)
}

fn build_calendar_patch_request_body(
    summary: Option<String>,
    description: Option<String>,
    location: Option<String>,
    time_zone: Option<String>,
) -> Result<serde_json::Value> {
    let mut body = serde_json::Map::new();
    if let Some(summary) = summary {
        body.insert("summary".into(), serde_json::Value::String(summary));
    }
    if let Some(description) = description {
        body.insert("description".into(), serde_json::Value::String(description));
    }
    if let Some(location) = location {
        body.insert("location".into(), serde_json::Value::String(location));
    }
    if let Some(time_zone) = time_zone {
        body.insert("timeZone".into(), serde_json::Value::String(time_zone));
    }
    anyhow::ensure!(
        !body.is_empty(),
        "at least one calendar metadata flag is required"
    );
    Ok(serde_json::Value::Object(body))
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
    recurrence: Vec<String>,
    reminders: Vec<String>,
    no_reminders: bool,
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
    if !recurrence.is_empty() {
        body.insert(
            "recurrence".into(),
            serde_json::Value::Array(
                recurrence
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    if no_reminders || !reminders.is_empty() {
        body.insert("reminders".into(), reminder_body(reminders, no_reminders)?);
    }

    Ok(serde_json::Value::Object(body))
}

#[allow(clippy::too_many_arguments)]
fn build_event_patch_body(
    summary: Option<String>,
    start: Option<String>,
    end: Option<String>,
    time_zone: Option<String>,
    all_day: bool,
    location: Option<String>,
    description: Option<String>,
    attendees: Vec<String>,
    recurrence: Vec<String>,
    reminders: Vec<String>,
    no_reminders: bool,
) -> Result<serde_json::Value> {
    if all_day && start.is_none() && end.is_none() {
        anyhow::bail!("--all-day requires --start or --end when patching");
    }
    if time_zone.is_some() && start.is_none() && end.is_none() {
        anyhow::bail!("--time-zone requires --start or --end when patching");
    }

    let mut body = serde_json::Map::new();
    if let Some(summary) = summary {
        body.insert("summary".into(), serde_json::Value::String(summary));
    }
    if let Some(start) = start {
        body.insert(
            "start".into(),
            event_time_body("start", start, time_zone.as_deref(), all_day)?,
        );
    }
    if let Some(end) = end {
        body.insert(
            "end".into(),
            event_time_body("end", end, time_zone.as_deref(), all_day)?,
        );
    }
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
    if !recurrence.is_empty() {
        body.insert(
            "recurrence".into(),
            serde_json::Value::Array(
                recurrence
                    .into_iter()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    if no_reminders || !reminders.is_empty() {
        body.insert("reminders".into(), reminder_body(reminders, no_reminders)?);
    }
    if body.is_empty() {
        anyhow::bail!("patch requires --event or at least one event field flag");
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

fn reminder_body(reminders: Vec<String>, no_reminders: bool) -> Result<serde_json::Value> {
    let overrides = reminders
        .into_iter()
        .map(parse_reminder)
        .collect::<Result<Vec<_>>>()?;
    if no_reminders {
        return Ok(serde_json::json!({ "useDefault": false }));
    }
    Ok(serde_json::json!({
        "useDefault": false,
        "overrides": overrides
    }))
}

fn parse_reminder(reminder: String) -> Result<serde_json::Value> {
    let (method, minutes) = reminder
        .split_once(':')
        .with_context(|| format!("invalid --reminder {reminder:?}; expected METHOD:MINUTES"))?;
    match method {
        "popup" | "email" => {}
        _ => anyhow::bail!("invalid --reminder method {method:?}; expected popup or email"),
    }
    let minutes: u32 = minutes
        .parse()
        .with_context(|| format!("invalid --reminder minutes in {reminder:?}"))?;
    Ok(serde_json::json!({
        "method": method,
        "minutes": minutes
    }))
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

fn write_acl_table(out: &mut impl Write, rules: &[serde_json::Value]) -> Result<()> {
    writeln!(out, "SCOPE TYPE\tSCOPE VALUE\tROLE\tRULE ID").context("failed to write output")?;
    for rule in rules {
        writeln!(
            out,
            "{}\t{}\t{}\t{}",
            nested_string_field(rule, "scope", "type"),
            nested_string_field(rule, "scope", "value"),
            string_field(rule, "role"),
            string_field(rule, "id"),
        )
        .context("failed to write output")?;
    }
    Ok(())
}

fn write_acl_rule_table(out: &mut impl Write, rule: &serde_json::Value) -> Result<()> {
    write_acl_table(out, std::slice::from_ref(rule))
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

fn write_freebusy_table(out: &mut impl Write, response: &serde_json::Value) -> Result<()> {
    writeln!(out, "CALENDAR ID\tSTART\tEND").context("failed to write output")?;
    let Some(calendars) = response
        .get("calendars")
        .and_then(serde_json::Value::as_object)
    else {
        return Ok(());
    };

    for (calendar_id, calendar) in calendars {
        let Some(busy) = calendar.get("busy").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for slot in busy {
            writeln!(
                out,
                "{}\t{}\t{}",
                calendar_id.replace('\t', " ").replace('\n', " "),
                string_field(slot, "start"),
                string_field(slot, "end"),
            )
            .context("failed to write output")?;
        }
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

fn nested_string_field(value: &serde_json::Value, object_field: &str, field: &str) -> String {
    value
        .get(object_field)
        .and_then(|object| object.get(field))
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
