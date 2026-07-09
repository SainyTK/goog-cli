use chrono::{Duration, Utc};
use wiremock::matchers::{body_json, header, method, path, query_param};
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
            recurrence: vec![],
            reminder: vec![],
            no_reminders: false,
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
            ],
            "recurrence": [
                "RRULE:FREQ=WEEKLY;COUNT=4"
            ],
            "reminders": {
                "useDefault": false,
                "overrides": [
                    { "method": "popup", "minutes": 10 },
                    { "method": "email", "minutes": 60 }
                ]
            }
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
            recurrence: vec!["RRULE:FREQ=WEEKLY;COUNT=4".into()],
            reminder: vec!["popup:10".into(), "email:60".into()],
            no_reminders: false,
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
            recurrence: vec![],
            reminder: vec![],
            no_reminders: false,
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

#[tokio::test]
async fn run_events_update_sends_event_body() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/calendar/v3/calendars/primary/events/event-456"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(body_json(serde_json::json!({
            "summary": "goog-e2e-calendar-updated",
            "start": { "dateTime": "2026-07-09T10:00:00Z" },
            "end": { "dateTime": "2026-07-09T10:30:00Z" }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-456",
            "summary": "goog-e2e-calendar-updated"
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("calendar-access"))
        .unwrap();
    let mut input = br#"{"summary":"goog-e2e-calendar-updated","start":{"dateTime":"2026-07-09T10:00:00Z"},"end":{"dateTime":"2026-07-09T10:30:00Z"}}"#.as_slice();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_events_command_to(
        &test_config(),
        &store,
        None,
        CalendarEventsCommand::Update {
            calendar_id: "primary".into(),
            event_id: "event-456".into(),
            event: Some("-".into()),
            summary: None,
            start: None,
            end: None,
            time_zone: None,
            all_day: false,
            location: None,
            description: None,
            attendee: vec![],
            recurrence: vec![],
            reminder: vec![],
            no_reminders: false,
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
        "{\"id\":\"event-456\",\"summary\":\"goog-e2e-calendar-updated\"}\n"
    );
}

#[tokio::test]
async fn run_events_update_builds_event_body_from_flags() {
    let server = MockServer::start().await;
    Mock::given(method("PUT"))
        .and(path("/calendar/v3/calendars/primary/events/event-789"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(body_json(serde_json::json!({
            "summary": "Planning moved",
            "start": {
                "dateTime": "2026-07-09T10:00:00+07:00",
                "timeZone": "Asia/Bangkok"
            },
            "end": {
                "dateTime": "2026-07-09T10:30:00+07:00",
                "timeZone": "Asia/Bangkok"
            },
            "location": "Office",
            "description": "Updated planning",
            "attendees": [
                { "email": "teammate@example.com" }
            ],
            "recurrence": [
                "RRULE:FREQ=DAILY;COUNT=3"
            ],
            "reminders": {
                "useDefault": false
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-789",
            "summary": "Planning moved"
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
        CalendarEventsCommand::Update {
            calendar_id: "primary".into(),
            event_id: "event-789".into(),
            event: None,
            summary: Some("Planning moved".into()),
            start: Some("2026-07-09T10:00:00+07:00".into()),
            end: Some("2026-07-09T10:30:00+07:00".into()),
            time_zone: Some("Asia/Bangkok".into()),
            all_day: false,
            location: Some("Office".into()),
            description: Some("Updated planning".into()),
            attendee: vec!["teammate@example.com".into()],
            recurrence: vec!["RRULE:FREQ=DAILY;COUNT=3".into()],
            reminder: vec![],
            no_reminders: true,
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
        "{\"id\":\"event-789\",\"summary\":\"Planning moved\"}\n"
    );
}

#[tokio::test]
async fn run_events_patch_sends_partial_event_body_from_flags() {
    let server = MockServer::start().await;
    Mock::given(method("PATCH"))
        .and(path("/calendar/v3/calendars/primary/events/event-789"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(body_json(serde_json::json!({
            "summary": "Planning renamed",
            "location": "Office",
            "recurrence": [
                "RRULE:FREQ=MONTHLY;COUNT=2"
            ],
            "reminders": {
                "useDefault": false,
                "overrides": [
                    { "method": "popup", "minutes": 5 }
                ]
            }
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "event-789",
            "summary": "Planning renamed"
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
        CalendarEventsCommand::Patch {
            calendar_id: "primary".into(),
            event_id: "event-789".into(),
            event: None,
            summary: Some("Planning renamed".into()),
            start: None,
            end: None,
            time_zone: None,
            all_day: false,
            location: Some("Office".into()),
            description: None,
            attendee: vec![],
            recurrence: vec!["RRULE:FREQ=MONTHLY;COUNT=2".into()],
            reminder: vec!["popup:5".into()],
            no_reminders: false,
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
        "{\"id\":\"event-789\",\"summary\":\"Planning renamed\"}\n"
    );
}

#[tokio::test]
async fn run_events_patch_rejects_empty_flag_body() {
    let store = MemoryStore::default();
    let mut input = std::io::empty();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    let err = run_events_command_to(
        &test_config(),
        &store,
        None,
        CalendarEventsCommand::Patch {
            calendar_id: "primary".into(),
            event_id: "event-789".into(),
            event: None,
            summary: None,
            start: None,
            end: None,
            time_zone: None,
            all_day: false,
            location: None,
            description: None,
            attendee: vec![],
            recurrence: vec![],
            reminder: vec![],
            no_reminders: false,
        },
        &mut input,
        false,
        &mut out,
        None,
        Some(&state_path),
    )
    .await
    .unwrap_err();

    assert!(err
        .to_string()
        .contains("patch requires --event or at least one event field flag"));
}

#[tokio::test]
async fn run_events_move_posts_destination_and_prints_event() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/primary/events/event-789/move"))
        .and(query_param("destination", "team@example.com"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(header("content-length", "0"))
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
        CalendarEventsCommand::Move {
            source_calendar_id: "primary".into(),
            event_id: "event-789".into(),
            destination: "team@example.com".into(),
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
async fn run_events_quick_add_posts_text_and_send_updates() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/calendars/primary/events/quickAdd"))
        .and(query_param("text", "Lunch with Sam tomorrow at noon"))
        .and(query_param("sendUpdates", "externalOnly"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(header("content-length", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "quick-event-1",
            "summary": "Lunch with Sam"
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
        CalendarEventsCommand::QuickAdd {
            calendar_id: "primary".into(),
            text: "Lunch with Sam tomorrow at noon".into(),
            send_updates: Some(crate::cli::CalendarSendUpdates::ExternalOnly),
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
        "{\"id\":\"quick-event-1\",\"summary\":\"Lunch with Sam\"}\n"
    );
}

#[tokio::test]
async fn run_freebusy_sends_query_body_and_prints_table() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/freeBusy"))
        .and(header("authorization", "Bearer calendar-access"))
        .and(body_json(serde_json::json!({
            "timeMin": "2026-07-09T09:00:00Z",
            "timeMax": "2026-07-09T17:00:00Z",
            "items": [
                { "id": "primary" },
                { "id": "team@example.com" }
            ],
            "timeZone": "Asia/Bangkok",
            "groupExpansionMax": 10,
            "calendarExpansionMax": 20
        })))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "calendars": {
                "primary": {
                    "busy": [
                        {
                            "start": "2026-07-09T10:00:00Z",
                            "end": "2026-07-09T10:30:00Z"
                        }
                    ]
                },
                "team@example.com": {
                    "busy": []
                }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("calendar-access"))
        .unwrap();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_freebusy_command_to(
        &test_config(),
        &store,
        None,
        FreeBusyCommand {
            time_min: "2026-07-09T09:00:00Z".into(),
            time_max: "2026-07-09T17:00:00Z".into(),
            calendars: vec!["primary".into(), "team@example.com".into()],
            time_zone: Some("Asia/Bangkok".into()),
            group_expansion_max: Some(10),
            calendar_expansion_max: Some(20),
            json: false,
        },
        false,
        &mut out,
        Some(&calendar_base_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "CALENDAR ID\tSTART\tEND\nprimary\t2026-07-09T10:00:00Z\t2026-07-09T10:30:00Z\n"
    );
}

#[tokio::test]
async fn run_freebusy_json_emits_raw_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/calendar/v3/freeBusy"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "calendars": {
                "primary": { "busy": [] }
            }
        })))
        .expect(1)
        .mount(&server)
        .await;

    let store = MemoryStore::default();
    store
        .save_token("alice@example.com", &calendar_token("calendar-access"))
        .unwrap();
    let mut out = Vec::new();
    let (_state_dir, state_path) = write_test_state();

    run_freebusy_command_to(
        &test_config(),
        &store,
        None,
        FreeBusyCommand {
            time_min: "2026-07-09T09:00:00Z".into(),
            time_max: "2026-07-09T17:00:00Z".into(),
            calendars: vec!["primary".into()],
            time_zone: None,
            group_expansion_max: None,
            calendar_expansion_max: None,
            json: true,
        },
        false,
        &mut out,
        Some(&calendar_base_url(&server)),
        Some(&state_path),
    )
    .await
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"calendars\":{\"primary\":{\"busy\":[]}}}\n"
    );
}
