use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::DriveCommand;
use crate::drive::{
    download, list_files, upload, DownloadFileOptions, DriveFile, ListFilesOptions,
    UploadFileOptions,
};

const DEFAULT_LIST_LIMIT: u32 = 50;
const ALL_PAGE_SIZE: u32 = 1000;

pub fn run<S: AccountStore>(
    cmd: DriveCommand,
    client: &AuthClient<'_, S>,
    output_json_by_default: bool,
    quiet: bool,
) -> Result<()> {
    match cmd {
        DriveCommand::List {
            limit,
            all,
            folder,
            json,
        } => {
            let json = should_emit_json(json, output_json_by_default);
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_list_to(
                client,
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
        DriveCommand::Download { file_id, output } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_download_to(
                client,
                file_id,
                output.map(PathBuf::from),
                quiet,
                None,
            ))
        }
        DriveCommand::Upload { path, folder } => {
            let runtime =
                tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_upload_to(
                client,
                PathBuf::from(path),
                folder,
                quiet,
                &mut std::io::stdout(),
                None,
            ))
        }
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

pub(super) async fn run_download_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    file_id: String,
    output: Option<PathBuf>,
    quiet: bool,
    files_url: Option<&str>,
) -> Result<()> {
    let options = download_options(file_id, output, files_url);
    let progress = (!quiet).then(new_download_progress);
    let result = download(client, &options, |bytes| {
        if let Some(progress) = &progress {
            progress.set_position(bytes);
        }
    })
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
    let mut remaining = requested_result_count(limit, all);
    let mut page_token = None;
    let mut wrote_table_header = false;
    let mut total = 0_u32;

    loop {
        let Some(page_size) = next_page_size(remaining) else {
            break;
        };
        let options = list_options(page_size, page_token.take(), folder.as_deref(), files_url);

        let page = list_files(client, &options)
            .await
            .context("failed to list Google Drive files")?;

        if json {
            write_ndjson(&page.files, out)?;
        } else {
            write_table(&page.files, out, &mut wrote_table_header)?;
        }

        let page_count = page.files.len() as u32;
        total += page_count;
        if let Some(left) = remaining.as_mut() {
            *left = left.saturating_sub(page_count);
        }

        if all && !quiet {
            writeln!(err, "Fetched {total} files...").context("failed to write progress")?;
        }

        match page.next_page_token {
            Some(token) if should_fetch_next_page(remaining, all) => {
                page_token = Some(token);
            }
            _ => break,
        }
    }

    Ok(())
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
    folder: Option<&str>,
    files_url: Option<&str>,
) -> ListFilesOptions {
    let mut options = ListFilesOptions::new(page_size);
    if let Some(page_token) = page_token {
        options = options.with_page_token(page_token);
    }
    if let Some(folder) = folder {
        options = options.with_folder(folder);
    }
    if let Some(files_url) = files_url {
        options = options.with_files_url(files_url);
    }
    options
}

pub(super) fn write_ndjson(files: &[DriveFile], out: &mut impl Write) -> Result<()> {
    for file in files {
        serde_json::to_writer(&mut *out, file).context("failed to serialize Drive file")?;
        writeln!(out).context("failed to write output")?;
    }
    Ok(())
}

pub(super) fn write_table(
    files: &[DriveFile],
    out: &mut impl Write,
    wrote_header: &mut bool,
) -> Result<()> {
    if !*wrote_header {
        writeln!(out, "NAME\tFILE ID\tPARENT FOLDER IDS\tMIME TYPE\tMODIFIED")
            .context("failed to write output")?;
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
