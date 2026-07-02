use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::error::AuthError;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeState {
    #[serde(default, skip_serializing_if = "HashMap::is_empty")]
    pub resource_account_mappings: HashMap<String, String>,
}

impl RuntimeState {
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

pub fn runtime_state_path() -> Result<PathBuf, AuthError> {
    let dir = dirs::config_dir().ok_or(AuthError::ConfigDirNotFound)?;
    Ok(dir.join("goog").join("state.toml"))
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
    toml::from_str(&contents).map_err(|e| AuthError::ConfigMalformed(e.to_string()))
}

pub(crate) fn save_runtime_state_to_path(
    state: &RuntimeState,
    path: &Path,
) -> Result<(), AuthError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(AuthError::ConfigWriteIo)?;
    }
    let contents =
        toml::to_string_pretty(state).map_err(|e| AuthError::ConfigMalformed(e.to_string()))?;
    std::fs::write(path, contents).map_err(AuthError::ConfigWriteIo)
}
