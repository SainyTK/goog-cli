use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use serde::Serialize;

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::auth::config::Config;
use crate::auth::state::resource_key;
use crate::auth::unified_access::{AccessFuture, UnifiedAccess};
use crate::cli::{DriveCommand, DriveFolderCommand};
use crate::drive::{
    download, list_files, upload, DownloadFileOptions, DownloadedFile, DriveError, DriveFile,
    ListFilesOptions, UploadFileOptions, UploadedFile, DRIVE_FOLDER_MIME_TYPE,
};

const DEFAULT_LIST_LIMIT: u32 = 50;
const ALL_PAGE_SIZE: u32 = 1000;
const TABLE_HEADER: &str = "NAME\tFILE ID\tPARENT FOLDER IDS\tMIME TYPE\tMODIFIED";
const FOLDER_TABLE_HEADER: &str = "NAME\tFOLDER ID\tPARENT FOLDER IDS\tMODIFIED";
const BROWSE_TABLE_HEADER: &str = "TYPE\tNAME\tID\tMIME TYPE\tMODIFIED";

type DriveResult<T> = std::result::Result<T, DriveError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DriveListKind {
    Files,
    Folders,
    Browse,
}

impl DriveListKind {
    fn item_name(self) -> &'static str {
        match self {
            Self::Files => "files",
            Self::Folders => "folders",
            Self::Browse => "items",
        }
    }
}

pub fn run<S: AccountStore>(
    cmd: DriveCommand,
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    output_json_by_default: bool,
    quiet: bool,
) -> Result<()> {
    match cmd {
        DriveCommand::Ls {
            limit,
            all,
            folder,
            json,
        } => {
            let json = should_emit_json(json, output_json_by_default);
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_ls_command_to(
                config,
                store,
                account_override,
                limit,
                all,
                folder,
                json,
                quiet,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
                None,
            ))
        }
        DriveCommand::List {
            limit,
            all,
            folder,
            json,
        } => {
            let json = should_emit_json(json, output_json_by_default);
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_list_command_to(
                config,
                store,
                account_override,
                limit,
                all,
                folder,
                json,
                quiet,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
                None,
            ))
        }
        DriveCommand::Folder { command } => match command {
            DriveFolderCommand::List {
                limit,
                all,
                parent,
                json,
            } => {
                let json = should_emit_json(json, output_json_by_default);
                let runtime =
                    tokio::runtime::Runtime::new().context("failed to start async runtime")?;
                runtime.block_on(run_folder_list_command_to(
                    config,
                    store,
                    account_override,
                    limit,
                    all,
                    parent,
                    json,
                    quiet,
                    &mut std::io::stdout(),
                    &mut std::io::stderr(),
                    None,
                ))
            }
        },
        DriveCommand::Download { file_id, output } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_download_unified_to(
                config,
                store,
                account_override,
                file_id,
                output.map(PathBuf::from),
                quiet,
                None,
                None,
            ))
        }
        DriveCommand::Upload { path, folder } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            if folder.is_some() {
                runtime.block_on(run_upload_unified_to(
                    config,
                    store,
                    account_override,
                    PathBuf::from(path),
                    folder,
                    quiet,
                    &mut std::io::stdout(),
                    None,
                    None,
                ))
            } else {
                let client = AuthClient::from_config(config.clone(), store, account_override)?;
                runtime.block_on(run_upload_to(
                    &client,
                    PathBuf::from(path),
                    None,
                    quiet,
                    &mut std::io::stdout(),
                    None,
                ))
            }
        }
    }
}

pub(super) async fn run_list_command_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    limit: Option<u32>,
    all: bool,
    folder: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<()> {
    if folder.is_some() {
        run_list_unified_to(
            config,
            store,
            account_override,
            DriveListKind::Files,
            limit,
            all,
            folder,
            json,
            quiet,
            out,
            err,
            files_url,
            None,
        )
        .await
    } else {
        let client = AuthClient::from_config(config.clone(), store, account_override)?;
        run_list_to(&client, limit, all, None, json, quiet, out, err, files_url).await
    }
}

pub(super) async fn run_ls_command_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    limit: Option<u32>,
    all: bool,
    folder: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<()> {
    if folder.is_some() {
        run_list_unified_to(
            config,
            store,
            account_override,
            DriveListKind::Browse,
            limit,
            all,
            folder,
            json,
            quiet,
            out,
            err,
            files_url,
            None,
        )
        .await
    } else {
        let client = AuthClient::from_config(config.clone(), store, account_override)?;
        run_ls_to(&client, limit, all, None, json, quiet, out, err, files_url).await
    }
}

pub(super) async fn run_folder_list_command_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<()> {
    if parent.is_some() {
        run_list_unified_to(
            config,
            store,
            account_override,
            DriveListKind::Folders,
            limit,
            all,
            parent,
            json,
            quiet,
            out,
            err,
            files_url,
            None,
        )
        .await
    } else {
        let client = AuthClient::from_config(config.clone(), store, account_override)?;
        run_folder_list_to(&client, limit, all, None, json, quiet, out, err, files_url).await
    }
}

pub(super) async fn run_upload_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    path: PathBuf,
    folder: Option<String>,
    quiet: bool,
    out: &mut impl Write,
    upload_url: Option<&str>,
) -> Result<()> {
    let file_size = tokio::fs::metadata(&path)
        .await
        .with_context(|| format!("failed to read upload file metadata: {}", path.display()))?
        .len();
    let options = upload_options(path, folder, upload_url);
    let progress = (!quiet).then(|| new_upload_progress(file_size));
    let uploaded = upload(client, &options, |bytes| {
        if let Some(progress) = &progress {
            progress.set_position(bytes);
        }
    })
    .await
    .context("failed to upload Google Drive file")?;

    if let Some(progress) = progress {
        progress.finish_and_clear();
    }

    writeln!(out, "{}\t{}", uploaded.id, uploaded.web_view_link)
        .context("failed to write output")?;
    Ok(())
}

pub(super) async fn run_upload_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    path: PathBuf,
    folder: Option<String>,
    quiet: bool,
    out: &mut impl Write,
    upload_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let Some(folder_id) = folder.clone() else {
        let client = AuthClient::from_config(config.clone(), store, account_override)?;
        return run_upload_to(&client, path, None, quiet, out, upload_url).await;
    };

    let file_size = tokio::fs::metadata(&path)
        .await
        .with_context(|| format!("failed to read upload file metadata: {}", path.display()))?
        .len();
    let options = upload_options(path, Some(folder_id.clone()), upload_url);
    let resource_key = resource_key("drive", &folder_id);
    let progress = (!quiet).then(|| new_upload_progress(file_size));
    let uploaded = upload_with_drive_unified_access(
        config,
        store,
        account_override,
        &resource_key,
        &options,
        &progress,
        state_path,
    )
    .await
    .context("failed to upload Google Drive file")?;

    if let Some(progress) = progress {
        progress.finish_and_clear();
    }

    writeln!(out, "{}\t{}", uploaded.id, uploaded.web_view_link)
        .context("failed to write output")?;
    Ok(())
}

pub(super) fn upload_options(
    path: PathBuf,
    folder: Option<String>,
    upload_url: Option<&str>,
) -> UploadFileOptions {
    let mut options = UploadFileOptions::new(path);
    if let Some(folder) = folder {
        options = options.with_folder(folder);
    }
    if let Some(upload_url) = upload_url {
        options = options.with_upload_url(upload_url);
    }
    options
}

pub(super) async fn run_download_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    file_id: String,
    output: Option<PathBuf>,
    quiet: bool,
    files_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let options = download_options(file_id.clone(), output, files_url);
    let resource_key = resource_key("drive", &file_id);
    let progress = (!quiet).then(new_download_progress);
    let result = download_with_drive_unified_access(
        config,
        store,
        account_override,
        &resource_key,
        &options,
        &progress,
        state_path,
    )
    .await
    .context("failed to download Google Drive file")?;

    if let Some(progress) = progress {
        progress.finish_and_clear();
    }

    if !quiet {
        eprintln!(
            "Downloaded {} bytes to {}",
            result.bytes,
            result.path.display()
        );
    }

    Ok(())
}

pub(super) fn download_options(
    file_id: String,
    output: Option<PathBuf>,
    files_url: Option<&str>,
) -> DownloadFileOptions {
    let mut options = DownloadFileOptions::new(file_id);
    if let Some(output) = output {
        options = options.with_output(output);
    }
    if let Some(files_url) = files_url {
        options = options.with_files_url(files_url);
    }
    options
}

fn new_download_progress() -> ProgressBar {
    let progress = ProgressBar::new_spinner();
    progress.set_style(
        ProgressStyle::with_template("{spinner} {bytes} downloaded ({bytes_per_sec})")
            .expect("download progress template is valid"),
    );
    progress.enable_steady_tick(std::time::Duration::from_millis(100));
    progress
}

fn new_upload_progress(total_bytes: u64) -> ProgressBar {
    let progress = ProgressBar::new(total_bytes);
    progress.set_style(
        ProgressStyle::with_template(
            "{bar:40.cyan/blue} {bytes}/{total_bytes} uploaded ({bytes_per_sec})",
        )
        .expect("upload progress template is valid"),
    );
    progress
}

pub(super) fn should_emit_json(json_flag: bool, output_json_by_default: bool) -> bool {
    json_flag || output_json_by_default
}

pub(super) async fn run_list_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    limit: Option<u32>,
    all: bool,
    folder: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<()> {
    run_list_items_to(
        client,
        DriveListKind::Files,
        limit,
        all,
        folder,
        json,
        quiet,
        out,
        err,
        files_url,
    )
    .await
}

#[allow(clippy::too_many_arguments)]
pub(super) async fn run_list_unified_to<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    kind: DriveListKind,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
    state_path: Option<&Path>,
) -> Result<()> {
    let Some(parent_id) = parent.clone() else {
        let client = AuthClient::from_config(config.clone(), store, account_override)?;
        return run_list_items_to(
            &client, kind, limit, all, None, json, quiet, out, err, files_url,
        )
        .await;
    };

    let resource_key = resource_key("drive", &parent_id);
    let mut files = collect_list_items_with_drive_unified_access(
        config,
        store,
        account_override,
        &resource_key,
        kind,
        limit,
        all,
        Some(parent_id),
        quiet,
        err,
        files_url,
        state_path,
    )
    .await
    .context("failed to list Google Drive files")?;

    prepare_list_items(kind, &mut files);

    if json {
        match kind {
            DriveListKind::Browse => write_browse_ndjson(&files, out)?,
            DriveListKind::Files | DriveListKind::Folders => write_ndjson(&files, out)?,
        }
    } else {
        let mut wrote_table_header = false;
        match kind {
            DriveListKind::Files => write_table(&files, out, &mut wrote_table_header)?,
            DriveListKind::Folders => write_folder_table(&files, out, &mut wrote_table_header)?,
            DriveListKind::Browse => write_browse_table(&files, out, &mut wrote_table_header)?,
        }
    }

    Ok(())
}

pub(super) async fn run_folder_list_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<()> {
    run_list_items_to(
        client,
        DriveListKind::Folders,
        limit,
        all,
        parent,
        json,
        quiet,
        out,
        err,
        files_url,
    )
    .await
}

pub(super) async fn run_ls_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    limit: Option<u32>,
    all: bool,
    folder: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<()> {
    run_list_items_to(
        client,
        DriveListKind::Browse,
        limit,
        all,
        folder,
        json,
        quiet,
        out,
        err,
        files_url,
    )
    .await
}

async fn run_list_items_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    kind: DriveListKind,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<()> {
    let mut wrote_table_header = false;
    let mut files =
        collect_list_items(client, kind, limit, all, parent, quiet, err, files_url).await?;
    prepare_list_items(kind, &mut files);

    if json {
        match kind {
            DriveListKind::Browse => write_browse_ndjson(&files, out)?,
            DriveListKind::Files | DriveListKind::Folders => write_ndjson(&files, out)?,
        }
    } else {
        match kind {
            DriveListKind::Files => write_table(&files, out, &mut wrote_table_header)?,
            DriveListKind::Folders => write_folder_table(&files, out, &mut wrote_table_header)?,
            DriveListKind::Browse => write_browse_table(&files, out, &mut wrote_table_header)?,
        }
    }

    Ok(())
}

async fn upload_with_drive_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    options: &UploadFileOptions,
    progress: &Option<ProgressBar>,
    state_path: Option<&Path>,
) -> DriveResult<UploadedFile> {
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, UploadedFile, DriveError> {
            Box::pin(upload_as_account(config, store, options, progress, account))
        },
        is_target_access_failure,
    )
    .await
}

async fn upload_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    options: &UploadFileOptions,
    progress: &Option<ProgressBar>,
    account: String,
) -> DriveResult<UploadedFile> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))?;
    let uploaded = upload(&client, options, |bytes| {
        if let Some(progress) = progress {
            progress.set_position(bytes);
        }
    })
    .await?;
    Ok(uploaded)
}

async fn download_with_drive_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    options: &DownloadFileOptions,
    progress: &Option<ProgressBar>,
    state_path: Option<&Path>,
) -> DriveResult<DownloadedFile> {
    UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, DownloadedFile, DriveError> {
            Box::pin(download_as_account(
                config, store, options, progress, account,
            ))
        },
        is_target_access_failure,
    )
    .await
}

async fn download_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    options: &DownloadFileOptions,
    progress: &Option<ProgressBar>,
    account: String,
) -> DriveResult<DownloadedFile> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))?;
    let downloaded = download(&client, options, |bytes| {
        if let Some(progress) = progress {
            progress.set_position(bytes);
        }
    })
    .await?;
    Ok(downloaded)
}

#[allow(clippy::too_many_arguments)]
async fn collect_list_items_with_drive_unified_access<S: AccountStore>(
    config: &Config,
    store: &S,
    account_override: Option<&str>,
    target_resource_key: &str,
    kind: DriveListKind,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    quiet: bool,
    err: &mut impl Write,
    files_url: Option<&str>,
    state_path: Option<&Path>,
) -> DriveResult<Vec<DriveFile>> {
    let (files, progress_output) = UnifiedAccess::run(
        config,
        account_override,
        target_resource_key,
        state_path,
        |account| -> AccessFuture<'_, (Vec<DriveFile>, Vec<u8>), DriveError> {
            let parent = parent.clone();
            Box::pin(async move {
                let mut progress_output = Vec::new();
                let files = collect_list_items_as_account(
                    config,
                    store,
                    kind,
                    limit,
                    all,
                    parent,
                    quiet,
                    &mut progress_output,
                    files_url,
                    account,
                )
                .await?;
                Ok((files, progress_output))
            })
        },
        is_target_access_failure,
    )
    .await?;

    err.write_all(&progress_output).map_err(DriveError::Io)?;
    Ok(files)
}

#[allow(clippy::too_many_arguments)]
async fn collect_list_items_as_account<S: AccountStore>(
    config: &Config,
    store: &S,
    kind: DriveListKind,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    quiet: bool,
    err: &mut impl Write,
    files_url: Option<&str>,
    account: String,
) -> DriveResult<Vec<DriveFile>> {
    let client = AuthClient::from_config(config.clone(), store, Some(&account))?;
    let files =
        collect_list_items_drive_error(&client, kind, limit, all, parent, quiet, err, files_url)
            .await?;
    Ok(files)
}

fn prepare_list_items(kind: DriveListKind, files: &mut [DriveFile]) {
    if kind == DriveListKind::Browse {
        sort_browse_items(files);
    }
}

async fn collect_list_items<S: AccountStore>(
    client: &AuthClient<'_, S>,
    kind: DriveListKind,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    quiet: bool,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> Result<Vec<DriveFile>> {
    let mut remaining = requested_result_count(limit, all);
    let mut page_token = None;
    let mut total = 0_u32;
    let mut files = Vec::new();

    loop {
        let Some(page_size) = next_page_size(remaining) else {
            break;
        };
        let options = list_options(
            page_size,
            page_token.take(),
            parent.as_deref(),
            files_url,
            kind,
        );

        let page = list_files(client, &options)
            .await
            .context("failed to list Google Drive files")?;

        let page_count = page.files.len() as u32;
        total += page_count;
        if let Some(left) = remaining.as_mut() {
            *left = left.saturating_sub(page_count);
        }

        if all && !quiet {
            writeln!(err, "Fetched {total} {}...", kind.item_name())
                .context("failed to write progress")?;
        }

        files.extend(page.files);

        match page.next_page_token {
            Some(token) if should_fetch_next_page(remaining, all) => {
                page_token = Some(token);
            }
            _ => break,
        }
    }

    Ok(files)
}

async fn collect_list_items_drive_error<S: AccountStore>(
    client: &AuthClient<'_, S>,
    kind: DriveListKind,
    limit: Option<u32>,
    all: bool,
    parent: Option<String>,
    quiet: bool,
    err: &mut impl Write,
    files_url: Option<&str>,
) -> DriveResult<Vec<DriveFile>> {
    let mut remaining = requested_result_count(limit, all);
    let mut page_token = None;
    let mut total = 0_u32;
    let mut files = Vec::new();

    loop {
        let Some(page_size) = next_page_size(remaining) else {
            break;
        };
        let options = list_options(
            page_size,
            page_token.take(),
            parent.as_deref(),
            files_url,
            kind,
        );

        let page = list_files(client, &options).await?;

        let page_count = page.files.len() as u32;
        total += page_count;
        if let Some(left) = remaining.as_mut() {
            *left = left.saturating_sub(page_count);
        }

        if all && !quiet {
            writeln!(err, "Fetched {total} {}...", kind.item_name()).map_err(DriveError::Io)?;
        }

        files.extend(page.files);

        match page.next_page_token {
            Some(token) if should_fetch_next_page(remaining, all) => {
                page_token = Some(token);
            }
            _ => break,
        }
    }

    Ok(files)
}

fn is_target_access_failure(err: &DriveError) -> bool {
    matches!(err, DriveError::NotFound | DriveError::PermissionDenied)
}

pub(super) fn requested_result_count(limit: Option<u32>, all: bool) -> Option<u32> {
    if all {
        limit
    } else {
        Some(limit.unwrap_or(DEFAULT_LIST_LIMIT))
    }
}

pub(super) fn next_page_size(remaining: Option<u32>) -> Option<u32> {
    let page_size = remaining.unwrap_or(ALL_PAGE_SIZE).min(ALL_PAGE_SIZE);
    (page_size > 0).then_some(page_size)
}

pub(super) fn should_fetch_next_page(remaining: Option<u32>, all: bool) -> bool {
    remaining.map_or(all, |left| left > 0)
}

pub(super) fn list_options(
    page_size: u32,
    page_token: Option<String>,
    parent: Option<&str>,
    files_url: Option<&str>,
    kind: DriveListKind,
) -> ListFilesOptions {
    let mut options = match kind {
        DriveListKind::Files => ListFilesOptions::new(page_size),
        DriveListKind::Folders => ListFilesOptions::folders(page_size),
        DriveListKind::Browse => ListFilesOptions::browse(page_size),
    };
    if let Some(page_token) = page_token {
        options = options.with_page_token(page_token);
    }
    if let Some(parent) = parent {
        options = options.with_folder(parent);
    }
    if let Some(files_url) = files_url {
        options = options.with_files_url(files_url);
    }
    options
}

pub(super) fn sort_browse_items(files: &mut [DriveFile]) {
    files.sort_by(|left, right| {
        browse_type_rank(left)
            .cmp(&browse_type_rank(right))
            .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
            .then_with(|| left.name.cmp(&right.name))
            .then_with(|| left.id.cmp(&right.id))
    });
}

fn browse_type_rank(file: &DriveFile) -> u8 {
    if is_folder(file) {
        0
    } else {
        1
    }
}

fn is_folder(file: &DriveFile) -> bool {
    file.mime_type == DRIVE_FOLDER_MIME_TYPE
}

fn browse_type(file: &DriveFile) -> &'static str {
    if is_folder(file) {
        "folder"
    } else {
        "file"
    }
}

pub(super) fn write_ndjson(files: &[DriveFile], out: &mut impl Write) -> Result<()> {
    for file in files {
        serde_json::to_writer(&mut *out, file).context("failed to serialize Drive file")?;
        writeln!(out).context("failed to write output")?;
    }
    Ok(())
}

#[derive(Serialize)]
struct BrowseFileJson<'a> {
    name: &'a str,
    id: &'a str,
    parents: &'a [String],
    #[serde(rename = "mimeType")]
    mime_type: &'a str,
    #[serde(rename = "modifiedTime")]
    modified_time: &'a str,
}

pub(super) fn write_browse_ndjson(files: &[DriveFile], out: &mut impl Write) -> Result<()> {
    for file in files {
        let row = BrowseFileJson {
            name: &file.name,
            id: &file.id,
            parents: &file.parent_ids,
            mime_type: &file.mime_type,
            modified_time: &file.modified_time,
        };
        serde_json::to_writer(&mut *out, &row).context("failed to serialize Drive browse item")?;
        writeln!(out).context("failed to write output")?;
    }
    Ok(())
}

pub(super) fn write_browse_table(
    files: &[DriveFile],
    out: &mut impl Write,
    wrote_header: &mut bool,
) -> Result<()> {
    if !*wrote_header {
        writeln!(out, "{BROWSE_TABLE_HEADER}").context("failed to write output")?;
        *wrote_header = true;
    }

    for file in files {
        let mime_type = if is_folder(file) {
            ""
        } else {
            file.mime_type.as_str()
        };
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            browse_type(file),
            file.name,
            file.id,
            mime_type,
            file.modified_time
        )
        .context("failed to write output")?;
    }

    Ok(())
}

pub(super) fn write_table(
    files: &[DriveFile],
    out: &mut impl Write,
    wrote_header: &mut bool,
) -> Result<()> {
    if !*wrote_header {
        writeln!(out, "{TABLE_HEADER}").context("failed to write output")?;
        *wrote_header = true;
    }

    for file in files {
        writeln!(
            out,
            "{}\t{}\t{}\t{}\t{}",
            file.name,
            file.id,
            file.parent_ids.join(","),
            file.mime_type,
            file.modified_time
        )
        .context("failed to write output")?;
    }

    Ok(())
}

pub(super) fn write_folder_table(
    files: &[DriveFile],
    out: &mut impl Write,
    wrote_header: &mut bool,
) -> Result<()> {
    if !*wrote_header {
        writeln!(out, "{FOLDER_TABLE_HEADER}").context("failed to write output")?;
        *wrote_header = true;
    }

    for file in files {
        writeln!(
            out,
            "{}\t{}\t{}\t{}",
            file.name,
            file.id,
            file.parent_ids.join(","),
            file.modified_time
        )
        .context("failed to write output")?;
    }

    Ok(())
}
