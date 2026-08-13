use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use url::Url;

const MAX_ADAPTER_OUTPUT_BYTES: usize = 1024 * 1024;

#[derive(Debug, Serialize)]
#[serde(tag = "action", rename_all = "camelCase")]
enum AdapterRequest<'a> {
    Stage {
        path: &'a Path,
        #[serde(rename = "mimeType")]
        mime_type: &'a str,
    },
    Cleanup {
        #[serde(rename = "cleanupToken")]
        cleanup_token: &'a str,
    },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AdapterStageResponse {
    uri: String,
    cleanup_token: Option<String>,
    expires_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StagedImage {
    pub uri: String,
    pub cleanup_token: Option<String>,
    pub expires_at: Option<String>,
}

pub(crate) fn stage_with_command(
    command: &Path,
    image_path: &Path,
    mime_type: &str,
) -> Result<StagedImage> {
    let absolute_path = image_path.canonicalize().with_context(|| {
        format!(
            "failed to resolve local image file: {}",
            image_path.display()
        )
    })?;
    let output = invoke_adapter(
        command,
        &AdapterRequest::Stage {
            path: &absolute_path,
            mime_type,
        },
    )?;
    ensure_adapter_succeeded(command, "stage", &output)?;
    ensure_bounded_output(command, &output)?;
    let response: AdapterStageResponse = serde_json::from_slice(&output.stdout)
        .context("staging command returned invalid stage JSON")?;
    validate_public_https_uri(&response.uri)?;
    if response.cleanup_token.as_deref() == Some("") {
        bail!("staging command returned an empty cleanupToken");
    }
    Ok(StagedImage {
        uri: response.uri,
        cleanup_token: response.cleanup_token,
        expires_at: response.expires_at,
    })
}

pub(crate) fn cleanup_with_command(command: &Path, cleanup_token: &str) -> Result<()> {
    let output = invoke_adapter(command, &AdapterRequest::Cleanup { cleanup_token })?;
    ensure_adapter_succeeded(command, "cleanup", &output)?;
    ensure_bounded_output(command, &output)
}

fn invoke_adapter(command: &Path, request: &AdapterRequest<'_>) -> Result<Output> {
    let mut child = Command::new(command)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .with_context(|| format!("failed to start staging command: {}", command.display()))?;
    let mut stdin = child
        .stdin
        .take()
        .context("failed to open staging command stdin")?;
    serde_json::to_writer(&mut stdin, request)
        .context("failed to encode staging command request")?;
    stdin
        .write_all(b"\n")
        .context("failed to write staging command request")?;
    drop(stdin);
    child
        .wait_with_output()
        .context("failed to wait for staging command")
}

fn ensure_adapter_succeeded(command: &Path, action: &str, output: &Output) -> Result<()> {
    if output.status.success() {
        return Ok(());
    }
    bail!(
        "staging command {} failed during {action} with status {}",
        command.display(),
        output.status
    )
}

fn ensure_bounded_output(command: &Path, output: &Output) -> Result<()> {
    if output.stdout.len() > MAX_ADAPTER_OUTPUT_BYTES
        || output.stderr.len() > MAX_ADAPTER_OUTPUT_BYTES
    {
        bail!(
            "staging command {} output exceeded {} bytes",
            command.display(),
            MAX_ADAPTER_OUTPUT_BYTES
        );
    }
    Ok(())
}

fn validate_public_https_uri(uri: &str) -> Result<()> {
    let parsed = Url::parse(uri).context("staging command returned an invalid uri")?;
    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        bail!("staging command uri must be an absolute HTTPS URL");
    }
    if !parsed.username().is_empty() || parsed.password().is_some() {
        bail!("staging command uri must not contain user credentials");
    }
    Ok(())
}

pub(crate) fn safe_command_path(command: &Path) -> PathBuf {
    command
        .canonicalize()
        .unwrap_or_else(|_| command.to_path_buf())
}
