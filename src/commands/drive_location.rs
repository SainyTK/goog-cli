//! Commands that show where Drive items sit: one item's metadata with its
//! parents and shared drive, and the list of shared drives.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result};

use super::drive::is_target_access_failure;
use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::drive::{
    get_file_metadata, list_shared_drives, DriveError, DriveFileMetadata, FileMetadataOptions,
    ListSharedDrivesOptions, SharedDrive,
};

const SHARED_DRIVES_TABLE_HEADER: &str = "NAME\tDRIVE ID";
const EMPTY_VALUE: &str = "-";

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_info_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    file_id: String,
    json: bool,
    out: &mut impl Write,
    files_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let mut options = FileMetadataOptions::new(file_id.clone());
    if let Some(files_url) = files_url {
        options = options.with_files_url(files_url);
    }
    let target_resource_key = resource_key("drive", &file_id);
    let metadata = UnifiedAccess::run(
        config,
        account_override,
        &target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, DriveFileMetadata, DriveError> {
            Box::pin(get_file_metadata_as_account(
                config, store, &options, account,
            ))
        },
        is_target_access_failure,
    )
    .await
    .context("failed to read Google Drive file metadata")?;

    write_file_metadata(out, &metadata, json)
}

async fn get_file_metadata_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    options: &FileMetadataOptions,
    account: String,
) -> Result<DriveFileMetadata, DriveError> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))?;
    get_file_metadata(&client, options).await
}

fn write_file_metadata(
    out: &mut impl Write,
    metadata: &DriveFileMetadata,
    json: bool,
) -> Result<()> {
    if json {
        serde_json::to_writer(&mut *out, metadata)
            .context("failed to serialize Google Drive file metadata")?;
        return writeln!(out).context("failed to write output");
    }
    let parent_ids = if metadata.parent_ids.is_empty() {
        EMPTY_VALUE.to_string()
    } else {
        metadata.parent_ids.join(",")
    };
    let rows = [
        ("NAME", metadata.name.as_str()),
        ("ID", metadata.id.as_str()),
        ("MIME TYPE", metadata.mime_type.as_str()),
        ("PARENT FOLDER IDS", parent_ids.as_str()),
        (
            "SHARED DRIVE ID",
            metadata.drive_id.as_deref().unwrap_or(EMPTY_VALUE),
        ),
        ("MODIFIED", metadata.modified_time.as_str()),
        ("TRASHED", if metadata.trashed { "true" } else { "false" }),
        (
            "LINK",
            metadata.web_view_link.as_deref().unwrap_or(EMPTY_VALUE),
        ),
    ];
    for (label, value) in rows {
        writeln!(out, "{label}\t{value}").context("failed to write output")?;
    }
    Ok(())
}

pub(super) async fn run_drives_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    json: bool,
    out: &mut impl Write,
    drives_url: Option<&str>,
) -> Result<()> {
    let mut options = ListSharedDrivesOptions::default();
    if let Some(drives_url) = drives_url {
        options = options.with_drives_url(drives_url);
    }
    let drives = list_shared_drives(client, &options)
        .await
        .context("failed to list Google shared drives")?;

    write_shared_drives(out, &drives, json)
}

fn write_shared_drives(out: &mut impl Write, drives: &[SharedDrive], json: bool) -> Result<()> {
    if json {
        for drive in drives {
            serde_json::to_writer(&mut *out, drive)
                .context("failed to serialize Google shared drive")?;
            writeln!(out).context("failed to write output")?;
        }
        return Ok(());
    }
    writeln!(out, "{SHARED_DRIVES_TABLE_HEADER}").context("failed to write output")?;
    for drive in drives {
        writeln!(out, "{}\t{}", drive.name, drive.id).context("failed to write output")?;
    }
    Ok(())
}
