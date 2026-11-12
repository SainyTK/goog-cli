use serde::{Deserialize, Serialize};
use url::Url;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;

use super::{ensure_success_response, DriveError, DRIVE_FILES_URL, DRIVE_SCOPES};

const FILE_METADATA_FIELDS: &str =
    "id,name,mimeType,parents,driveId,webViewLink,modifiedTime,trashed";

/// One Drive item's metadata, including where it sits: its parent folder IDs
/// and, for an item on a shared drive, that drive's ID.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DriveFileMetadata {
    pub id: String,
    pub name: String,
    pub mime_type: String,
    #[serde(
        default,
        rename(serialize = "parentIds", deserialize = "parents"),
        skip_serializing_if = "Vec::is_empty"
    )]
    pub parent_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub drive_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub web_view_link: Option<String>,
    pub modified_time: String,
    #[serde(default)]
    pub trashed: bool,
}

#[derive(Debug, Clone)]
pub struct FileMetadataOptions {
    pub file_id: String,
    files_url: String,
}

impl FileMetadataOptions {
    pub fn new(file_id: impl Into<String>) -> Self {
        Self {
            file_id: file_id.into(),
            files_url: DRIVE_FILES_URL.to_string(),
        }
    }

    pub(crate) fn with_files_url(mut self, files_url: impl Into<String>) -> Self {
        self.files_url = files_url.into();
        self
    }

    fn request_url(&self) -> Result<Url, DriveError> {
        let mut url = Url::parse(&self.files_url)?;
        url.path_segments_mut()
            .map_err(|_| {
                DriveError::InvalidResponse("Google Drive API URL cannot be a base".into())
            })?
            .push(&self.file_id);
        url.query_pairs_mut()
            .append_pair("fields", FILE_METADATA_FIELDS)
            .append_pair("supportsAllDrives", "true");
        Ok(url)
    }
}

pub async fn get_file_metadata<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &FileMetadataOptions,
) -> Result<DriveFileMetadata, DriveError> {
    let response = client
        .send_with_scopes(client.get(options.request_url()?), DRIVE_SCOPES)
        .await
        .map_err(DriveError::Auth)?;
    ensure_success_response(response)
        .await?
        .json::<DriveFileMetadata>()
        .await
        .map_err(|error| DriveError::InvalidResponse(error.to_string()))
}
