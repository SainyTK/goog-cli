use serde::Deserialize;

use super::error::AuthError;

#[derive(Debug, Deserialize)]
struct ClientSecretFile {
    installed: Option<OAuthAppFields>,
    web: Option<OAuthAppFields>,
}

#[derive(Debug, Deserialize)]
struct OAuthAppFields {
    client_id: Option<String>,
    client_secret: Option<String>,
}

#[derive(Debug, PartialEq)]
pub struct OAuthAppSecrets {
    pub client_id: String,
    pub client_secret: String,
}

pub fn parse_client_secret_file(path: &str) -> Result<OAuthAppSecrets, AuthError> {
    let path_buf = std::path::PathBuf::from(path);
    if !path_buf.exists() {
        return Err(AuthError::OAuthAppFileNotFound {
            path: path.to_string(),
        });
    }

    let contents = std::fs::read_to_string(&path_buf).map_err(AuthError::OAuthAppIo)?;
    let file: ClientSecretFile = serde_json::from_str(&contents)?;

    let fields = file
        .installed
        .or(file.web)
        .ok_or(AuthError::OAuthAppUnrecognizedStructure)?;

    let client_id = fields.client_id.filter(|s| !s.is_empty()).ok_or_else(|| {
        AuthError::OAuthAppMissingField {
            field: "client_id".into(),
        }
    })?;

    let client_secret = fields
        .client_secret
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AuthError::OAuthAppMissingField {
            field: "client_secret".into(),
        })?;

    Ok(OAuthAppSecrets {
        client_id,
        client_secret,
    })
}
