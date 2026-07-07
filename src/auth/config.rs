use std::path::PathBuf;

use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use super::error::AuthError;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub oauth_app: Option<OAuthAppConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settings: Option<SettingsConfig>,
    #[serde(default, skip_serializing)]
    #[allow(dead_code)]
    pub accounts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OAuthAppConfig {
    pub client_id: String,
    pub client_secret: String,
    #[serde(default)]
    pub app_type: OAuthAppType,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum OAuthAppType {
    Desktop,
    Web,
    Device,
    #[default]
    #[value(skip)]
    Unknown,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct SettingsConfig {
    #[serde(default, skip_serializing)]
    #[allow(dead_code)]
    pub active_account: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    let home = dirs::home_dir().ok_or(AuthError::ConfigDirNotFound)?;
    Ok(home.join(".goog").join("config.toml"))
}

fn old_config_path() -> Result<PathBuf, AuthError> {
    let dir = dirs::config_dir().ok_or(AuthError::ConfigDirNotFound)?;
    Ok(dir.join("goog").join("config.toml"))
}

pub fn load_config() -> Result<Config, AuthError> {
    let path = config_path()?;
    let old_path = old_config_path()?;
    load_config_from_paths(&path, &old_path)
}

pub(super) fn load_config_from_paths(
    path: &std::path::Path,
    old_path: &std::path::Path,
) -> Result<Config, AuthError> {
    if path.exists() {
        return load_config_from_path(&path);
    }

    if old_path.exists() {
        let config = load_config_from_path(&old_path)?;
        save_config_to_path(&config, &path)?;
        return Ok(config);
    }

    Ok(Config::default())
}

pub(super) fn load_config_from_path(path: &std::path::Path) -> Result<Config, AuthError> {
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

pub fn resolve_account_selector(config: &Config, selector: &str) -> Result<String, AuthError> {
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

pub(super) fn save_config_to_path(
    config: &Config,
    path: &std::path::Path,
) -> Result<(), AuthError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AuthError::ConfigWriteIo)?;
    }
    let contents =
        toml::to_string_pretty(config).map_err(|e| AuthError::ConfigMalformed(e.to_string()))?;
    std::fs::write(path, contents).map_err(AuthError::ConfigWriteIo)
}
