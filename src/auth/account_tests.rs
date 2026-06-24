use chrono::{TimeZone, Utc};

use super::account::{AccountStore, Token};
use super::testing::MemoryStore;

fn sample_token() -> Token {
    Token {
        access_token: "access-abc".into(),
        refresh_token: "refresh-def".into(),
        expiry: Utc.with_ymd_and_hms(2030, 1, 1, 0, 0, 0).unwrap(),
        scopes: vec!["openid".into(), "email".into()],
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
