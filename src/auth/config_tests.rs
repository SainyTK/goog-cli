use std::fs;
use std::path::PathBuf;

use tempfile::TempDir;

use super::config::{
    load_config_from_path, load_config_from_paths, resolve_account, save_config_to_path,
    switch_active_account, Config, OAuthAppConfig, OAuthAppType, SettingsConfig,
};
use super::error::AuthError;

fn write_config_in(dir: &TempDir, contents: &str) -> PathBuf {
    let path = dir.path().join("config.toml");
    fs::write(&path, contents).unwrap();
    path
}

fn config_with_accounts(active_account: Option<&str>, accounts: &[&str]) -> Config {
    Config {
        oauth_app: None,
        settings: active_account.map(|email| SettingsConfig {
            active_account: Some(email.to_string()),
            output: None,
        }),
        accounts: accounts.iter().copied().map(str::to_string).collect(),
    }
}

#[test]
fn round_trips_oauth_app_config() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");

    let config = Config {
        oauth_app: Some(OAuthAppConfig {
            client_id: "my-client-id".into(),
            client_secret: "my-client-secret".into(),
            app_type: OAuthAppType::Desktop,
        }),
        settings: None,
        accounts: Vec::new(),
    };

    let contents = toml::to_string_pretty(&config).unwrap();
    fs::write(&path, &contents).unwrap();

    let loaded: Config = toml::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
    assert_eq!(loaded, config);
}

#[test]
fn parses_config_with_all_fields() {
    let dir = TempDir::new().unwrap();
    write_config_in(
        &dir,
        r#"
[oauth_app]
client_id = "abc123"
client_secret = "secret456"

[settings]
active_account = "user@example.com"
output = "json"
"#,
    );

    let contents = fs::read_to_string(dir.path().join("config.toml")).unwrap();
    let config: Config = toml::from_str(&contents).unwrap();

    let app = config.oauth_app.unwrap();
    assert_eq!(app.client_id, "abc123");
    assert_eq!(app.client_secret, "secret456");
    assert_eq!(app.app_type, OAuthAppType::Unknown);

    let settings = config.settings.unwrap();
    assert_eq!(settings.active_account.as_deref(), Some("user@example.com"));
    assert_eq!(settings.output.as_deref(), Some("json"));
}

#[test]
fn returns_default_config_when_file_missing() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("config.toml");
    assert!(!path.exists());

    let config = load_config_from_path(&path).unwrap();
    assert!(config.oauth_app.is_none());
    assert!(config.settings.is_none());
}

#[test]
fn copies_oauth_setup_forward_from_old_config_path_without_account_state() {
    let dir = TempDir::new().unwrap();
    let old_path = dir.path().join("old").join("config.toml");
    let new_path = dir.path().join("new").join("config.toml");
    let old_config = config_with_accounts(Some("alice@example.com"), &["alice@example.com"]);
    let mut old_config = old_config;
    old_config.oauth_app = Some(OAuthAppConfig {
        client_id: "client-id".into(),
        client_secret: "client-secret".into(),
        app_type: OAuthAppType::Desktop,
    });
    save_config_to_path(&old_config, &old_path).unwrap();

    let loaded = load_config_from_paths(&new_path, &old_path).unwrap();
    let copied = load_config_from_path(&new_path).unwrap();

    assert_eq!(loaded.oauth_app, old_config.oauth_app);
    assert_eq!(copied.oauth_app, old_config.oauth_app);
    assert!(copied.accounts.is_empty());
    assert!(copied.active_account().is_none());
}

#[test]
fn serialises_only_present_fields() {
    let config = Config {
        oauth_app: Some(OAuthAppConfig {
            client_id: "id".into(),
            client_secret: "sec".into(),
            app_type: OAuthAppType::Device,
        }),
        settings: None,
        accounts: Vec::new(),
    };
    let s = toml::to_string_pretty(&config).unwrap();
    assert!(s.contains("client_id"));
    assert!(s.contains("app_type = \"device\""));
    assert!(!s.contains("settings"));
}

#[test]
fn defaults_missing_oauth_app_type_to_unknown() {
    let contents = r#"
[oauth_app]
client_id = "abc123"
client_secret = "secret456"
"#;

    let config: Config = toml::from_str(contents).unwrap();

    assert_eq!(config.oauth_app.unwrap().app_type, OAuthAppType::Unknown);
}

#[test]
fn switch_active_account_updates_existing_account() {
    let mut config = config_with_accounts(
        Some("alice@example.com"),
        &["alice@example.com", "bob@example.com"],
    );

    let email = switch_active_account(&mut config, "bob@example.com").unwrap();

    assert_eq!(email, "bob@example.com");
    assert_eq!(config.active_account(), Some("bob@example.com"));
}

#[test]
fn switch_active_account_accepts_partial_selector() {
    let mut config = config_with_accounts(
        Some("alice@example.com"),
        &["alice@example.com", "bob@example.com"],
    );

    let email = switch_active_account(&mut config, "bo").unwrap();

    assert_eq!(email, "bob@example.com");
    assert_eq!(config.active_account(), Some("bob@example.com"));
}

#[test]
fn switch_active_account_prefers_exact_match_before_partial_match() {
    let mut config = config_with_accounts(
        None,
        &["sales@example.com", "al@example.com", "alice@example.com"],
    );

    let email = switch_active_account(&mut config, "al@example.com").unwrap();

    assert_eq!(email, "al@example.com");
    assert_eq!(config.active_account(), Some("al@example.com"));
}

#[test]
fn switch_active_account_uses_first_partial_match_in_account_order() {
    let mut config = config_with_accounts(
        None,
        &["alice@example.com", "alina@example.com", "bob@example.com"],
    );

    let email = switch_active_account(&mut config, "ali").unwrap();

    assert_eq!(email, "alice@example.com");
    assert_eq!(config.active_account(), Some("alice@example.com"));
}

#[test]
fn switch_active_account_matches_case_insensitively() {
    let mut config = config_with_accounts(None, &["alice@example.com"]);

    let email = switch_active_account(&mut config, "ALI").unwrap();

    assert_eq!(email, "alice@example.com");
    assert_eq!(config.active_account(), Some("alice@example.com"));
}

#[test]
fn switch_active_account_trims_selector() {
    let mut config = config_with_accounts(None, &["alice@example.com"]);

    let email = switch_active_account(&mut config, "  ali  ").unwrap();

    assert_eq!(email, "alice@example.com");
    assert_eq!(config.active_account(), Some("alice@example.com"));
}

#[test]
fn switch_active_account_rejects_blank_selector() {
    let mut config = config_with_accounts(None, &["alice@example.com"]);

    let err = switch_active_account(&mut config, "   ").unwrap_err();

    assert!(matches!(
        err,
        AuthError::AccountNotFound { email } if email.is_empty()
    ));
    assert!(config.settings.is_none());
}

#[test]
fn switch_active_account_rejects_unknown_account() {
    let mut config = config_with_accounts(None, &["alice@example.com"]);

    let err = switch_active_account(&mut config, "bob@example.com").unwrap_err();

    assert!(matches!(
        err,
        AuthError::AccountNotFound { email } if email == "bob@example.com"
    ));
    assert!(config.settings.is_none());
}

#[test]
fn account_override_resolves_to_logged_in_account() {
    let config = config_with_accounts(
        Some("alice@example.com"),
        &["alice@example.com", "bob@example.com"],
    );

    let account = resolve_account(&config, Some("bob@example.com")).unwrap();

    assert_eq!(account.as_deref(), Some("bob@example.com"));
}

#[test]
fn account_override_does_not_change_active_account() {
    let config = config_with_accounts(
        Some("alice@example.com"),
        &["alice@example.com", "bob@example.com"],
    );

    let _ = resolve_account(&config, Some("bob@example.com")).unwrap();

    assert_eq!(config.active_account(), Some("alice@example.com"));
}

#[test]
fn account_override_rejects_unknown_account() {
    let config = config_with_accounts(None, &["alice@example.com"]);

    let err = resolve_account(&config, Some("bob@example.com")).unwrap_err();

    assert!(matches!(
        err,
        AuthError::AccountNotFound { email } if email == "bob@example.com"
    ));
}

#[test]
fn account_resolution_falls_back_to_active_account() {
    let config = config_with_accounts(Some("alice@example.com"), &["alice@example.com"]);

    let account = resolve_account(&config, None).unwrap();

    assert_eq!(account.as_deref(), Some("alice@example.com"));
}

#[test]
fn save_config_creates_parent_dirs_and_strips_account_state() {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("nested").join("config.toml");
    let config = config_with_accounts(
        Some("alice@example.com"),
        &["alice@example.com", "bob@example.com"],
    );

    save_config_to_path(&config, &path).unwrap();
    let loaded = load_config_from_path(&path).unwrap();

    assert_eq!(loaded.oauth_app, config.oauth_app);
    assert!(loaded.accounts.is_empty());
    assert!(loaded.active_account().is_none());
}
