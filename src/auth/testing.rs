use std::collections::HashMap;
use std::sync::Mutex;

use super::account::{AccountStore, Token};
use super::error::AuthError;

#[derive(Default)]
pub struct MemoryStore {
    inner: Mutex<HashMap<String, Token>>,
    active_account: Mutex<Option<String>>,
}

impl MemoryStore {
    #[cfg(test)]
    pub fn seed_account_without_activating(&self, email: &str, token: &Token) {
        self.inner
            .lock()
            .unwrap()
            .insert(email.to_string(), token.clone());
    }
}

impl AccountStore for MemoryStore {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
        self.inner
            .lock()
            .unwrap()
            .insert(email.to_string(), token.clone());
        let mut active = self.active_account.lock().unwrap();
        if active.is_none() {
            *active = Some(email.to_string());
        }
        Ok(())
    }

    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError> {
        Ok(self.inner.lock().unwrap().get(email).cloned())
    }

    fn account_emails(&self) -> Result<Vec<String>, AuthError> {
        Ok(self.inner.lock().unwrap().keys().cloned().collect())
    }

    fn active_account(&self) -> Result<Option<String>, AuthError> {
        Ok(self.active_account.lock().unwrap().clone())
    }

    fn account_exists(&self, email: &str) -> Result<bool, AuthError> {
        Ok(self.inner.lock().unwrap().contains_key(email))
    }

    fn switch_active_account(&self, selector: &str) -> Result<String, AuthError> {
        let accounts = self.inner.lock().unwrap();
        let selector = selector.trim();
        let selector_key = selector.to_lowercase();
        let email = accounts
            .keys()
            .find(|account| account.to_lowercase() == selector_key)
            .or_else(|| {
                accounts
                    .keys()
                    .find(|account| account.to_lowercase().contains(&selector_key))
            })
            .cloned()
            .ok_or_else(|| AuthError::AccountNotFound {
                email: selector.to_string(),
            })?;
        drop(accounts);
        *self.active_account.lock().unwrap() = Some(email.clone());
        Ok(email)
    }
}
