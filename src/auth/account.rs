use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::AuthError;

const KEYRING_SERVICE: &str = "goog";

/// When set, `resolve_account_store` reads/writes tokens from this file
/// instead of the OS keychain. Intended for headless environments (e.g. a
/// Sandcastle sandbox) that have no access to the host keychain -- never set
/// this for normal interactive use, since a token file grants whoever can
/// read it full access to that account within its authorized scopes.
const TOKEN_FILE_ENV_VAR: &str = "GOOG_TOKEN_FILE";

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

/// An `AccountStore` backed by a single JSON file holding a map of email to
/// token, rather than the OS keychain -- populated via `goog auth export`.
pub struct FileAccountStore {
    path: PathBuf,
}

impl FileAccountStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// Overwrites the file with exactly this set of accounts, discarding
    /// whatever was there before. Used by `goog auth export` to produce a
    /// file that reflects the current keychain state, not a merge with a
    /// stale previous export.
    pub fn replace_all(&self, tokens: &std::collections::HashMap<String, Token>) -> Result<(), AuthError> {
        self.write_map(tokens)
    }

    fn read_map(&self) -> Result<std::collections::HashMap<String, Token>, AuthError> {
        match std::fs::read_to_string(&self.path) {
            Ok(payload) => serde_json::from_str(&payload)
                .map_err(|e| AuthError::TokenFile(format!("deserialize token file: {e}"))),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
            Err(e) => Err(AuthError::TokenFile(format!(
                "read {}: {e}",
                self.path.display()
            ))),
        }
    }

    fn write_map(&self, map: &std::collections::HashMap<String, Token>) -> Result<(), AuthError> {
        let payload = serde_json::to_string_pretty(map)
            .map_err(|e| AuthError::TokenFile(format!("serialize token file: {e}")))?;
        std::fs::write(&self.path, payload)
            .map_err(|e| AuthError::TokenFile(format!("write {}: {e}", self.path.display())))?;
        restrict_permissions(&self.path)
    }
}

impl AccountStore for FileAccountStore {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
        let mut map = self.read_map()?;
        map.insert(email.to_string(), token.clone());
        self.write_map(&map)
    }

    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError> {
        let map = self.read_map()?;
        Ok(map.get(email).cloned())
    }
}

#[cfg(unix)]
fn restrict_permissions(path: &std::path::Path) -> Result<(), AuthError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| AuthError::TokenFile(format!("set permissions on {}: {e}", path.display())))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &std::path::Path) -> Result<(), AuthError> {
    Ok(())
}

/// The account store actually used at runtime: the OS keychain by default,
/// or a single-account token file when `GOOG_TOKEN_FILE` is set.
pub enum AccountStoreImpl {
    Keyring(KeyringStore),
    File(FileAccountStore),
}

impl AccountStore for AccountStoreImpl {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
        match self {
            AccountStoreImpl::Keyring(store) => store.save_token(email, token),
            AccountStoreImpl::File(store) => store.save_token(email, token),
        }
    }

    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError> {
        match self {
            AccountStoreImpl::Keyring(store) => store.load_token(email),
            AccountStoreImpl::File(store) => store.load_token(email),
        }
    }
}

pub fn resolve_account_store() -> AccountStoreImpl {
    match std::env::var_os(TOKEN_FILE_ENV_VAR) {
        Some(path) if !path.is_empty() => {
            AccountStoreImpl::File(FileAccountStore::new(PathBuf::from(path)))
        }
        _ => AccountStoreImpl::Keyring(KeyringStore),
    }
}
