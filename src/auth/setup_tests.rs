use std::fs;

use tempfile::NamedTempFile;

use super::config::OAuthAppType;
use super::error::AuthError;
use super::setup::parse_client_secret_file;

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
    let creds = parse_client_secret_file(file.path().to_str().unwrap()).unwrap();
    assert_eq!(creds.client_id, "id123");
    assert_eq!(creds.client_secret, "sec456");
    assert_eq!(creds.app_type, OAuthAppType::Desktop);
}

#[test]
fn parses_web_app_credentials() {
    let file = write_json(
        r#"{"web":{"client_id":"web-id","client_secret":"web-sec","redirect_uris":["https://example.com"]}}"#,
    );
    let creds = parse_client_secret_file(file.path().to_str().unwrap()).unwrap();
    assert_eq!(creds.client_id, "web-id");
    assert_eq!(creds.client_secret, "web-sec");
    assert_eq!(creds.app_type, OAuthAppType::Web);
}

#[test]
fn errors_when_file_contains_multiple_app_shapes() {
    let file = write_json(
        r#"{"installed":{"client_id":"id","client_secret":"sec"},"web":{"client_id":"web-id","client_secret":"web-sec"}}"#,
    );
    let err = parse_client_secret_file(file.path().to_str().unwrap()).unwrap_err();
    assert!(matches!(err, AuthError::OAuthAppUnrecognizedStructure));
}

#[test]
fn errors_on_missing_file() {
    let err = parse_client_secret_file("/nonexistent/path/client_secret.json").unwrap_err();
    assert!(matches!(err, AuthError::OAuthAppFileNotFound { .. }));
}

#[test]
fn errors_on_invalid_json() {
    let file = write_json("not json at all");
    let err = parse_client_secret_file(file.path().to_str().unwrap()).unwrap_err();
    assert!(matches!(err, AuthError::OAuthAppInvalidJson(_)));
}

#[test]
fn errors_on_unrecognised_structure() {
    let file = write_json(r#"{"something_else":{}}"#);
    let err = parse_client_secret_file(file.path().to_str().unwrap()).unwrap_err();
    assert!(matches!(err, AuthError::OAuthAppUnrecognizedStructure));
}

#[test]
fn errors_on_missing_client_id() {
    let file = write_json(r#"{"installed":{"client_secret":"sec"}}"#);
    let err = parse_client_secret_file(file.path().to_str().unwrap()).unwrap_err();
    assert!(matches!(&err, AuthError::OAuthAppMissingField { field } if field == "client_id"));
}

#[test]
fn errors_on_missing_client_secret() {
    let file = write_json(r#"{"installed":{"client_id":"id"}}"#);
    let err = parse_client_secret_file(file.path().to_str().unwrap()).unwrap_err();
    assert!(matches!(&err, AuthError::OAuthAppMissingField { field } if field == "client_secret"));
}

#[test]
fn errors_on_empty_client_id() {
    let file = write_json(r#"{"installed":{"client_id":"","client_secret":"sec"}}"#);
    let err = parse_client_secret_file(file.path().to_str().unwrap()).unwrap_err();
    assert!(matches!(&err, AuthError::OAuthAppMissingField { field } if field == "client_id"));
}
