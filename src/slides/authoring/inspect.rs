use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;

use super::artifacts::create_montage;
use crate::auth::account::AccountStore;
use crate::auth::client::AuthClient;
use crate::drive::{
    export_google_file, DriveError, ExportGoogleFileOptions, GoogleFileExportFormat,
};
use crate::slides::{
    fetch_page_thumbnail_once, get_presentation, GetPageThumbnailOptions, GetPresentationOptions,
    SlidesError,
};

const INSPECT_REPORT_VERSION: u32 = 1;
const DEFAULT_CONSISTENCY_TIMEOUT: Duration = Duration::from_secs(30);
const INITIAL_THUMBNAIL_RETRY_DELAY: Duration = Duration::from_millis(250);
const MAX_THUMBNAIL_RETRY_DELAY: Duration = Duration::from_secs(2);

#[derive(Debug, Clone)]
pub struct InspectDeckRequest {
    pub presentation_id: String,
    pub qa_dir: PathBuf,
    pub export_pptx: Option<PathBuf>,
    pub export_pdf: Option<PathBuf>,
    pub consistency_timeout: Duration,
    #[cfg(test)]
    pub(crate) presentations_url: Option<String>,
    #[cfg(test)]
    pub(crate) drive_files_url: Option<String>,
}

impl InspectDeckRequest {
    pub fn new(presentation_id: impl Into<String>, qa_dir: impl Into<PathBuf>) -> Self {
        Self {
            presentation_id: presentation_id.into(),
            qa_dir: qa_dir.into(),
            export_pptx: None,
            export_pdf: None,
            consistency_timeout: DEFAULT_CONSISTENCY_TIMEOUT,
            #[cfg(test)]
            presentations_url: None,
            #[cfg(test)]
            drive_files_url: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectDeckReport {
    pub report_version: u32,
    pub result: &'static str,
    pub account: String,
    pub presentation_id: String,
    pub presentation_url: String,
    pub title: String,
    pub slides: Vec<InspectedSlide>,
    pub artifacts: InspectArtifacts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectedSlide {
    pub number: usize,
    pub object_id: String,
    pub element_count: usize,
    pub visible_text: Vec<String>,
    pub thumbnail: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InspectArtifacts {
    pub qa_dir: PathBuf,
    pub report: PathBuf,
    pub montage: PathBuf,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pptx: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pdf: Option<PathBuf>,
}

#[derive(Debug, thiserror::Error)]
pub enum InspectDeckError {
    #[error("failed to read Google Slides presentation: {0}")]
    Read(#[source] SlidesError),

    #[error("failed to fetch thumbnail for slide {slide_number}: {source}")]
    Thumbnail {
        slide_number: usize,
        #[source]
        source: SlidesError,
    },

    #[error("failed to create the Slides montage: {0}")]
    Montage(#[source] SlidesError),

    #[error("failed to export the Google Slides presentation: {0}")]
    Export(#[source] DriveError),

    #[error("failed to write the Slides inspection bundle: {0}")]
    Io(#[source] std::io::Error),

    #[error("failed to serialize the Slides inspection report: {0}")]
    Serialize(#[from] serde_json::Error),

    #[error("auth error: {0}")]
    Auth(#[from] crate::auth::error::AuthError),

    #[error("Google Slides presentation contains a slide without an object ID")]
    MissingSlideObjectId,
}

impl InspectDeckError {
    pub fn is_target_access_failure(&self) -> bool {
        matches!(
            self,
            Self::Read(SlidesError::NotFound | SlidesError::PermissionDenied)
                | Self::Thumbnail {
                    source: SlidesError::NotFound | SlidesError::PermissionDenied,
                    ..
                }
                | Self::Export(DriveError::NotFound | DriveError::PermissionDenied)
        )
    }
}

pub async fn inspect_deck<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: &InspectDeckRequest,
) -> Result<InspectDeckReport, InspectDeckError> {
    create_private_directory(&request.qa_dir)?;
    let thumbnails_dir = request.qa_dir.join("thumbnails");
    recreate_private_directory(&thumbnails_dir)?;
    remove_owned_file_if_exists(&request.qa_dir.join("inspect-report.json"))?;
    remove_owned_file_if_exists(&request.qa_dir.join("montage.png"))?;

    let get_options = GetPresentationOptions::new(&request.presentation_id);
    #[cfg(test)]
    let get_options = request
        .presentations_url
        .as_ref()
        .map_or(get_options.clone(), |url| {
            get_options.with_presentations_url(url)
        });
    let presentation = get_presentation(client, &get_options)
        .await
        .map_err(InspectDeckError::Read)?;

    let slide_values = presentation
        .get("slides")
        .and_then(serde_json::Value::as_array)
        .map(Vec::as_slice)
        .unwrap_or_default();
    let mut thumbnails = Vec::with_capacity(slide_values.len());
    let mut slides = Vec::with_capacity(slide_values.len());
    let consistency_deadline = tokio::time::Instant::now() + request.consistency_timeout;

    for (index, slide) in slide_values.iter().enumerate() {
        let slide_number = index + 1;
        let object_id = slide
            .get("objectId")
            .and_then(serde_json::Value::as_str)
            .ok_or(InspectDeckError::MissingSlideObjectId)?;
        let thumbnail_options = GetPageThumbnailOptions::new(&request.presentation_id, object_id);
        #[cfg(test)]
        let thumbnail_options =
            request
                .presentations_url
                .as_ref()
                .map_or(thumbnail_options.clone(), |url| {
                    thumbnail_options
                        .with_presentations_url(url)
                        .allow_insecure_content_url_for_tests()
                });
        let thumbnail =
            fetch_thumbnail_until_consistent(client, &thumbnail_options, consistency_deadline)
                .await
                .map_err(|source| InspectDeckError::Thumbnail {
                    slide_number,
                    source,
                })?;
        let thumbnail_path = thumbnails_dir.join(format!("slide-{slide_number:02}.png"));
        write_private_file(&thumbnail_path, &thumbnail.bytes)?;

        slides.push(InspectedSlide {
            number: slide_number,
            object_id: object_id.to_string(),
            element_count: slide
                .get("pageElements")
                .and_then(serde_json::Value::as_array)
                .map_or(0, Vec::len),
            visible_text: visible_text(slide),
            thumbnail: thumbnail_path,
        });
        thumbnails.push(thumbnail);
    }

    if thumbnails.is_empty() {
        return Err(InspectDeckError::Montage(SlidesError::Artifact(
            "cannot inspect a presentation without slides".into(),
        )));
    }

    let montage_path = request.qa_dir.join("montage.png");
    create_montage(&thumbnails, &montage_path).map_err(InspectDeckError::Montage)?;

    if let Some(output) = &request.export_pptx {
        export(client, request, output, GoogleFileExportFormat::PowerPoint).await?;
    }
    if let Some(output) = &request.export_pdf {
        export(client, request, output, GoogleFileExportFormat::Pdf).await?;
    }

    let report_path = request.qa_dir.join("inspect-report.json");
    let report = InspectDeckReport {
        report_version: INSPECT_REPORT_VERSION,
        result: "success",
        account: client.account_email().to_string(),
        presentation_id: request.presentation_id.clone(),
        presentation_url: format!(
            "https://docs.google.com/presentation/d/{}/edit",
            request.presentation_id
        ),
        title: presentation
            .get("title")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        slides,
        artifacts: InspectArtifacts {
            qa_dir: request.qa_dir.clone(),
            report: report_path.clone(),
            montage: montage_path,
            pptx: request.export_pptx.clone(),
            pdf: request.export_pdf.clone(),
        },
    };
    let report_json = serde_json::to_vec_pretty(&report)?;
    write_private_file(&report_path, &report_json)?;

    Ok(report)
}

async fn fetch_thumbnail_until_consistent<S: AccountStore>(
    client: &AuthClient<'_, S>,
    options: &GetPageThumbnailOptions,
    deadline: tokio::time::Instant,
) -> Result<crate::slides::PageThumbnail, SlidesError> {
    let mut retry_delay = INITIAL_THUMBNAIL_RETRY_DELAY;
    loop {
        match fetch_page_thumbnail_once(client, options).await {
            Ok(thumbnail) => return Ok(thumbnail),
            Err(SlidesError::NotFound) if tokio::time::Instant::now() < deadline => {
                let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
                tokio::time::sleep(retry_delay.min(remaining)).await;
                retry_delay = retry_delay
                    .checked_mul(2)
                    .unwrap_or(MAX_THUMBNAIL_RETRY_DELAY)
                    .min(MAX_THUMBNAIL_RETRY_DELAY);
            }
            Err(error) => return Err(error),
        }
    }
}

async fn export<S: AccountStore>(
    client: &AuthClient<'_, S>,
    request: &InspectDeckRequest,
    output: &Path,
    format: GoogleFileExportFormat,
) -> Result<(), InspectDeckError> {
    let options = ExportGoogleFileOptions::new(&request.presentation_id, format, output);
    #[cfg(test)]
    let options = request
        .drive_files_url
        .as_ref()
        .map_or(options.clone(), |url| options.with_files_url(url));
    export_google_file(client, &options, |_| {})
        .await
        .map_err(InspectDeckError::Export)?;
    Ok(())
}

fn create_private_directory(path: &Path) -> Result<(), InspectDeckError> {
    std::fs::create_dir_all(path).map_err(InspectDeckError::Io)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))
            .map_err(InspectDeckError::Io)?;
    }
    Ok(())
}

fn recreate_private_directory(path: &Path) -> Result<(), InspectDeckError> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
            std::fs::remove_file(path).map_err(InspectDeckError::Io)?;
        }
        Ok(_) => std::fs::remove_dir_all(path).map_err(InspectDeckError::Io)?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(InspectDeckError::Io(error)),
    }
    create_private_directory(path)
}

fn remove_owned_file_if_exists(path: &Path) -> Result<(), InspectDeckError> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(InspectDeckError::Io(error)),
    }
}

fn write_private_file(path: &Path, contents: &[u8]) -> Result<(), InspectDeckError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(InspectDeckError::Io)?;
    temporary
        .write_all(contents)
        .map_err(InspectDeckError::Io)?;
    temporary.flush().map_err(InspectDeckError::Io)?;
    temporary
        .as_file()
        .sync_all()
        .map_err(InspectDeckError::Io)?;
    temporary
        .persist(path)
        .map_err(|error| InspectDeckError::Io(error.error))?;
    Ok(())
}

fn visible_text(value: &serde_json::Value) -> Vec<String> {
    let mut text = Vec::new();
    collect_visible_text(value, &mut text);
    text
}

fn collect_visible_text(value: &serde_json::Value, text: &mut Vec<String>) {
    match value {
        serde_json::Value::Array(values) => {
            for value in values {
                collect_visible_text(value, text);
            }
        }
        serde_json::Value::Object(fields) => {
            if let Some(content) = fields
                .get("textRun")
                .and_then(|text_run| text_run.get("content"))
                .and_then(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|content| !content.is_empty())
            {
                text.push(content.to_string());
            }
            for (key, value) in fields {
                if key != "textRun" {
                    collect_visible_text(value, text);
                }
            }
        }
        _ => {}
    }
}
