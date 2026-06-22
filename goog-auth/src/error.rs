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
}
