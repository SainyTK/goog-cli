use chrono::{Duration, Utc};
use wiremock::matchers::{body_json, header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use crate::auth::account::{AccountStore, Token};
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType, SettingsConfig};
use crate::auth::state::{
    load_runtime_state_from_path, resource_key, save_runtime_state_to_path, RuntimeState,
};
use crate::auth::testing::MemoryStore;
use crate::calendar::CALENDAR_SCOPE;
use crate::cli::{CalendarCalendarsCommand, CalendarEventsCommand};

use super::calendar::*;

fn test_config() -> Config {
    Config {
        oauth_app: Some(OAuthAppConfig {
            client_id: "client-123".into(),
            client_secret: "secret-456".into(),
            app_type: OAuthAppType::Desktop,
        }),
        settings: Some(SettingsConfig {
            active_account: Some("alice@example.com".into()),
            output: None,
        }),
        accounts: vec!["alice@example.com".into()],
    }
}

fn calendar_token(access_token: &str) -> Token {
    Token {
        access_token: access_token.into(),
        refresh_token: "refresh-123".into(),
        expiry: Utc::now() + Duration::hours(1),
        scopes: vec![CALENDAR_SCOPE.into()],
    }
}

fn calendar_base_url(server: &MockServer) -> String {
    format!("{}/calendar/v3", server.uri())
}

fn write_test_state() -> (tempfile::TempDir, std::path::PathBuf) {
    let state_dir = tempfile::tempdir().unwrap();
    let state_path = state_dir.path().join("auth.json");
    save_runtime_state_to_path(
        &RuntimeState {
            version: crate::auth::state::AUTH_STATE_VERSION,
            active_account: Some("alice@example.com".into()),
            accounts: vec![],
            resource_account_mappings: Default::default(),
        },
        &state_path,
    )
    .unwrap();
    (state_dir, state_path)
}

#[tokio::test]
async fn run_calendars_list_prints_table() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/users/me/calendarList"))
        .and(header("authorization", "Bearer calendar-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                {
                    "id": "primary",
                    "summary": "Primary",
                    "accessRole": "owner",
                    "timeZone": "Asia/Bangkok"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("calendar-access"))
        .unwrap();
    let mut out = Vec::new();

    run_calendars_command_to(
        &test_config(),
        &store,
        None,
        CalendarCalendarsCommand::List {
            limit: Some(1),
            all: false,
            json: false,
        },
        false,
        &mut out,
        Some(&calendar_base_url(&server)),
        None,
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "SUMMARY\tCALENDAR ID\tACCESS ROLE\tTIME ZONE\nPrimary\tprimary\towner\tAsia/Bangkok\n"
    );
}

#[tokio::test]
async fn run_events_list_uses_unified_fallback_and_maps_calendar() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("authorization", "Bearer alice-access"))
        .respond_with(ResponseTemplate::new(403).set_body_string("denied"))
        .expect(1)
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("authorization", "Bearer bob-access"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "items": [
                {
                    "id": "event-123",
                    "summary": "Standup",
                    "start": { "dateTime": "2026-07-09T09:00:00Z" },
                    "end": { "dateTime": "2026-07-09T09:30:00Z" },
                    "status": "confirmed"
                }
            ]
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("alice-access"))
        .unwrap();
    store
        .save_token("bob@example.com", &calendar_token("bob-access"))
        .unwrap();
    let config = Config {
        oauth_app: test_config().oauth_app,
        settings: test_config().settings,
        accounts: vec!["alice@example.com".into(), "bob@example.com".into()],
    };
    let (_state_dir, state_path) = write_test_state();
    let mut input = std::io::empty();
    let mut out = Vec::new();

    run_events_command_to(
        &config,
        &store,
        None,
        CalendarEventsCommand::List {
            calendar_id: "primary".into(),
            limit: Some(1),
            all: false,
            time_min: None,
            time_max: None,
            query: None,
            single_events: false,
            json: false,
        },
        &mut input,
        false,
        &mut out,
        Some(&calendar_base_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "SUMMARY\tEVENT ID\tSTART\tEND\tSTATUS\nStandup\tevent-123\t2026-07-09T09:00:00Z\t2026-07-09T09:30:00Z\tconfirmed\n"
    );
    let state = load_runtime_state_from_path(&state_path).unwrap();
    assert_eq!(
        state.account_for_resource(&resource_key("calendar", "primary")),
        Some("bob@example.com")
    );
}

#[tokio::test]
async fn run_events_create_sends_event_body() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(body_json(serde_json::json!({
            "summary": "goog-e2e-calendar",
            "start": { "dateTime": "2026-07-09T09:00:00Z" },
            "end": { "dateTime": "2026-07-09T09:30:00Z" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-456",
            "summary": "goog-e2e-calendar"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("calendar-access"))
        .unwrap();
    let mut input = br#"{"summary":"goog-e2e-calendar","start":{"dateTime":"2026-07-09T09:00:00Z"},"end":{"dateTime":"2026-07-09T09:30:00Z"}}"#.as_slice();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_events_command_to(
        &test_config(),
        &store,
        None,
        CalendarEventsCommand::Create {
            calendar_id: "primary".into(),
            event: Some("-".into()),
            summary: None,
            start: None,
            end: None,
            time_zone: None,
            all_day: false,
            location: None,
            description: None,
            attendee: vec![],
        },
        &mut input,
        false,
        &mut out,
        Some(&calendar_base_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"id\":\"event-456\",\"summary\":\"goog-e2e-calendar\"}\n"
    );
}

#[tokio::test]
async fn run_events_create_builds_event_body_from_flags() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(body_json(serde_json::json!({
            "summary": "Planning",
            "start": {
                "dateTime": "2026-07-09T09:00:00+07:00",
                "timeZone": "Asia/Bangkok"
            },
            "end": {
                "dateTime": "2026-07-09T09:30:00+07:00",
                "timeZone": "Asia/Bangkok"
            },
            "location": "Office",
            "description": "Weekly planning",
            "attendees": [
                { "email": "teammate@example.com" },
                { "email": "lead@example.com" }
            ]
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-789",
            "summary": "Planning"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("calendar-access"))
        .unwrap();
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_events_command_to(
        &test_config(),
        &store,
        None,
        CalendarEventsCommand::Create {
            calendar_id: "primary".into(),
            event: None,
            summary: Some("Planning".into()),
            start: Some("2026-07-09T09:00:00+07:00".into()),
            end: Some("2026-07-09T09:30:00+07:00".into()),
            time_zone: Some("Asia/Bangkok".into()),
            all_day: false,
            location: Some("Office".into()),
            description: Some("Weekly planning".into()),
            attendee: vec!["teammate@example.com".into(), "lead@example.com".into()],
        },
        &mut input,
        false,
        &mut out,
        Some(&calendar_base_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"id\":\"event-789\",\"summary\":\"Planning\"}\n"
    );
}

#[tokio::test]
async fn run_events_create_builds_all_day_event_body_from_flags() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/primary/events"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(body_json(serde_json::json!({
            "summary": "Out of office",
            "start": { "date": "2026-07-09" },
            "end": { "date": "2026-07-10" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-all-day",
            "summary": "Out of office"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("calendar-access"))
        .unwrap();
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_events_command_to(
        &test_config(),
        &store,
        None,
        CalendarEventsCommand::Create {
            calendar_id: "primary".into(),
            event: None,
            summary: Some("Out of office".into()),
            start: Some("2026-07-09".into()),
            end: Some("2026-07-10".into()),
            time_zone: None,
            all_day: true,
            location: None,
            description: None,
            attendee: vec![],
        },
        &mut input,
        false,
        &mut out,
        Some(&calendar_base_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"id\":\"event-all-day\",\"summary\":\"Out of office\"}\n"
    );
}
