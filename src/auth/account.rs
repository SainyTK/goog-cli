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
