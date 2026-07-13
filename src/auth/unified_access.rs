use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use super::config::Config;
use super::error::AuthError;
use super::state::{
    load_runtime_state, load_runtime_state_from_path, save_runtime_state,
    save_runtime_state_to_path, RuntimeState,
};

pub type AccessFuture<'a, T, E> = Pin<Box<dyn Future<Output = Result<T, E>> + 'a>>;

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

    pub async fn run<'a, T, E, A, C>(
        config: &Config,
        account_override: Option<&str>,
        target_resource_key: &str,
        state_path: Option<&Path>,
        mut attempt: A,
        is_target_access_failure: C,
    ) -> Result<T, E>
    where
        E: From<AuthError>,
        A: FnMut(String) -> AccessFuture<'a, T, E>,
        C: Fn(&E) -> bool,
    {
        let mut access = Self::load(target_resource_key, state_path).map_err(E::from)?;

        if let Some(account_override) = account_override {
            let account = account_override.to_string();
            let result = attempt(account.clone()).await?;
            access.record_success(account).map_err(E::from)?;
            return Ok(result);
        }

        let candidates = access.candidates(config);
        let mut last_target_access_failure = None;

        for account in candidates {
            match attempt(account.clone()).await {
                Ok(result) => {
                    access.record_success(account).map_err(E::from)?;
                    return Ok(result);
                }
                Err(err) if is_target_access_failure(&err) => {
                    last_target_access_failure = Some(err);
                }
                Err(err) => return Err(err),
            }
        }

        Err(last_target_access_failure
            .unwrap_or_else(|| AuthError::ActiveAccountNotConfigured.into()))
    }
}

pub fn unified_access_candidates(
    config: &Config,
    state: &RuntimeState,
    target_resource_key: &str,
) -> Vec<String> {
    let mut candidates = Vec::new();

    if let Some(mapped) = state.account_for_resource(target_resource_key) {
        push_if_configured(config, state, &mut candidates, mapped);
    }

    if let Some(active) = state
        .active_account
        .as_deref()
        .or_else(|| config.active_account())
    {
        push_if_configured(config, state, &mut candidates, active);
    }

    if state.accounts.is_empty() {
        for account in &config.accounts {
            push_candidate(&mut candidates, account);
        }
    } else {
        for account in &state.accounts {
            push_candidate(&mut candidates, &account.email);
        }
    }

    candidates
}

fn push_if_configured(
    config: &Config,
    state: &RuntimeState,
    candidates: &mut Vec<String>,
    account: &str,
) {
    if (state.accounts.is_empty()
        && config
            .accounts
            .iter()
            .any(|configured| configured == account))
        || state
            .accounts
            .iter()
            .any(|configured| configured.email == account)
    {
        push_candidate(candidates, account);
    }
}

fn push_candidate(candidates: &mut Vec<String>, account: &str) {
    if !candidates.iter().any(|candidate| candidate == account) {
        candidates.push(account.to_string());
    }
}
