use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::AuthError;

const KEYRING_SERVICE: &str = "goog";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
    pub expiry: DateTime<Utc>,
    pub scopes: Vec<String>,
}

pub trait AccountStore {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError>;
    #[allow(dead_code)]
    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError>;
}

pub struct KeyringStore;

impl AccountStore for KeyringStore {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, email)
            .map_err(|e| AuthError::Keyring(e.to_string()))?;
        let payload = serde_json::to_string(token)
            .map_err(|e| AuthError::Keyring(format!("serialize token: {e}")))?;
        entry
            .set_password(&payload)
            .map_err(|e| AuthError::Keyring(e.to_string()))
    }

    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError> {
        let entry = keyring::Entry::new(KEYRING_SERVICE, email)
            .map_err(|e| AuthError::Keyring(e.to_string()))?;
        match entry.get_password() {
            Ok(payload) => {
                let token: Token = serde_json::from_str(&payload)
                    .map_err(|e| AuthError::Keyring(format!("deserialize token: {e}")))?;
                Ok(Some(token))
            }
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(AuthError::Keyring(e.to_string())),
        }
    }
}

#[cfg(test)]
pub mod testing {
    use super::*;
    use std::collections::HashMap;
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct MemoryStore {
        inner: Mutex<HashMap<String, Token>>,
    }

    impl AccountStore for MemoryStore {
        fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
            self.inner
                .lock()
                .unwrap()
                .insert(email.to_string(), token.clone());
            Ok(())
        }

        fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError> {
            Ok(self.inner.lock().unwrap().get(email).cloned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::MemoryStore;
    use super::*;
    use chrono::TimeZone;

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
            store.load_token("first@example.com").unwrap().unwrap().access_token,
            "first"
        );
        assert_eq!(
            store.load_token("second@example.com").unwrap().unwrap().access_token,
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
}
