use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::account::Token;
use super::error::AuthError;

pub const AUTH_STATE_VERSION: u32 = 1;
pub(crate) const TOKEN_FILE_ENV_VAR: &str = "GOOG_TOKEN_FILE";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthState {
    #[serde(default = "default_version")]
    pub version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active_account: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub accounts: Vec<AuthStateAccount>,
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub resource_account_mappings: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthStateAccount {
    pub email: String,
    pub token: Token,
}

pub type RuntimeState = AuthState;

fn default_version() -> u32 {
    AUTH_STATE_VERSION
}

impl Default for AuthState {
    fn default() -> Self {
        Self {
            version: AUTH_STATE_VERSION,
            active_account: None,
            accounts: Vec::new(),
            resource_account_mappings: HashMap::new(),
        }
    }
}

impl AuthState {
    pub fn account_emails(&self) -> Vec<String> {
        self.accounts
            .iter()
            .map(|account| account.email.clone())
            .collect()
    }

    pub fn token_for_account(&self, email: &str) -> Option<&Token> {
        self.accounts
            .iter()
            .find(|account| account.email == email)
            .map(|account| &account.token)
    }

    pub fn save_token_for_account(&mut self, email: &str, token: Token) {
        match self
            .accounts
            .iter_mut()
            .find(|account| account.email == email)
        {
            Some(account) => account.token = token,
            None => self.accounts.push(AuthStateAccount {
                email: email.to_string(),
                token,
            }),
        }

        if self.active_account.is_none() {
            self.active_account = Some(email.to_string());
        }
    }

    pub fn resolve_account_selector(&self, selector: &str) -> Result<String, AuthError> {
        let selector = selector.trim();
        if selector.is_empty() {
            return Err(AuthError::AccountNotFound {
                email: selector.to_string(),
            });
        }

        let selector_key = selector.to_lowercase();
        self.accounts
            .iter()
            .find(|account| account.email.to_lowercase() == selector_key)
            .or_else(|| {
                self.accounts
                    .iter()
                    .find(|account| account.email.to_lowercase().contains(&selector_key))
            })
            .map(|account| account.email.clone())
            .ok_or_else(|| AuthError::AccountNotFound {
                email: selector.to_string(),
            })
    }

    pub fn switch_active_account(&mut self, selector: &str) -> Result<String, AuthError> {
        let email = self.resolve_account_selector(selector)?;
        self.active_account = Some(email.clone());
        Ok(email)
    }

    pub fn account_for_resource(&self, resource_key: &str) -> Option<&str> {
        self.resource_account_mappings
            .get(resource_key)
            .map(String::as_str)
    }

    pub fn set_resource_account(
        &mut self,
        resource_key: impl Into<String>,
        account: impl Into<String>,
    ) {
        self.resource_account_mappings
            .insert(resource_key.into(), account.into());
    }
}

pub fn resource_key(kind: &str, id: &str) -> String {
    format!("{kind}:{id}")
}

pub fn auth_state_path() -> Result<PathBuf, AuthError> {
    let home = dirs::home_dir().ok_or(AuthError::ConfigDirNotFound)?;
    Ok(home.join(".goog").join("auth.json"))
}

pub fn runtime_state_path() -> Result<PathBuf, AuthError> {
    if let Some(path) = std::env::var_os(TOKEN_FILE_ENV_VAR).filter(|path| !path.is_empty()) {
        return Ok(PathBuf::from(path));
    }
    auth_state_path()
}

pub fn load_runtime_state() -> Result<RuntimeState, AuthError> {
    load_runtime_state_from_path(&runtime_state_path()?)
}

pub fn save_runtime_state(state: &RuntimeState) -> Result<(), AuthError> {
    save_runtime_state_to_path(state, &runtime_state_path()?)
}

pub(crate) fn load_runtime_state_from_path(path: &Path) -> Result<RuntimeState, AuthError> {
    if !path.exists() {
        return Ok(RuntimeState::default());
    }
    let contents = std::fs::read_to_string(path).map_err(AuthError::ConfigReadIo)?;
    serde_json::from_str(&contents).map_err(|e| AuthError::ConfigMalformed(e.to_string()))
}

pub(crate) fn save_runtime_state_to_path(
    state: &RuntimeState,
    path: &Path,
) -> Result<(), AuthError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AuthError::ConfigWriteIo)?;
    }
    let contents = serde_json::to_string_pretty(state)
        .map_err(|e| AuthError::ConfigMalformed(e.to_string()))?;
    std::fs::write(path, contents).map_err(AuthError::ConfigWriteIo)?;
    restrict_permissions(path)
}

#[cfg(unix)]
fn restrict_permissions(path: &Path) -> Result<(), AuthError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(AuthError::ConfigWriteIo)
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), AuthError> {
    Ok(())
}
