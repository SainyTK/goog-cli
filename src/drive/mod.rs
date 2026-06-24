pub mod error;

pub use error::DriveError;

use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

pub const DRIVE_SCOPE: &str = "https://www.googleapis.com/auth/drive";
pub const DRIVE_SCOPES: &[&str] = &[DRIVE_SCOPE];
const DRIVE_FILES_URL: &str = "https://www.googleapis.com/drive/v3/files";
const DRIVE_FILES_FIELDS: &str = "nextPageToken,files(id,name,mimeType,modifiedTime)";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DriveFile {
    pub name: String,
    pub id: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
    #[serde(rename = "modifiedTime")]
    pub modified_time: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FilesPage {
    #[serde(default)]
    pub files: Vec<DriveFile>,
    #[serde(rename = "nextPageToken")]
    pub next_page_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListFilesOptions {
    pub page_size: u32,
    pub page_token: Option<String>,
    files_url: String,
}

impl ListFilesOptions {
    pub fn new(page_size: u32) -> Self {
        Self {
            page_size,
            page_token: None,
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub fn with_page_token(mut self, page_token: impl Into<String>) -> Self {
        self.page_token = Some(page_token.into());
        self
    }

    #[cfg(test)]
    pub(crate) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }
}

pub async fn list_files<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListFilesOptions,
) -> Result<FilesPage, DriveError> {
    let mut url = Url::parse(&options.files_url)?;
    {
        let mut query = url.query_pairs_mut();
        query
            .append_pair("pageSize", &options.page_size.to_string())
            .append_pair("orderBy", "modifiedTime desc")
            .append_pair("fields", DRIVE_FILES_FIELDS);
        if let Some(page_token) = &options.page_token {
            query.append_pair("pageToken", page_token);
        }
    }

    let response = client
        .send_with_scopes(client.get(url), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;

    match response.status() {
        status if status.is_success() => response
            .json::<FilesPage>()
            .await
            .map_err(|e| DriveError::InvalidResponse(e.to_string())),
        StatusCode::NOT_FOUND => Err(DriveError::NotFound),
        StatusCode::FORBIDDEN => Err(DriveError::PermissionDenied),
        status => {
            let body = response.text().await.unwrap_or_default();
            Err(DriveError::Api { status, body })
        }
    }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::auth::account::{testing::MemoryStore, AccountStore, Token};
    use crate::auth::config::{Config, OAuthAppConfig, SettingsConfig};

    const SINGLE_PAGE_RESPONSE: &str =
        include_str!("../../tests/fixtures/drive/files_page_single.json");
    const EMPTY_PAGE_WITH_TOKEN_RESPONSE: &str =
        include_str!("../../tests/fixtures/drive/files_page_empty_with_token.json");

    fn test_config() -> Config {
        Config {
            oauth_app: Some(OAuthAppConfig {
                client_id: "client-123".into(),
                client_secret: "secret-456".into(),
            }),
            settings: Some(SettingsConfig {
                active_account: Some("alice@example.com".into()),
                output: None,
            }),
            accounts: vec!["alice@example.com".into()],
        }
    }

    fn drive_token() -> Token {
        Token {
            access_token: "drive-access".into(),
            refresh_token: "refresh-123".into(),
            expiry: Utc::now() + Duration::hours(1),
            scopes: vec![DRIVE_SCOPE.into()],
        }
    }

    fn test_client(store: &MemoryStore) -> AuthClient<'_, MemoryStore> {
        store.save_token("alice@example.com", &drive_token()).unwrap();
        AuthClient::from_config(test_config(), store, None).unwrap()
    }

    #[tokio::test]
    async fn list_files_deserializes_a_single_page_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(header("authorization", "Bearer drive-access"))
            .and(query_param("pageSize", "50"))
            .and(query_param("orderBy", "modifiedTime desc"))
            .and(query_param("fields", DRIVE_FILES_FIELDS))
            .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let options = ListFilesOptions::new(50)
            .with_files_url(format!("{}/drive/v3/files", server.uri()));

        let page = list_files(&client, &options).await.unwrap();

        assert_eq!(page.next_page_token, None);
        assert_eq!(
            page.files,
            vec![DriveFile {
                name: "Roadmap".into(),
                id: "file-1".into(),
                mime_type: "application/vnd.google-apps.document".into(),
                modified_time: "2026-06-24T10:15:00.000Z".into(),
            }]
        );
    }

    #[tokio::test]
    async fn list_files_sends_next_page_token_and_returns_next_page_token() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(header("authorization", "Bearer drive-access"))
            .and(query_param("pageSize", "25"))
            .and(query_param("pageToken", "token-1"))
            .respond_with(ResponseTemplate::new(200).set_body_string(EMPTY_PAGE_WITH_TOKEN_RESPONSE))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let options = ListFilesOptions::new(25)
            .with_page_token("token-1")
            .with_files_url(format!("{}/drive/v3/files", server.uri()));

        let page = list_files(&client, &options).await.unwrap();

        assert_eq!(page.next_page_token.as_deref(), Some("token-2"));
        assert!(page.files.is_empty());
    }

    #[tokio::test]
    async fn list_files_returns_drive_error_for_not_found_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let options = ListFilesOptions::new(50)
            .with_files_url(format!("{}/drive/v3/files", server.uri()));

        let err = list_files(&client, &options).await.unwrap_err();

        assert!(matches!(err, DriveError::NotFound));
    }

    #[tokio::test]
    async fn list_files_returns_drive_error_for_permission_denied_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let options = ListFilesOptions::new(50)
            .with_files_url(format!("{}/drive/v3/files", server.uri()));

        let err = list_files(&client, &options).await.unwrap_err();

        assert!(matches!(err, DriveError::PermissionDenied));
    }
}
