use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::cli::DriveCommand;
use crate::drive::{download, list_files, DownloadFileOptions, DriveFile, ListFilesOptions};

const DEFAULT_LIST_LIMIT: u32 = 50;
const ALL_PAGE_SIZE: u32 = 1000;

pub fn run<S: AccountStore>(
    cmd: DriveCommand,
    client: &AuthClient<'_, S>,
    output_json_by_default: bool,
    quiet: bool,
) -> Result<()> {
    match cmd {
        DriveCommand::List { limit, all, json } => {
            let json = should_emit_json(json, output_json_by_default);
            let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_list_to(
                client,
                limit,
                all,
                json,
                quiet,
                &mut std::io::stdout(),
                &mut std::io::stderr(),
                None,
            ))
        }
        DriveCommand::Download { file_id, output } => {
            let runtime = tokio::runtime::Runtime::new().context("failed to start async runtime")?;
            runtime.block_on(run_download_to(
                client,
                file_id,
                output.map(PathBuf::from),
                quiet,
                None,
            ))
        }
        DriveCommand::Upload { .. } => {
            println!("not yet implemented");
            Ok(())
        }
    }
}

async fn run_download_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    file_id: String,
    output: Option<PathBuf>,
    quiet: bool,
    #[cfg_attr(not(test), allow(unused_variables))] files_url: Option<&str>,
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
        eprintln!("Downloaded {} bytes to {}", result.bytes, result.path.display());
    }

    Ok(())
}

fn download_options(
    file_id: String,
    output: Option<PathBuf>,
    #[cfg_attr(not(test), allow(unused_variables))] files_url: Option<&str>,
) -> DownloadFileOptions {
    let mut options = DownloadFileOptions::new(file_id);
    if let Some(output) = output {
        options = options.with_output(output);
    }
    #[cfg(test)]
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

fn should_emit_json(json_flag: bool, output_json_by_default: bool) -> bool {
    json_flag || output_json_by_default
}

async fn run_list_to<S: AccountStore>(
    client: &AuthClient<'_, S>,
    limit: Option<u32>,
    all: bool,
    json: bool,
    quiet: bool,
    out: &mut impl Write,
    err: &mut impl Write,
    #[cfg_attr(not(test), allow(unused_variables))] files_url: Option<&str>,
) -> Result<()> {
    let mut remaining = requested_result_count(limit, all);
    let mut page_token = None;
    let mut wrote_table_header = false;
    let mut total = 0_u32;

    loop {
        let Some(page_size) = next_page_size(remaining) else {
            break;
        };
        let options = list_options(page_size, page_token.take(), files_url);

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

fn requested_result_count(limit: Option<u32>, all: bool) -> Option<u32> {
    if all {
        limit
    } else {
        Some(limit.unwrap_or(DEFAULT_LIST_LIMIT))
    }
}

fn next_page_size(remaining: Option<u32>) -> Option<u32> {
    let page_size = remaining.unwrap_or(ALL_PAGE_SIZE).min(ALL_PAGE_SIZE);
    (page_size > 0).then_some(page_size)
}

fn should_fetch_next_page(remaining: Option<u32>, all: bool) -> bool {
    remaining.map_or(all, |left| left > 0)
}

fn list_options(
    page_size: u32,
    page_token: Option<String>,
    #[cfg_attr(not(test), allow(unused_variables))] files_url: Option<&str>,
) -> ListFilesOptions {
    let mut options = ListFilesOptions::new(page_size);
    if let Some(page_token) = page_token {
        options = options.with_page_token(page_token);
    }
    #[cfg(test)]
    if let Some(files_url) = files_url {
        options = options.with_files_url(files_url);
    }
    options
}

fn write_ndjson(files: &[DriveFile], out: &mut impl Write) -> Result<()> {
    for file in files {
        serde_json::to_writer(&mut *out, file).context("failed to serialize Drive file")?;
        writeln!(out).context("failed to write output")?;
    }
    Ok(())
}

fn write_table(
    files: &[DriveFile],
    out: &mut impl Write,
    wrote_header: &mut bool,
) -> Result<()> {
    if !*wrote_header {
        writeln!(out, "NAME\tFILE ID\tMIME TYPE\tMODIFIED").context("failed to write output")?;
        *wrote_header = true;
    }

    for file in files {
        writeln!(
            out,
            "{}\t{}\t{}\t{}",
            file.name, file.id, file.mime_type, file.modified_time
        )
        .context("failed to write output")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, Utc};
    use wiremock::matchers::{header, method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    use super::*;
    use crate::auth::account::{testing::MemoryStore, AccountStore, Token};
    use crate::auth::config::{Config, OAuthAppConfig, SettingsConfig};
    use crate::drive::DRIVE_SCOPE;

    const SINGLE_PAGE_RESPONSE: &str =
        include_str!("../../tests/fixtures/drive/files_page_single.json");
    const FIRST_PAGE_RESPONSE: &str =
        include_str!("../../tests/fixtures/drive/files_page_first.json");
    const SECOND_PAGE_RESPONSE: &str =
        include_str!("../../tests/fixtures/drive/files_page_second.json");

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

    #[test]
    fn write_ndjson_uses_drive_api_field_names() {
        let mut out = Vec::new();
        write_ndjson(
            &[DriveFile {
                name: "Roadmap".into(),
                id: "file-1".into(),
                mime_type: "application/vnd.google-apps.document".into(),
                modified_time: "2026-06-24T10:15:00.000Z".into(),
            }],
            &mut out,
        )
        .unwrap();

        let rendered = String::from_utf8(out).unwrap();
        assert_eq!(
            rendered,
            "{\"name\":\"Roadmap\",\"id\":\"file-1\",\"mimeType\":\"application/vnd.google-apps.document\",\"modifiedTime\":\"2026-06-24T10:15:00.000Z\"}\n"
        );
    }

    #[test]
    fn write_table_includes_expected_columns() {
        let mut out = Vec::new();
        let mut wrote_header = false;
        write_table(
            &[DriveFile {
                name: "Roadmap".into(),
                id: "file-1".into(),
                mime_type: "application/vnd.google-apps.document".into(),
                modified_time: "2026-06-24T10:15:00.000Z".into(),
            }],
            &mut out,
            &mut wrote_header,
        )
        .unwrap();

        let rendered = String::from_utf8(out).unwrap();
        assert!(rendered.contains("NAME\tFILE ID\tMIME TYPE\tMODIFIED"));
        assert!(rendered.contains("Roadmap\tfile-1\tapplication/vnd.google-apps.document"));
    }

    #[test]
    fn json_flag_overrides_table_default() {
        assert!(should_emit_json(true, false));
    }

    #[test]
    fn config_output_json_flips_default_output() {
        assert!(should_emit_json(false, true));
    }

    #[tokio::test]
    async fn run_list_uses_json_when_config_default_requests_json() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(header("authorization", "Bearer drive-access"))
            .and(query_param("pageSize", "50"))
            .respond_with(ResponseTemplate::new(200).set_body_string(SINGLE_PAGE_RESPONSE))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let files_url = format!("{}/drive/v3/files", server.uri());

        run_list_to(
            &client,
            None,
            false,
            true,
            false,
            &mut out,
            &mut err,
            Some(&files_url),
        )
        .await
        .unwrap();

        assert_eq!(
            String::from_utf8(out).unwrap(),
            "{\"name\":\"Roadmap\",\"id\":\"file-1\",\"mimeType\":\"application/vnd.google-apps.document\",\"modifiedTime\":\"2026-06-24T10:15:00.000Z\"}\n"
        );
        assert!(err.is_empty());
    }

    #[tokio::test]
    async fn run_list_all_fetches_following_pages_and_reports_progress() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("pageSize", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(FIRST_PAGE_RESPONSE))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("pageSize", "1"))
            .and(query_param("pageToken", "token-2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(SECOND_PAGE_RESPONSE))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let files_url = format!("{}/drive/v3/files", server.uri());

        run_list_to(
            &client,
            Some(2),
            true,
            false,
            false,
            &mut out,
            &mut err,
            Some(&files_url),
        )
        .await
        .unwrap();

        let rendered = String::from_utf8(out).unwrap();
        assert!(rendered.contains("First\tfile-1\ttext/plain"));
        assert!(rendered.contains("Second\tfile-2\ttext/plain"));
        assert_eq!(
            String::from_utf8(err).unwrap(),
            "Fetched 1 files...\nFetched 2 files...\n"
        );
    }

    #[tokio::test]
    async fn run_list_limit_can_span_multiple_pages_without_all() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("pageSize", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(FIRST_PAGE_RESPONSE))
            .expect(1)
            .mount(&server)
            .await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .and(query_param("pageSize", "1"))
            .and(query_param("pageToken", "token-2"))
            .respond_with(ResponseTemplate::new(200).set_body_string(SECOND_PAGE_RESPONSE))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let files_url = format!("{}/drive/v3/files", server.uri());

        run_list_to(
            &client,
            Some(2),
            false,
            false,
            false,
            &mut out,
            &mut err,
            Some(&files_url),
        )
        .await
        .unwrap();

        let rendered = String::from_utf8(out).unwrap();
        assert!(rendered.contains("First\tfile-1\ttext/plain"));
        assert!(rendered.contains("Second\tfile-2\ttext/plain"));
        assert!(err.is_empty());
    }

    #[tokio::test]
    async fn run_list_returns_clear_error_for_not_found_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(404).set_body_string("not found"))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let files_url = format!("{}/drive/v3/files", server.uri());

        let result = run_list_to(
            &client,
            None,
            false,
            false,
            false,
            &mut out,
            &mut err,
            Some(&files_url),
        )
        .await;

        let message = format!("{:#}", result.unwrap_err());
        assert!(message.contains("failed to list Google Drive files"));
        assert!(message.contains("Google Drive resource was not found"));
        assert!(out.is_empty());
    }

    #[tokio::test]
    async fn run_list_returns_clear_error_for_permission_denied_response() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/drive/v3/files"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .expect(1)
            .mount(&server)
            .await;

        let store = MemoryStore::default();
        let client = test_client(&store);
        let mut out = Vec::new();
        let mut err = Vec::new();
        let files_url = format!("{}/drive/v3/files", server.uri());

        let result = run_list_to(
            &client,
            None,
            false,
            false,
            false,
            &mut out,
            &mut err,
            Some(&files_url),
        )
        .await;

        let message = format!("{:#}", result.unwrap_err());
        assert!(message.contains("failed to list Google Drive files"));
        assert!(message.contains("Google Drive permission denied"));
        assert!(out.is_empty());
    }
}
