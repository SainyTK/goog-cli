use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::error::AuthError;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub oauth_app: Option<OAuthAppConfig>,
    pub settings: Option<SettingsConfig>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuthAppConfig {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct SettingsConfig {
    pub active_account: Option<String>,
    pub output: Option<String>,
}

impl Config {
    pub(crate) fn active_account(&self) -> Option<&str> {
        self.settings
            .as_ref()
            .and_then(|settings| settings.active_account.as_deref())
    }
}

pub fn config_path() -> Result<PathBuf, AuthError> {
    let dir = dirs::config_dir().ok_or(AuthError::ConfigDirNotFound)?;
    Ok(dir.join("goog").join("config.toml"))
}

pub fn load_config() -> Result<Config, AuthError> {
    load_config_from_path(&config_path()?)
}

fn load_config_from_path(path: &std::path::Path) -> Result<Config, AuthError> {
    if !path.exists() {
        return Ok(Config::default());
    }
    let contents = std::fs::read_to_string(path).map_err(AuthError::ConfigReadIo)?;
    toml::from_str(&contents).map_err(|e| AuthError::ConfigMalformed(e.to_string()))
}

pub fn resolve_account(
    config: &Config,
    account_override: Option<&str>,
) -> Result<Option<String>, AuthError> {
    match account_override {
        Some(email) => {
            ensure_logged_in(config, email)?;
            Ok(Some(email.to_string()))
        }
        None => Ok(config.active_account().map(str::to_string)),
    }
}

pub fn switch_active_account(config: &mut Config, selector: &str) -> Result<String, AuthError> {
    let email = resolve_account_selector(config, selector)?;
    let settings = config.settings.get_or_insert_with(SettingsConfig::default);
    settings.active_account = Some(email.clone());
    Ok(email)
}

fn resolve_account_selector(config: &Config, selector: &str) -> Result<String, AuthError> {
    let selector = selector.trim();
    if selector.is_empty() {
        return Err(AuthError::AccountNotFound {
            email: selector.to_string(),
        });
    }

    let selector_key = selector.to_lowercase();
    config
        .accounts
        .iter()
        .find(|account| account.to_lowercase() == selector_key)
        .or_else(|| {
            config
                .accounts
                .iter()
                .find(|account| account.to_lowercase().contains(&selector_key))
        })
        .cloned()
        .ok_or_else(|| AuthError::AccountNotFound {
            email: selector.to_string(),
        })
}

fn ensure_logged_in(config: &Config, email: &str) -> Result<(), AuthError> {
    if config.accounts.iter().any(|account| account == email) {
        Ok(())
    } else {
        Err(AuthError::AccountNotFound {
            email: email.to_string(),
        })
    }
}

pub fn save_config(config: &Config) -> Result<(), AuthError> {
    let path = config_path()?;
    save_config_to_path(config, &path)
}

fn save_config_to_path(config: &Config, path: &std::path::Path) -> Result<(), AuthError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AuthError::ConfigWriteIo)?;
    }
    let contents =
        toml::to_string_pretty(config).map_err(|e| AuthError::ConfigMalformed(e.to_string()))?;
    std::fs::write(path, contents).map_err(AuthError::ConfigWriteIo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

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
    fn serialises_only_present_fields() {
        let config = Config {
            oauth_app: Some(OAuthAppConfig {
                client_id: "id".into(),
                client_secret: "sec".into(),
            }),
            settings: None,
            accounts: Vec::new(),
        };
        let s = toml::to_string_pretty(&config).unwrap();
        assert!(s.contains("client_id"));
        assert!(!s.contains("settings"));
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
    fn save_config_creates_parent_dirs_and_round_trips_accounts() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        let config = config_with_accounts(
            Some("alice@example.com"),
            &["alice@example.com", "bob@example.com"],
        );

        save_config_to_path(&config, &path).unwrap();
        let loaded = load_config_from_path(&path).unwrap();

        assert_eq!(loaded, config);
    }
}
