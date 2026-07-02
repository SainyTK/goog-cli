use thiserror::Error;

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("OAuth App file not found: {path}")]
    OAuthAppFileNotFound { path: String },

    #[error("OAuth App file is not valid JSON: {0}")]
    OAuthAppInvalidJson(#[from] serde_json::Error),

    #[error("OAuth App file is missing required field: {field}")]
    OAuthAppMissingField { field: String },

    #[error("OAuth App file has an unrecognized structure (expected 'installed' or 'web' key)")]
    OAuthAppUnrecognizedStructure,

    #[error("failed to read OAuth App file: {0}")]
    OAuthAppIo(std::io::Error),

    #[error("config directory could not be determined")]
    ConfigDirNotFound,

    #[error("failed to read config: {0}")]
    ConfigReadIo(std::io::Error),

    #[error("failed to write config: {0}")]
    ConfigWriteIo(std::io::Error),

    #[error("config file is malformed: {0}")]
    ConfigMalformed(String),

    #[error("system keychain error: {0}")]
    Keyring(String),

    #[error("token file error: {0}")]
    TokenFile(String),

    #[error("OAuth App is not configured -- run `goog auth setup` first")]
    OAuthAppNotConfigured,

    #[error("account is not logged in: {email}")]
    AccountNotFound { email: String },

    #[error("OAuth flow failed: {0}")]
    OAuthFlow(String),

    #[error("token exchange failed: {0}")]
    TokenExchange(String),

    #[error("token for account {email} was not found -- run `goog auth login` again")]
    TokenNotFound { email: String },

    #[error("no active account configured -- run `goog auth login` or pass `--account`")]
    ActiveAccountNotConfigured,

    #[error("token was revoked or expired: {0}")]
    TokenRevoked(String),

    #[error("request was unauthorized after token refresh: {0}")]
    Unauthorized(String),

    #[error("request cannot be retried after an authorization failure")]
    RequestNotRetryable,

    #[error("network error: {0}")]
    Network(String),
}
