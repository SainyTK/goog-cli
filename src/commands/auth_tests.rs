use crate::auth::account::TokenSaveOutcome;
use crate::auth::config::{Config, OAuthAppConfig, OAuthAppType};
use crate::auth::error::AuthError;
use crate::auth::testing::MemoryStore;

use super::auth::{
    add_account_to_config, build_oauth_app_secrets, perform_device_login, run_setup_to,
    write_login_completion_to, SETUP_GUIDE,
};

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
