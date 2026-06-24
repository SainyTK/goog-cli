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
        Some(email) => ensure_logged_in(config, email).map(|()| Some(email.to_string())),
        None => Ok(config
            .settings
            .as_ref()
            .and_then(|settings| settings.active_account.clone())),
    }
}

pub fn switch_active_account(config: &mut Config, email: &str) -> Result<(), AuthError> {
    ensure_logged_in(config, email)?;
    let settings = config.settings.get_or_insert_with(SettingsConfig::default);
    settings.active_account = Some(email.to_string());
    Ok(())
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
    let contents = toml::to_string_pretty(config)
        .map_err(|e| AuthError::ConfigMalformed(e.to_string()))?;
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
        let mut config = Config {
            oauth_app: None,
            settings: Some(SettingsConfig {
                active_account: Some("alice@example.com".into()),
                output: None,
            }),
            accounts: vec!["alice@example.com".into(), "bob@example.com".into()],
        };

        switch_active_account(&mut config, "bob@example.com").unwrap();

        assert_eq!(
            config.settings.unwrap().active_account.as_deref(),
            Some("bob@example.com")
        );
    }

    #[test]
    fn switch_active_account_rejects_unknown_account() {
        let mut config = Config {
            oauth_app: None,
            settings: None,
            accounts: vec!["alice@example.com".into()],
        };

        let err = switch_active_account(&mut config, "bob@example.com").unwrap_err();

        assert!(matches!(
            err,
            AuthError::AccountNotFound { email } if email == "bob@example.com"
        ));
        assert!(config.settings.is_none());
    }

    #[test]
    fn account_override_resolves_to_logged_in_account() {
        let config = Config {
            oauth_app: None,
            settings: Some(SettingsConfig {
                active_account: Some("alice@example.com".into()),
                output: None,
            }),
            accounts: vec!["alice@example.com".into(), "bob@example.com".into()],
        };

        let account = resolve_account(&config, Some("bob@example.com")).unwrap();

        assert_eq!(account.as_deref(), Some("bob@example.com"));
    }

    #[test]
    fn account_override_does_not_change_active_account() {
        let config = Config {
            oauth_app: None,
            settings: Some(SettingsConfig {
                active_account: Some("alice@example.com".into()),
                output: None,
            }),
            accounts: vec!["alice@example.com".into(), "bob@example.com".into()],
        };

        let _ = resolve_account(&config, Some("bob@example.com")).unwrap();

        assert_eq!(
            config.settings.unwrap().active_account.as_deref(),
            Some("alice@example.com")
        );
    }

    #[test]
    fn account_override_rejects_unknown_account() {
        let config = Config {
            oauth_app: None,
            settings: None,
            accounts: vec!["alice@example.com".into()],
        };

        let err = resolve_account(&config, Some("bob@example.com")).unwrap_err();

        assert!(matches!(
            err,
            AuthError::AccountNotFound { email } if email == "bob@example.com"
        ));
    }

    #[test]
    fn account_resolution_falls_back_to_active_account() {
        let config = Config {
            oauth_app: None,
            settings: Some(SettingsConfig {
                active_account: Some("alice@example.com".into()),
                output: None,
            }),
            accounts: vec!["alice@example.com".into()],
        };

        let account = resolve_account(&config, None).unwrap();

        assert_eq!(account.as_deref(), Some("alice@example.com"));
    }

    #[test]
    fn save_config_creates_parent_dirs_and_round_trips_accounts() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested").join("config.toml");
        let config = Config {
            oauth_app: None,
            settings: Some(SettingsConfig {
                active_account: Some("alice@example.com".into()),
                output: None,
            }),
            accounts: vec!["alice@example.com".into(), "bob@example.com".into()],
        };

        save_config_to_path(&config, &path).unwrap();
        let loaded = load_config_from_path(&path).unwrap();

        assert_eq!(loaded, config);
    }
}
