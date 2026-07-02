use std::path::{Path, PathBuf};

use super::config::Config;
use super::error::AuthError;
use super::state::{
    load_runtime_state, load_runtime_state_from_path, save_runtime_state,
    save_runtime_state_to_path, RuntimeState,
};

pub struct UnifiedAccess {
    state: RuntimeState,
    state_path: Option<PathBuf>,
    target_resource_key: String,
}

impl UnifiedAccess {
    pub fn load(
        target_resource_key: impl Into<String>,
        state_path: Option<&Path>,
    ) -> Result<Self, AuthError> {
        let state = match state_path {
            Some(path) => load_runtime_state_from_path(path),
            None => load_runtime_state(),
        }?;

        Ok(Self {
            state,
            state_path: state_path.map(Path::to_path_buf),
            target_resource_key: target_resource_key.into(),
        })
    }

    pub fn candidates(&self, config: &Config) -> Vec<String> {
        unified_access_candidates(config, &self.state, &self.target_resource_key)
    }

    pub fn record_success(&mut self, account: impl Into<String>) -> Result<(), AuthError> {
        self.state
            .set_resource_account(self.target_resource_key.clone(), account);
        match self.state_path.as_deref() {
            Some(path) => save_runtime_state_to_path(&self.state, path),
            None => save_runtime_state(&self.state),
        }
    }
}

pub fn unified_access_candidates(
    config: &Config,
    state: &RuntimeState,
    target_resource_key: &str,
) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(mapped) = state.account_for_resource(target_resource_key) {
        push_if_configured(config, &mut candidates, mapped);
    }

    if let Some(active) = config.active_account() {
        push_if_configured(config, &mut candidates, active);
    }

    for account in &config.accounts {
        push_candidate(&mut candidates, account);
    }

    candidates
}

fn push_if_configured(config: &Config, candidates: &mut Vec<String>, account: &str) {
    if config
        .accounts
        .iter()
        .any(|configured| configured == account)
    {
        push_candidate(candidates, account);
    }
}

fn push_candidate(candidates: &mut Vec<String>, account: &str) {
    if !candidates.iter().any(|candidate| candidate == account) {
        candidates.push(account.to_string());
    }
}
