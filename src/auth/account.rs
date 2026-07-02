use std::collections::HashMap;
use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::AuthError;

const KEYRING_SERVICE: &str = "goog";

/// When set, `resolve_account_store` reads/writes tokens from this file
/// instead of the OS keychain. Intended for headless environments (e.g. a
/// Sandcastle sandbox) that have no access to the host keychain -- never set
/// this for normal interactive use, since a token file grants whoever can
/// read it full access to that account within its authorized scopes.
pub(crate) const TOKEN_FILE_ENV_VAR: &str = "GOOG_TOKEN_FILE";

type TokenMap = HashMap<String, Token>;

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

    #[allow(dead_code)]
    fn load_token(&self, email: &str) -> Result<Option<Token>, AuthError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenSaveOutcome {
    PromptFreeAccessGuaranteed,
    PromptFreeAccessNotGuaranteed,
}

impl TokenSaveOutcome {
    pub fn prompt_free_access_guaranteed() -> Self {
        Self::PromptFreeAccessGuaranteed
    }

    pub fn prompt_free_access_not_guaranteed() -> Self {
        Self::PromptFreeAccessNotGuaranteed
    }

    pub fn prompt_free_access_is_guaranteed(&self) -> bool {
        matches!(self, Self::PromptFreeAccessGuaranteed)
    }
}

pub struct KeyringStore;

impl AccountStore for KeyringStore {
    fn save_token(&self, email: &str, token: &Token) -> Result<(), AuthError> {
        let payload = serialize_keyring_token(token)?;
        save_keyring_payload(email, &payload)
    }

    fn save_token_for_login(
        &self,
        email: &str,
        token: &Token,
    ) -> Result<TokenSaveOutcome, AuthError> {
        let payload = serialize_keyring_token(token)?;
        save_keyring_payload_for_login(email, &payload)
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

fn serialize_keyring_token(token: &Token) -> Result<String, AuthError> {
    serde_json::to_string(token).map_err(|e| AuthError::Keyring(format!("serialize token: {e}")))
}

#[cfg(not(target_os = "macos"))]
fn save_keyring_payload(email: &str, payload: &str) -> Result<(), AuthError> {
    save_keyring_payload_with_default_access(email, payload)
}

#[cfg(not(target_os = "macos"))]
fn save_keyring_payload_for_login(
    email: &str,
    payload: &str,
) -> Result<TokenSaveOutcome, AuthError> {
    save_keyring_payload_with_default_access(email, payload)?;
    Ok(TokenSaveOutcome::prompt_free_access_guaranteed())
}

fn save_keyring_payload_with_default_access(email: &str, payload: &str) -> Result<(), AuthError> {
    let entry = keyring::Entry::new(KEYRING_SERVICE, email)
        .map_err(|e| AuthError::Keyring(e.to_string()))?;
    entry
        .set_password(payload)
        .map_err(|e| AuthError::Keyring(e.to_string()))
}

#[cfg(target_os = "macos")]
fn save_keyring_payload(email: &str, payload: &str) -> Result<(), AuthError> {
    macos_keychain::save_trusted_cli_password(KEYRING_SERVICE, email, payload.as_bytes())
}

#[cfg(target_os = "macos")]
fn save_keyring_payload_for_login(
    email: &str,
    payload: &str,
) -> Result<TokenSaveOutcome, AuthError> {
    match save_keyring_payload(email, payload) {
        Ok(()) => Ok(TokenSaveOutcome::prompt_free_access_guaranteed()),
        Err(_) => {
            save_keyring_payload_with_default_access(email, payload)?;
            Ok(TokenSaveOutcome::prompt_free_access_not_guaranteed())
        }
    }
}

#[cfg(target_os = "macos")]
mod macos_keychain {
    use std::ffi::CString;
    use std::os::raw::{c_char, c_void};
    use std::os::unix::ffi::OsStrExt;
    use std::ptr;

    use core_foundation::array::CFArray;
    use core_foundation::base::TCFType;
    use core_foundation::declare_TCFType;
    use core_foundation::impl_TCFType;
    use core_foundation::string::CFString;
    use core_foundation_sys::array::CFArrayRef;
    use core_foundation_sys::base::{CFTypeID, OSStatus};
    use core_foundation_sys::string::CFStringRef;
    use security_framework::base::Error;
    use security_framework::os::macos::access::SecAccess;
    use security_framework::os::macos::keychain::{SecKeychain, SecPreferencesDomain};
    use security_framework::os::macos::keychain_item::SecKeychainItem;
    use security_framework::os::macos::passwords::find_generic_password;
    use security_framework_sys::base::{
        errSecItemNotFound, errSecSuccess, SecAccessRef, SecKeychainAttribute,
        SecKeychainAttributeList, SecKeychainItemRef, SecKeychainRef,
    };

    use super::AuthError;

    const SEC_GENERIC_PASSWORD_ITEM_CLASS: u32 = u32::from_be_bytes(*b"genp");
    const SEC_ACCOUNT_ITEM_ATTR: u32 = u32::from_be_bytes(*b"acct");
    const SEC_SERVICE_ITEM_ATTR: u32 = u32::from_be_bytes(*b"svce");

    enum OpaqueSecTrustedApplicationRef {}
    type SecTrustedApplicationRef = *mut OpaqueSecTrustedApplicationRef;

    declare_TCFType! {
        SecTrustedApplication, SecTrustedApplicationRef
    }
    impl_TCFType!(
        SecTrustedApplication,
        SecTrustedApplicationRef,
        SecTrustedApplicationGetTypeID
    );

    unsafe impl Sync for SecTrustedApplication {}
    unsafe impl Send for SecTrustedApplication {}

    extern "C" {
        fn SecTrustedApplicationGetTypeID() -> CFTypeID;
        fn SecTrustedApplicationCreateFromPath(
            path: *const c_char,
            app: *mut SecTrustedApplicationRef,
        ) -> OSStatus;
        fn SecAccessCreate(
            descriptor: CFStringRef,
            trustedlist: CFArrayRef,
            access: *mut SecAccessRef,
        ) -> OSStatus;
        fn SecKeychainItemCreateFromContent(
            item_class: u32,
            attr_list: *mut SecKeychainAttributeList,
            length: u32,
            data: *const c_void,
            keychain: SecKeychainRef,
            initial_access: SecAccessRef,
            item_ref: *mut SecKeychainItemRef,
        ) -> OSStatus;
    }

    pub fn save_trusted_cli_password(
        service: &str,
        account: &str,
        password: &[u8],
    ) -> Result<(), AuthError> {
        let keychain =
            SecKeychain::default_for_domain(SecPreferencesDomain::User).map_err(to_auth_error)?;

        delete_existing(&keychain, service, account)?;

        let access = access_for_current_executable(service)?;

        let mut attrs = [
            attr(SEC_SERVICE_ITEM_ATTR, service),
            attr(SEC_ACCOUNT_ITEM_ATTR, account),
        ];
        let mut attr_list = SecKeychainAttributeList {
            count: attrs.len() as u32,
            attr: attrs.as_mut_ptr(),
        };
        let mut item = ptr::null_mut();
        let status = unsafe {
            SecKeychainItemCreateFromContent(
                SEC_GENERIC_PASSWORD_ITEM_CLASS,
                &mut attr_list,
                password.len() as u32,
                password.as_ptr().cast(),
                keychain.as_CFTypeRef() as SecKeychainRef,
                access.as_concrete_TypeRef(),
                &mut item,
            )
        };
        if !item.is_null() {
            let item = unsafe { SecKeychainItem::wrap_under_create_rule(item) };
            drop(item);
        }
        if status != errSecSuccess {
            return Err(to_auth_error(Error::from_code(status)));
        }

        Ok(())
    }

    fn access_for_current_executable(service: &str) -> Result<SecAccess, AuthError> {
        let trusted_app = current_executable_trusted_app()?;
        let trusted_apps = CFArray::from_CFTypes(&[trusted_app]);
        let descriptor = CFString::new(service);
        let mut access = ptr::null_mut();
        let status = unsafe {
            SecAccessCreate(
                descriptor.as_concrete_TypeRef(),
                trusted_apps.as_concrete_TypeRef(),
                &mut access,
            )
        };
        if status == errSecSuccess {
            Ok(unsafe { SecAccess::wrap_under_create_rule(access) })
        } else {
            Err(to_auth_error(Error::from_code(status)))
        }
    }

    fn delete_existing(
        keychain: &SecKeychain,
        service: &str,
        account: &str,
    ) -> Result<(), AuthError> {
        match find_generic_password(Some(std::slice::from_ref(keychain)), service, account) {
            Ok((_, item)) => {
                item.delete();
                Ok(())
            }
            Err(err) if err.code() == errSecItemNotFound => Ok(()),
            Err(err) => Err(to_auth_error(err)),
        }
    }

    fn current_executable_trusted_app() -> Result<SecTrustedApplication, AuthError> {
        let exe = std::env::current_exe()
            .map_err(|e| AuthError::Keyring(format!("resolve current executable: {e}")))?;
        let path = CString::new(exe.as_os_str().as_bytes())
            .map_err(|_| AuthError::Keyring("current executable path contains NUL".into()))?;
        let mut app = ptr::null_mut();
        let status = unsafe { SecTrustedApplicationCreateFromPath(path.as_ptr(), &mut app) };
        if status == errSecSuccess {
            Ok(unsafe { SecTrustedApplication::wrap_under_create_rule(app) })
        } else {
            Err(to_auth_error(Error::from_code(status)))
        }
    }

    fn attr(tag: u32, value: &str) -> SecKeychainAttribute {
        SecKeychainAttribute {
            tag,
            length: value.len() as u32,
            data: value.as_ptr() as *mut c_void,
        }
    }

    fn to_auth_error(err: Error) -> AuthError {
        AuthError::Keyring(err.to_string())
    }
}

/// An `AccountStore` backed by a single JSON file holding a map of email to
/// token, rather than the OS keychain.
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
    pub fn replace_all(&self, tokens: &TokenMap) -> Result<(), AuthError> {
        self.write_map(tokens)
    }

    fn read_map(&self) -> Result<TokenMap, AuthError> {
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

    fn write_map(&self, map: &TokenMap) -> Result<(), AuthError> {
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
fn restrict_permissions(path: &Path) -> Result<(), AuthError> {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
        .map_err(|e| AuthError::TokenFile(format!("set permissions on {}: {e}", path.display())))
}

#[cfg(not(unix))]
fn restrict_permissions(_path: &Path) -> Result<(), AuthError> {
    Ok(())
}

/// The account store actually used at runtime: the OS keychain by default,
/// or a token file when `GOOG_TOKEN_FILE` is set.
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

    fn save_token_for_login(
        &self,
        email: &str,
        token: &Token,
    ) -> Result<TokenSaveOutcome, AuthError> {
        match self {
            AccountStoreImpl::Keyring(store) => store.save_token_for_login(email, token),
            AccountStoreImpl::File(store) => store.save_token_for_login(email, token),
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
