use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;

use super::config::{resolve_account, Config};
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

        if account_override.is_some() {
            let account = resolve_account(config, account_override)
                .map_err(E::from)?
                .expect("explicit account resolution returns an account");
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

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;
    use crate::auth::config::SettingsConfig;

    #[derive(Debug)]
    enum TestError {
        Auth,
        Target,
        Other,
    }

    impl From<AuthError> for TestError {
        fn from(_: AuthError) -> Self {
            Self::Auth
        }
    }

    fn config() -> Config {
        Config {
            accounts: vec![
                "active@example.com".to_string(),
                "mapped@example.com".to_string(),
                "other@example.com".to_string(),
            ],
            settings: Some(SettingsConfig {
                active_account: Some("active@example.com".to_string()),
                output: None,
            }),
            oauth_app: None,
        }
    }

    fn state_path() -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("state.toml");
        (temp, path)
    }

    fn read_mapping(path: &Path, key: &str) -> Option<String> {
        load_runtime_state_from_path(path)
            .unwrap()
            .account_for_resource(key)
            .map(str::to_string)
    }

    #[test]
    fn candidates_start_with_mapping_then_active_then_remaining_accounts() {
        let mut state = RuntimeState::default();
        state.set_resource_account("docs:123", "mapped@example.com");

        assert_eq!(
            unified_access_candidates(&config(), &state, "docs:123"),
            vec![
                "mapped@example.com".to_string(),
                "active@example.com".to_string(),
                "other@example.com".to_string(),
            ]
        );
    }

    #[tokio::test]
    async fn run_records_successful_mapping_after_default_fallback() {
        let (_temp, path) = state_path();
        let attempts = RefCell::new(Vec::new());
        let outcomes = RefCell::new(vec![Err(TestError::Target), Ok("ok")]);

        let result = UnifiedAccess::run(
            &config(),
            None,
            "docs:123",
            Some(&path),
            |account| -> AccessFuture<'_, &'static str, TestError> {
                attempts.borrow_mut().push(account);
                let outcome = outcomes.borrow_mut().remove(0);
                Box::pin(async move { outcome })
            },
            |err| matches!(err, TestError::Target),
        )
        .await
        .unwrap();

        assert_eq!(result, "ok");
        assert_eq!(
            attempts.into_inner(),
            vec![
                "active@example.com".to_string(),
                "mapped@example.com".to_string(),
            ]
        );
        assert_eq!(
            read_mapping(&path, "docs:123"),
            Some("mapped@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn run_records_successful_mapping_for_explicit_account_without_fallback() {
        let (_temp, path) = state_path();
        let attempts = RefCell::new(Vec::new());

        let result = UnifiedAccess::run(
            &config(),
            Some("other@example.com"),
            "docs:123",
            Some(&path),
            |account| -> AccessFuture<'_, &'static str, TestError> {
                attempts.borrow_mut().push(account);
                Box::pin(async { Ok("ok") })
            },
            |err| matches!(err, TestError::Target),
        )
        .await
        .unwrap();

        assert_eq!(result, "ok");
        assert_eq!(attempts.into_inner(), vec!["other@example.com".to_string()]);
        assert_eq!(
            read_mapping(&path, "docs:123"),
            Some("other@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn run_repairs_stale_mapping_after_target_access_failure() {
        let (_temp, path) = state_path();
        let mut state = RuntimeState::default();
        state.set_resource_account("docs:123", "mapped@example.com");
        save_runtime_state_to_path(&state, &path).unwrap();

        let attempts = RefCell::new(Vec::new());
        let outcomes = RefCell::new(vec![Err(TestError::Target), Ok("ok")]);

        UnifiedAccess::run(
            &config(),
            None,
            "docs:123",
            Some(&path),
            |account| -> AccessFuture<'_, &'static str, TestError> {
                attempts.borrow_mut().push(account);
                let outcome = outcomes.borrow_mut().remove(0);
                Box::pin(async move { outcome })
            },
            |err| matches!(err, TestError::Target),
        )
        .await
        .unwrap();

        assert_eq!(
            attempts.into_inner(),
            vec![
                "mapped@example.com".to_string(),
                "active@example.com".to_string(),
            ]
        );
        assert_eq!(
            read_mapping(&path, "docs:123"),
            Some("active@example.com".to_string())
        );
    }

    #[tokio::test]
    async fn run_stops_on_non_target_failure_without_trying_later_accounts() {
        let (_temp, path) = state_path();
        let attempts = RefCell::new(Vec::new());

        let result = UnifiedAccess::run(
            &config(),
            None,
            "docs:123",
            Some(&path),
            |account| -> AccessFuture<'_, &'static str, TestError> {
                attempts.borrow_mut().push(account);
                Box::pin(async { Err(TestError::Other) })
            },
            |err| matches!(err, TestError::Target),
        )
        .await;

        assert!(matches!(result, Err(TestError::Other)));
        assert_eq!(
            attempts.into_inner(),
            vec!["active@example.com".to_string()]
        );
        assert_eq!(read_mapping(&path, "docs:123"), None);
    }
}
