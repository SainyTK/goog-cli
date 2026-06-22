use serde::Deserialize;

use crate::error::AuthError;

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
pub struct OAuthAppCredentials {
    pub client_id: String,
    pub client_secret: String,
}

pub fn parse_credentials_file(path: &str) -> Result<OAuthAppCredentials, AuthError> {
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
        .ok_or(AuthError::OAuthAppUnrecognisedStructure)?;

    let client_id = fields
        .client_id
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AuthError::OAuthAppMissingField {
            field: "client_id".into(),
        })?;

    let client_secret = fields
        .client_secret
        .filter(|s| !s.is_empty())
        .ok_or_else(|| AuthError::OAuthAppMissingField {
            field: "client_secret".into(),
        })?;

    Ok(OAuthAppCredentials {
        client_id,
        client_secret,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::NamedTempFile;

    fn write_json(contents: &str) -> NamedTempFile {
        let file = NamedTempFile::new().unwrap();
        fs::write(file.path(), contents).unwrap();
        file
    }

    #[test]
    fn parses_installed_app_credentials() {
        let file = write_json(
            r#"{"installed":{"client_id":"id123","client_secret":"sec456","redirect_uris":["http://localhost"]}}"#,
        );
        let creds = parse_credentials_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(creds.client_id, "id123");
        assert_eq!(creds.client_secret, "sec456");
    }

    #[test]
    fn parses_web_app_credentials() {
        let file = write_json(
            r#"{"web":{"client_id":"web-id","client_secret":"web-sec","redirect_uris":["https://example.com"]}}"#,
        );
        let creds = parse_credentials_file(file.path().to_str().unwrap()).unwrap();
        assert_eq!(creds.client_id, "web-id");
        assert_eq!(creds.client_secret, "web-sec");
    }

    #[test]
    fn errors_on_missing_file() {
        let err = parse_credentials_file("/nonexistent/path/client_secret.json").unwrap_err();
        assert!(matches!(err, AuthError::OAuthAppFileNotFound { .. }));
    }

    #[test]
    fn errors_on_invalid_json() {
        let file = write_json("not json at all");
        let err = parse_credentials_file(file.path().to_str().unwrap()).unwrap_err();
        assert!(matches!(err, AuthError::OAuthAppInvalidJson(_)));
    }

    #[test]
    fn errors_on_unrecognised_structure() {
        let file = write_json(r#"{"something_else":{}}"#);
        let err = parse_credentials_file(file.path().to_str().unwrap()).unwrap_err();
        assert!(matches!(err, AuthError::OAuthAppUnrecognisedStructure));
    }

    #[test]
    fn errors_on_missing_client_id() {
        let file = write_json(r#"{"installed":{"client_secret":"sec"}}"#);
        let err = parse_credentials_file(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            matches!(&err, AuthError::OAuthAppMissingField { field } if field == "client_id")
        );
    }

    #[test]
    fn errors_on_missing_client_secret() {
        let file = write_json(r#"{"installed":{"client_id":"id"}}"#);
        let err = parse_credentials_file(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            matches!(&err, AuthError::OAuthAppMissingField { field } if field == "client_secret")
        );
    }

    #[test]
    fn errors_on_empty_client_id() {
        let file = write_json(r#"{"installed":{"client_id":"","client_secret":"sec"}}"#);
        let err = parse_credentials_file(file.path().to_str().unwrap()).unwrap_err();
        assert!(
            matches!(&err, AuthError::OAuthAppMissingField { field } if field == "client_id")
        );
    }
}
