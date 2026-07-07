use super::account::{
    resolve_account_store, AccountStore, AccountStoreImpl, FileAccountStore, Token,
};
use super::state::{AuthState, TOKEN_FILE_ENV_VAR};
use super::testing::MemoryStore;
use chrono::{TimeZone, Utc};

static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn sample_token() -> Token {
    Token {
        access_token: "access-abc".into(),
        refresh_token: "refresh-def".into(),
        expiry: Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap(),
        scopes: vec!["openid".into(), "email".into()],
    }
}

struct EnvGuard {
    name: &'static str,
    original: Option<std::ffi::OsString>,
}

impl EnvGuard {
    fn unset(name: &'static str) -> Self {
        let original = std::env::var_os(name);
        std::env::remove_var(name);
        Self { name, original }
    }

    fn set(name: &'static str, value: &std::path::Path) -> Self {
        let original = std::env::var_os(name);
        std::env::set_var(name, value);
        Self { name, original }
    }
}

impl Drop for EnvGuard {
    fn drop(&mut self) {
        match &self.original {
            Some(value) => std::env::set_var(self.name, value),
            None => std::env::remove_var(self.name),
        }
    }
}

#[test]
fn round_trips_a_token_through_memory_store() {
    let store = MemoryStore::default();
    let token = sample_token();

    store.save_token("alice@example.com", &token).unwrap();
    let loaded = store.load_token("alice@example.com").unwrap();

    assert_eq!(loaded, Some(token));
}

#[test]
fn returns_none_for_unknown_account() {
    let store = MemoryStore::default();
    assert!(store.load_token("ghost@example.com").unwrap().is_none());
}

#[test]
fn tokens_are_namespaced_by_email() {
    let store = MemoryStore::default();
    let mut t1 = sample_token();
    t1.access_token = "first".into();
    let mut t2 = sample_token();
    t2.access_token = "second".into();

    store.save_token("first@example.com", &t1).unwrap();
    store.save_token("second@example.com", &t2).unwrap();

    assert_eq!(
        store
            .load_token("first@example.com")
            .unwrap()
            .unwrap()
            .access_token,
        "first"
    );
    assert_eq!(
        store
            .load_token("second@example.com")
            .unwrap()
            .unwrap()
            .access_token,
        "second"
    );
}

#[test]
fn token_serializes_to_json() {
    let token = sample_token();
    let json = serde_json::to_string(&token).unwrap();
    let parsed: Token = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed, token);
}

#[test]
fn file_store_round_trips_a_token_by_email() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileAccountStore::new(dir.path().join("tokens.json"));
    let token = sample_token();

    store.save_token("alice@example.com", &token).unwrap();
    let loaded = store.load_token("alice@example.com").unwrap();

    assert_eq!(loaded, Some(token));
}

#[test]
fn file_store_login_save_reports_prompt_free_access_guaranteed() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileAccountStore::new(dir.path().join("tokens.json"));
    let token = sample_token();

    let outcome = store
        .save_token_for_login("alice@example.com", &token)
        .unwrap();

    assert!(outcome.prompt_free_access_is_guaranteed());
    assert_eq!(store.load_token("alice@example.com").unwrap(), Some(token));
}

#[cfg(unix)]
#[test]
fn file_store_writes_user_only_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("auth.json");
    let store = FileAccountStore::new(path.clone());

    store
        .save_token("alice@example.com", &sample_token())
        .unwrap();

    let mode = std::fs::metadata(path).unwrap().permissions().mode() & 0o777;
    assert_eq!(mode, 0o600);
}

#[test]
fn file_store_returns_none_when_file_is_missing() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileAccountStore::new(dir.path().join("does-not-exist.json"));

    assert!(store.load_token("alice@example.com").unwrap().is_none());
}

#[test]
fn file_store_returns_none_for_an_email_not_in_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileAccountStore::new(dir.path().join("tokens.json"));
    store
        .save_token("alice@example.com", &sample_token())
        .unwrap();

    assert!(store.load_token("bob@example.com").unwrap().is_none());
}

#[test]
fn file_store_holds_multiple_accounts_without_clobbering() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileAccountStore::new(dir.path().join("tokens.json"));
    let mut t1 = sample_token();
    t1.access_token = "first".into();
    let mut t2 = sample_token();
    t2.access_token = "second".into();

    store.save_token("first@example.com", &t1).unwrap();
    store.save_token("second@example.com", &t2).unwrap();

    assert_eq!(
        store
            .load_token("first@example.com")
            .unwrap()
            .unwrap()
            .access_token,
        "first"
    );
    assert_eq!(
        store
            .load_token("second@example.com")
            .unwrap()
            .unwrap()
            .access_token,
        "second"
    );
}

#[test]
fn file_store_replace_all_discards_accounts_not_in_the_new_set() {
    let dir = tempfile::tempdir().unwrap();
    let store = FileAccountStore::new(dir.path().join("tokens.json"));
    store
        .save_token("stale@example.com", &sample_token())
        .unwrap();

    let mut fresh = AuthState::default();
    fresh.save_token_for_account("alice@example.com", sample_token());
    store.replace_all(&fresh).unwrap();

    assert!(store.load_token("stale@example.com").unwrap().is_none());
    assert!(store.load_token("alice@example.com").unwrap().is_some());
}

#[test]
fn resolve_account_store_uses_default_auth_state_file_by_default() {
    let _lock = ENV_LOCK.lock().unwrap();
    let _guard = EnvGuard::unset(TOKEN_FILE_ENV_VAR);

    let store = resolve_account_store().unwrap();

    assert!(matches!(store, AccountStoreImpl::File(_)));
}

#[test]
fn resolve_account_store_uses_file_only_when_token_file_is_explicit() {
    let _lock = ENV_LOCK.lock().unwrap();
    let dir = tempfile::tempdir().unwrap();
    let token_path = dir.path().join("tokens.json");
    let _guard = EnvGuard::set(TOKEN_FILE_ENV_VAR, &token_path);

    let store = resolve_account_store().unwrap();

    assert!(matches!(store, AccountStoreImpl::File(_)));
}
