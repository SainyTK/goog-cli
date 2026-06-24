use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::error::AuthError;

#[derive(Debug, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub oauth_app: Option<OAuthAppConfig>,
    pub settings: Option<SettingsConfig>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
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

pub fn save_config(config: &Config) -> Result<(), AuthError> {
    let path = config_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AuthError::ConfigWriteIo)?;
    }
    let contents = toml::to_string_pretty(config)
        .map_err(|e| AuthError::ConfigMalformed(e.to_string()))?;
    std::fs::write(&path, contents).map_err(AuthError::ConfigWriteIo)
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
        };
        let s = toml::to_string_pretty(&config).unwrap();
        assert!(s.contains("client_id"));
        assert!(!s.contains("settings"));
    }
}
