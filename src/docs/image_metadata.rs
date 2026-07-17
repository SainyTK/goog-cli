use std::io::Cursor;

use anyhow::{bail, Context, Result};
use futures_util::StreamExt;
use image::ImageReader;

use crate::docs::image_fit::SourceImageDimensions;

const MAX_METADATA_BYTES: usize = 8 * 1024 * 1024;
const MAX_SOURCE_DIMENSION_PX: u32 = 100_000;

pub(crate) async fn inspect_remote_image_dimensions(
    image_uri: &str,
) -> Result<SourceImageDimensions> {
    let parsed =
        url::Url::parse(image_uri).context("image URI must be a valid HTTP or HTTPS URL")?;
    if !matches!(parsed.scheme(), "http" | "https") {
        bail!("image URI must use HTTP or HTTPS for Google Docs insertion");
    }

    let response = reqwest::Client::new()
        .get(parsed)
        .send()
        .await
        .with_context(|| format!("failed to fetch image metadata from {image_uri}"))?
        .error_for_status()
        .with_context(|| format!("image metadata request failed for {image_uri}"))?;
    let mut stream = response.bytes_stream();
    let mut prefix = Vec::new();

    while let Some(chunk) = stream.next().await {
        let chunk =
            chunk.with_context(|| format!("failed to read image metadata from {image_uri}"))?;
        let remaining = MAX_METADATA_BYTES.saturating_sub(prefix.len());
        prefix.extend_from_slice(&chunk[..chunk.len().min(remaining)]);

        if let Some(dimensions) = dimensions_from_prefix(&prefix)? {
            return Ok(dimensions);
        }
        if prefix.len() == MAX_METADATA_BYTES {
            bail!(
                "image dimensions were not found within the first {MAX_METADATA_BYTES} bytes of {image_uri}"
            );
        }
    }

    dimensions_from_prefix(&prefix)?.with_context(|| {
        format!("could not read PNG, JPEG, GIF, or WebP dimensions from {image_uri}")
    })
}

pub(super) fn dimensions_from_prefix(bytes: &[u8]) -> Result<Option<SourceImageDimensions>> {
    if bytes.len() < 16 {
        return Ok(None);
    }
    let Ok(reader) = ImageReader::new(Cursor::new(bytes)).with_guessed_format() else {
        return Ok(None);
    };
    let Some(format) = reader.format() else {
        return Ok(None);
    };
    if !matches!(
        format,
        image::ImageFormat::Png
            | image::ImageFormat::Jpeg
            | image::ImageFormat::Gif
            | image::ImageFormat::WebP
    ) {
        bail!("image must be PNG, JPEG, GIF, or WebP");
    }
    let Ok((width_px, height_px)) = reader.into_dimensions() else {
        return Ok(None);
    };
    let (width_px, height_px) = displayed_dimensions(format, bytes, width_px, height_px);
    if width_px == 0 || height_px == 0 {
        bail!("source image width and height must be greater than zero");
    }
    if width_px > MAX_SOURCE_DIMENSION_PX || height_px > MAX_SOURCE_DIMENSION_PX {
        bail!(
            "source image dimensions exceed the supported maximum of {MAX_SOURCE_DIMENSION_PX} pixels"
        );
    }
    Ok(Some(SourceImageDimensions {
        width_px,
        height_px,
    }))
}

fn displayed_dimensions(
    format: image::ImageFormat,
    bytes: &[u8],
    width_px: u32,
    height_px: u32,
) -> (u32, u32) {
    if format != image::ImageFormat::Jpeg {
        return (width_px, height_px);
    }
    let Ok(metadata) = exif::Reader::new().read_from_container(&mut Cursor::new(bytes)) else {
        return (width_px, height_px);
    };
    let orientation = metadata
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|field| field.value.get_uint(0));
    if matches!(orientation, Some(5..=8)) {
        (height_px, width_px)
    } else {
        (width_px, height_px)
    }
}
