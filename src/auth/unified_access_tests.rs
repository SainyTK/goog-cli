use std::cell::RefCell;
use std::path::{Path, PathBuf};

use super::config::{Config, SettingsConfig};
use super::error::AuthError;
use super::state::{load_runtime_state_from_path, save_runtime_state_to_path, RuntimeState};
use super::unified_access::{unified_access_candidates, AccessFuture, UnifiedAccess};

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
