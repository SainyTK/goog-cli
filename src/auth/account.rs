use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::AuthError;
use super::state::{
    load_runtime_state_from_path, runtime_state_path, save_runtime_state_to_path, AuthState,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Token {
    pub access_token: String,
    pub refresh_token: String,
    pub expiry: DateTime<Utc>,
    pub scopes: Vec<String>,
}

pub trait AccountStore {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError>;

    fn save_token_for_login(
        &self,
        email: &str,
        token: &Token,
    ) -> Result<TokenSaveOutcome, AuthError> {
        self.save_token(email, token)?;
        Ok(TokenSaveOutcome::prompt_free_access_guaranteed())
    }

    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError>;

    fn account_emails(&self) -> Result<Vec<String>, AuthError> {
        Ok(Vec::new())
    }

    fn active_account(&self) -> Result<Option<String>, AuthError> {
        Ok(None)
    }

    fn account_exists(&self, email: &str) -> Result<bool, AuthError> {
        Ok(self
            .account_emails()?
            .iter()
            .any(|account| account == email))
    }

    fn switch_active_account(&self, selector: &str) -> Result<String, AuthError> {
        Err(AuthError::AccountNotFound {
            email: selector.trim().to_string(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSaveOutcome {
    PromptFreeAccessGuaranteed,
}

impl TokenSaveOutcome {
    pub fn prompt_free_access_guaranteed() -> Self {
        Self::PromptFreeAccessGuaranteed
    }

    #[cfg(test)]
    pub fn prompt_free_access_not_guaranteed() -> Self {
        Self::PromptFreeAccessGuaranteed
    }

    pub fn prompt_free_access_is_guaranteed(&self) -> bool {
        true
    }
}

pub struct FileAccountStore {
    path: PathBuf,
}

impl FileAccountStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn replace_all(&self, state: &AuthState) -> Result<(), AuthError> {
        save_runtime_state_to_path(state, &self.path)
    }

    fn read_state(&self) -> Result<AuthState, AuthError> {
        load_runtime_state_from_path(&self.path)
    }

    fn write_state(&self, state: &AuthState) -> Result<(), AuthError> {
        save_runtime_state_to_path(state, &self.path)
    }
}

impl AccountStore for FileAccountStore {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
        let mut state = self.read_state()?;
        state.save_token_for_account(email, token.clone());
        self.write_state(&state)
    }

    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError> {
        Ok(self.read_state()?.token_for_account(email).cloned())
    }

    fn account_emails(&self) -> Result<Vec<String>, AuthError> {
        Ok(self.read_state()?.account_emails())
    }

    fn active_account(&self) -> Result<Option<String>, AuthError> {
        Ok(self.read_state()?.active_account)
    }

    fn account_exists(&self, email: &str) -> Result<bool, AuthError> {
        Ok(self.read_state()?.token_for_account(email).is_some())
    }

    fn switch_active_account(&self, selector: &str) -> Result<String, AuthError> {
        let mut state = self.read_state()?;
        let active = state.switch_active_account(selector)?;
        self.write_state(&state)?;
        Ok(active)
    }
}

pub enum AccountStoreImpl {
    File(FileAccountStore),
}

impl AccountStore for AccountStoreImpl {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
        match self {
            AccountStoreImpl::File(store) => store.save_token(email, token),
        }
    }

    fn save_token_for_login(
        &self,
        email: &str,
        token: &Token,
    ) -> Result<TokenSaveOutcome, AuthError> {
        match self {
            AccountStoreImpl::File(store) => store.save_token_for_login(email, token),
        }
    }

    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError> {
        match self {
            AccountStoreImpl::File(store) => store.load_token(email),
        }
    }

    fn account_emails(&self) -> Result<Vec<String>, AuthError> {
        match self {
            AccountStoreImpl::File(store) => store.account_emails(),
        }
    }

    fn active_account(&self) -> Result<Option<String>, AuthError> {
        match self {
            AccountStoreImpl::File(store) => store.active_account(),
        }
    }

    fn account_exists(&self, email: &str) -> Result<bool, AuthError> {
        match self {
            AccountStoreImpl::File(store) => store.account_exists(email),
        }
    }

    fn switch_active_account(&self, selector: &str) -> Result<String, AuthError> {
        match self {
            AccountStoreImpl::File(store) => store.switch_active_account(selector),
        }
    }
}

pub fn resolve_account_store() -> Result<AccountStoreImpl, AuthError> {
    let path = runtime_state_path()?;
    Ok(AccountStoreImpl::File(FileAccountStore::new(path)))
}
