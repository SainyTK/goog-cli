use std::collections::HashMap;
use std::sync::Mutex;

use super::account::{AccountStore, Token};
use super::error::AuthError;

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
