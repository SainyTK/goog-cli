use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

use super::{ensure_success_response, DriveError, DRIVE_SCOPES};

const DRIVE_DRIVES_URL: &str = "https://www.googleapis.com/drive/v3/drives";
const SHARED_DRIVES_FIELDS: &str = "nextPageToken,drives(id,name)";
const SHARED_DRIVES_PAGE_SIZE: &str = "100";

/// A shared drive the account is a member of. Its ID is also the ID of the
/// drive's root folder, so it works anywhere a folder ID does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SharedDrive {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SharedDrivesPage {
    #[serde(default)]
    drives: Vec<SharedDrive>,
    next_page_token: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ListSharedDrivesOptions {
    drives_url: String,
}

impl Default for ListSharedDrivesOptions {
    fn default() -> Self {
        Self {
            drives_url: DRIVE_DRIVES_URL.to_string(),
        }
    }
}

impl ListSharedDrivesOptions {
    pub(crate) fn with_drives_url(mut self, drives_url: impl Into<String>) -> Self {
        self.drives_url = drives_url.into();
        self
    }

    fn request_url(&self, page_token: Option<&str>) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.drives_url)?;
        {
            let mut query = url.query_pairs_mut();
            query
                .append_pair("pageSize", SHARED_DRIVES_PAGE_SIZE)
                .append_pair("fields", SHARED_DRIVES_FIELDS);
            if let Some(page_token) = page_token {
                query.append_pair("pageToken", page_token);
            }
        }
        Ok(url)
    }
}

/// Lists every shared drive the account is a member of, across all pages,
/// sorted by name.
pub async fn list_shared_drives<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &ListSharedDrivesOptions,
) -> Result<Vec<SharedDrive>, DriveError> {
    let mut drives = Vec::new();
    let mut page_token = None;
    loop {
        let response = client
            .send_with_scopes(
                client.get(options.request_url(page_token.as_deref())?),
                DRIVE_SCOPES,
            )
            .await
            .map_err(DriveError::Auth)?;
        let page = ensure_success_response(response)
            .await?
            .json::<SharedDrivesPage>()
            .await
            .map_err(|error| DriveError::InvalidResponse(error.to_string()))?;
        drives.extend(page.drives);
        match page.next_page_token {
            Some(next_page_token) => page_token = Some(next_page_token),
            None => break,
        }
    }
    drives.sort_by(|left, right| {
        left.name
            .to_lowercase()
            .cmp(&right.name.to_lowercase())
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(drives)
}
