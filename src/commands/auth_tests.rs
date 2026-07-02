use std::path::{Path, PathBuf};

use crate::auth::account::TokenSaveOutcome;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType};
use crate::auth::error::AuthError;
use crate::auth::testing::MemoryStore;

use super::auth::{
    add_account_to_config, build_oauth_app_secrets, perform_device_login, run_mappings_clear_to,
    run_mappings_list_to, run_setup_to, write_login_completion_to, SETUP_GUIDE,
};
use crate::auth::state::{
    load_runtime_state_from_path, resource_key, save_runtime_state_to_path, RuntimeState,
};

struct RuntimeStateFixture {
    _temp_dir: tempfile::TempDir,
    path: PathBuf,
}

impl RuntimeStateFixture {
    fn with_mappings(mappings: &[(&str, &str, &str)]) -> Self {
        let fixture = Self::missing("state.toml");
        let mut state = RuntimeState::default();

        for (surface, resource_id, account) in mappings {
            state.set_resource_account(resource_key(surface, resource_id), *account);
        }

        save_runtime_state_to_path(&state, &fixture.path).unwrap();
        fixture
    }

    fn missing(file_name: &str) -> Self {
        let temp_dir = tempfile::tempdir().unwrap();
        let path = temp_dir.path().join(file_name);
        Self {
            _temp_dir: temp_dir,
            path,
        }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

#[test]
fn setup_guide_describes_direct_client_id_and_secret_entry() {
    let mut out: Vec<u8> = Vec::new();
    assert!(SETUP_GUIDE.contains("1."), "guide is missing step 1");
    assert!(SETUP_GUIDE.contains("8."), "guide is missing step 8");
    assert!(
        SETUP_GUIDE.contains("client ID and client secret"),
        "guide is missing direct entry hint"
    );
    assert!(
        SETUP_GUIDE.contains("Desktop app"),
        "guide is missing Desktop app hint"
    );
    assert!(
        SETUP_GUIDE.contains("console.cloud.google.com"),
        "guide is missing GCP Console URL"
    );

    let result = run_setup_to(
        Some("/nonexistent/client_secret.json".into()),
        None,
        &mut out,
    );
    assert!(result.is_err(), "expected error for nonexistent file");
    assert!(
        out.is_empty(),
        "guide must not be printed when --client-secret-file is given"
    );
}

#[test]
fn build_oauth_app_secrets_trims_values() {
    let secrets = build_oauth_app_secrets("  id123  ".into(), "  sec456  ".into()).unwrap();

    assert_eq!(secrets.client_id, "id123");
    assert_eq!(secrets.client_secret, "sec456");
    assert_eq!(secrets.app_type, OAuthAppType::Desktop);
}

#[test]
fn device_login_rejects_non_device_oauth_app_before_network_request() {
    let app = OAuthAppConfig {
        client_id: "client-123".into(),
        client_secret: "secret-456".into(),
        app_type: OAuthAppType::Desktop,
    };
    let store = MemoryStore::default();

    let err = perform_device_login(&app, &store).unwrap_err();

    let msg = err.to_string();
    assert!(msg.contains("TVs and Limited Input devices"));
    assert!(msg.contains("--app-type device"));
}

#[test]
fn build_oauth_app_secrets_rejects_blank_client_id() {
    let err = build_oauth_app_secrets("  ".into(), "sec456".into()).unwrap_err();

    assert!(
        matches!(&err.downcast_ref::<AuthError>(), Some(AuthError::OAuthAppMissingField { field }) if field == "client_id")
    );
}

#[test]
fn build_oauth_app_secrets_rejects_blank_client_secret() {
    let err = build_oauth_app_secrets("id123".into(), "  ".into()).unwrap_err();

    assert!(
        matches!(&err.downcast_ref::<AuthError>(), Some(AuthError::OAuthAppMissingField { field }) if field == "client_secret")
    );
}

#[test]
fn add_account_dedups_repeated_logins() {
    let mut config = Config::default();
    add_account_to_config(&mut config, "alice@example.com");
    add_account_to_config(&mut config, "alice@example.com");
    assert_eq!(config.accounts, vec!["alice@example.com".to_string()]);
}

#[test]
fn add_account_appends_a_second_distinct_email() {
    let mut config = Config::default();
    add_account_to_config(&mut config, "alice@example.com");
    add_account_to_config(&mut config, "bob@example.com");
    assert_eq!(
        config.accounts,
        vec![
            "alice@example.com".to_string(),
            "bob@example.com".to_string()
        ]
    );
}

#[test]
fn first_login_becomes_the_active_account() {
    let mut config = Config::default();
    add_account_to_config(&mut config, "alice@example.com");
    assert_eq!(
        config.settings.unwrap().active_account.as_deref(),
        Some("alice@example.com")
    );
}

#[test]
fn second_login_does_not_displace_active_account() {
    let mut config = Config::default();
    add_account_to_config(&mut config, "alice@example.com");
    add_account_to_config(&mut config, "bob@example.com");
    assert_eq!(
        config.settings.unwrap().active_account.as_deref(),
        Some("alice@example.com")
    );
}

#[test]
fn login_warns_when_keychain_prompt_free_access_is_not_guaranteed() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();

    write_login_completion_to(
        "alice@example.com",
        &TokenSaveOutcome::prompt_free_access_not_guaranteed(),
        &mut out,
        &mut err,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Authorized as alice@example.com\n"
    );
    let warning = String::from_utf8(err).unwrap();
    assert!(warning.contains("Keychain Access Prompts"));
    assert!(warning.contains("Google browser consent prompts"));
    assert!(warning.contains("goog auth login"));
    assert!(!warning.contains("access-abc"));
    assert!(!warning.contains("refresh-def"));
}

#[test]
fn login_does_not_warn_when_keychain_prompt_free_access_is_guaranteed() {
    let mut out: Vec<u8> = Vec::new();
    let mut err: Vec<u8> = Vec::new();

    write_login_completion_to(
        "alice@example.com",
        &TokenSaveOutcome::prompt_free_access_guaranteed(),
        &mut out,
        &mut err,
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Authorized as alice@example.com\n"
    );
    assert!(String::from_utf8(err).unwrap().is_empty());
}

#[test]
fn mappings_list_renders_resource_account_mappings() {
    let state = RuntimeStateFixture::with_mappings(&[
        ("docs", "document-123", "alice@example.com"),
        ("sheets", "spreadsheet-456", "bob@example.com"),
    ]);
    let mut out = Vec::new();

    run_mappings_list_to(false, &mut out, Some(state.path())).unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "SURFACE  RESOURCE ID      ACCOUNT          \n\
docs     document-123     alice@example.com\n\
sheets   spreadsheet-456  bob@example.com  \n"
    );
}

#[test]
fn mappings_list_renders_json_resource_account_mappings() {
    let state =
        RuntimeStateFixture::with_mappings(&[("docs", "document-123", "alice@example.com")]);
    let mut out = Vec::new();

    run_mappings_list_to(true, &mut out, Some(state.path())).unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "{\"surface\":\"docs\",\"resource_id\":\"document-123\",\"account\":\"alice@example.com\",\"resource_key\":\"docs:document-123\"}\n"
    );
}

#[test]
fn mappings_clear_filters_by_surface_and_resource_id() {
    let fixture = RuntimeStateFixture::with_mappings(&[
        ("docs", "document-123", "alice@example.com"),
        ("docs", "document-456", "bob@example.com"),
        ("sheets", "document-123", "carol@example.com"),
    ]);
    let mut out = Vec::new();

    run_mappings_clear_to(
        Some("docs"),
        Some("document-123"),
        &mut out,
        Some(fixture.path()),
    )
    .unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Cleared 1 Resource Account Mapping(s).\n"
    );
    let state = load_runtime_state_from_path(fixture.path()).unwrap();
    assert_eq!(
        state.account_for_resource(&resource_key("docs", "document-456")),
        Some("bob@example.com")
    );
    assert_eq!(
        state.account_for_resource(&resource_key("sheets", "document-123")),
        Some("carol@example.com")
    );
    assert_eq!(
        state.account_for_resource(&resource_key("docs", "document-123")),
        None
    );
}

#[test]
fn mappings_clear_rejects_partial_filter() {
    let fixture =
        RuntimeStateFixture::with_mappings(&[("docs", "document-123", "alice@example.com")]);
    let mut out = Vec::new();

    let err =
        run_mappings_clear_to(Some("docs"), None, &mut out, Some(fixture.path())).unwrap_err();

    let message = err.to_string();
    assert!(message.contains("--surface"));
    assert!(message.contains("--resource-id"));
    assert!(out.is_empty());
    assert_eq!(
        load_runtime_state_from_path(fixture.path())
            .unwrap()
            .account_for_resource(&resource_key("docs", "document-123")),
        Some("alice@example.com")
    );
}

#[test]
fn mappings_clear_without_filters_clears_all_mappings() {
    let fixture = RuntimeStateFixture::with_mappings(&[
        ("docs", "document-123", "alice@example.com"),
        ("sheets", "spreadsheet-456", "bob@example.com"),
    ]);
    let mut out = Vec::new();

    run_mappings_clear_to(None, None, &mut out, Some(fixture.path())).unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Cleared 2 Resource Account Mapping(s).\n"
    );
    assert!(load_runtime_state_from_path(fixture.path())
        .unwrap()
        .resource_account_mappings
        .is_empty());
}

#[test]
fn mappings_list_handles_missing_runtime_state_file() {
    let state = RuntimeStateFixture::missing("missing-state.toml");
    let mut out = Vec::new();

    run_mappings_list_to(false, &mut out, Some(state.path())).unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "No Resource Account Mappings remembered.\n"
    );
}

#[test]
fn mappings_clear_handles_missing_runtime_state_file() {
    let state = RuntimeStateFixture::missing("missing-state.toml");
    let mut out = Vec::new();

    run_mappings_clear_to(None, None, &mut out, Some(state.path())).unwrap();

    assert_eq!(
        String::from_utf8(out).unwrap(),
        "Cleared 0 Resource Account Mapping(s).\n"
    );
    assert!(load_runtime_state_from_path(state.path())
        .unwrap()
        .resource_account_mappings
        .is_empty());
}

#[test]
fn mappings_list_reports_malformed_runtime_state() {
    let state = RuntimeStateFixture::missing("state.toml");
    std::fs::write(state.path(), "[resource_account_mappings\n").unwrap();
    let mut out = Vec::new();

    let err = run_mappings_list_to(false, &mut out, Some(state.path())).unwrap_err();

    let message = format!("{err:#}");
    assert!(message.contains("failed to load runtime state"));
    assert!(message.contains("config file is malformed"));
    assert!(out.is_empty());
}
