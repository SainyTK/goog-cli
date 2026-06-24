use crate::auth::config::Config;
use crate::auth::error::AuthError;

use super::auth::{add_account_to_config, build_oauth_app_secrets, run_setup_to, SETUP_GUIDE};

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

    let result = run_setup_to(Some("/nonexistent/client_secret.json".into()), &mut out);
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
