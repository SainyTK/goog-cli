use std::io::{BufReader, Cursor, Write};
use std::path::{Path, PathBuf};

use image::imageops::{replace, FilterType};
use image::{DynamicImage, ImageFormat, ImageReader, Limits, Rgba, RgbaImage};

use crate::slides::{
    PageThumbnail, SlidesError, DEFAULT_MAX_THUMBNAIL_BYTES, DEFAULT_MAX_THUMBNAIL_PIXELS,
};

const CELL_IMAGE_WIDTH: u32 = 400;
const CELL_IMAGE_HEIGHT: u32 = 225;
const LABEL_HEIGHT: u32 = 32;
const GRID_GAP: u32 = 16;
const OUTER_PADDING: u32 = 16;
const MAX_MONTAGE_PIXELS: u64 = 40_000_000;
const DIGIT_SCALE: u32 = 3;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MontageArtifact {
    pub path: PathBuf,
    pub width: u32,
    pub height: u32,
    pub slide_count: usize,
    pub columns: u32,
    pub rows: u32,
}

pub fn create_montage(
    thumbnails: &[PageThumbnail],
    output: impl AsRef<Path>,
) -> Result<MontageArtifact, SlidesError> {
    if thumbnails.is_empty() {
        return Err(SlidesError::Artifact(
            "cannot build a montage without slide thumbnails".into(),
        ));
    }

    let slide_count = u32::try_from(thumbnails.len())
        .map_err(|_| SlidesError::Artifact("montage slide count is too large".into()))?;
    let columns = square_grid_columns(slide_count);
    let rows = slide_count.div_ceil(columns);
    let cell_height = CELL_IMAGE_HEIGHT + LABEL_HEIGHT;
    let width = grid_extent(columns, CELL_IMAGE_WIDTH)?;
    let height = grid_extent(rows, cell_height)?;
    let montage_pixels = u64::from(width) * u64::from(height);
    if montage_pixels > MAX_MONTAGE_PIXELS {
        return Err(SlidesError::Artifact(format!(
            "montage dimensions {width}x{height} exceed the supported pixel limit"
        )));
    }

    let mut canvas = RgbaImage::from_pixel(width, height, Rgba([238, 240, 243, 255]));
    for (index, thumbnail) in thumbnails.iter().enumerate() {
        let decoded = decode_thumbnail(thumbnail)?;
        let rendered = if decoded.width() > CELL_IMAGE_WIDTH || decoded.height() > CELL_IMAGE_HEIGHT
        {
            decoded.resize(CELL_IMAGE_WIDTH, CELL_IMAGE_HEIGHT, FilterType::Lanczos3)
        } else {
            decoded
        };
        let column = u32::try_from(index).expect("slide count fits u32") % columns;
        let row = u32::try_from(index).expect("slide count fits u32") / columns;
        let cell_x = OUTER_PADDING + column * (CELL_IMAGE_WIDTH + GRID_GAP);
        let cell_y = OUTER_PADDING + row * (cell_height + GRID_GAP);
        let image_x = cell_x + (CELL_IMAGE_WIDTH - rendered.width()) / 2;
        let image_y = cell_y + (CELL_IMAGE_HEIGHT - rendered.height()) / 2;
        replace(
            &mut canvas,
            &rendered.to_rgba8(),
            i64::from(image_x),
            i64::from(image_y),
        );
        draw_slide_number(
            &mut canvas,
            u32::try_from(index + 1).expect("slide count fits u32"),
            cell_x + 8,
            cell_y + CELL_IMAGE_HEIGHT + 5,
        );
    }

    let output = output.as_ref();
    let output_parent = output
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let mut temporary_file =
        tempfile::NamedTempFile::new_in(output_parent).map_err(SlidesError::ArtifactIo)?;
    DynamicImage::ImageRgba8(canvas)
        .write_to(&mut temporary_file, ImageFormat::Png)
        .map_err(|error| SlidesError::Artifact(error.to_string()))?;
    temporary_file.flush().map_err(SlidesError::ArtifactIo)?;
    temporary_file
        .as_file()
        .sync_all()
        .map_err(SlidesError::ArtifactIo)?;
    temporary_file
        .persist(output)
        .map_err(|error| SlidesError::ArtifactIo(error.error))?;

    Ok(MontageArtifact {
        path: output.to_path_buf(),
        width,
        height,
        slide_count: thumbnails.len(),
        columns,
        rows,
    })
}

fn square_grid_columns(slide_count: u32) -> u32 {
    let mut columns = 1_u32;
    while columns.saturating_mul(columns) < slide_count {
        columns += 1;
    }
    columns
}

fn grid_extent(cell_count: u32, cell_extent: u32) -> Result<u32, SlidesError> {
    OUTER_PADDING
        .checked_mul(2)
        .and_then(|padding| {
            cell_count
                .checked_mul(cell_extent)
                .and_then(|cells| padding.checked_add(cells))
        })
        .and_then(|extent| {
            cell_count
                .saturating_sub(1)
                .checked_mul(GRID_GAP)
                .and_then(|gaps| extent.checked_add(gaps))
        })
        .ok_or_else(|| SlidesError::Artifact("montage dimensions overflow".into()))
}

fn decode_thumbnail(thumbnail: &PageThumbnail) -> Result<DynamicImage, SlidesError> {
    if thumbnail.bytes.len() > DEFAULT_MAX_THUMBNAIL_BYTES {
        return Err(SlidesError::Artifact(format!(
            "slide thumbnail exceeds the {DEFAULT_MAX_THUMBNAIL_BYTES}-byte decode limit"
        )));
    }
    let pixel_count = u64::from(thumbnail.width)
        .checked_mul(u64::from(thumbnail.height))
        .ok_or_else(|| SlidesError::Artifact("thumbnail dimensions overflow".into()))?;
    if thumbnail.width == 0 || thumbnail.height == 0 || pixel_count > DEFAULT_MAX_THUMBNAIL_PIXELS {
        return Err(SlidesError::Artifact(format!(
            "slide thumbnail dimensions {}x{} exceed the supported decode limit",
            thumbnail.width, thumbnail.height
        )));
    }
    let max_alloc = pixel_count
        .checked_mul(8)
        .ok_or_else(|| SlidesError::Artifact("thumbnail dimensions overflow".into()))?;
    let mut limits = Limits::default();
    limits.max_image_width = Some(thumbnail.width);
    limits.max_image_height = Some(thumbnail.height);
    limits.max_alloc = Some(max_alloc);
    let mut reader = ImageReader::with_format(
        BufReader::new(Cursor::new(thumbnail.bytes.as_ref())),
        ImageFormat::Png,
    );
    reader.limits(limits);
    let decoded = reader
        .decode()
        .map_err(|error| SlidesError::Artifact(format!("invalid slide thumbnail PNG: {error}")))?;
    if decoded.width() != thumbnail.width || decoded.height() != thumbnail.height {
        return Err(SlidesError::Artifact(format!(
            "slide thumbnail dimensions do not match metadata: expected {}x{}, decoded {}x{}",
            thumbnail.width,
            thumbnail.height,
            decoded.width(),
            decoded.height()
        )));
    }
    Ok(decoded)
}

fn draw_slide_number(canvas: &mut RgbaImage, number: u32, x: u32, y: u32) {
    let digits = number.to_string();
    for (position, digit) in digits.bytes().enumerate() {
        let glyph = DIGITS[usize::from(digit - b'0')];
        let digit_x = x + u32::try_from(position).expect("digit position fits u32") * 18;
        for (glyph_y, row) in glyph.iter().copied().enumerate() {
            for glyph_x in 0..5_u32 {
                if row & (1 << (4 - glyph_x)) == 0 {
                    continue;
                }
                for offset_y in 0..DIGIT_SCALE {
                    for offset_x in 0..DIGIT_SCALE {
                        let pixel_x = digit_x + glyph_x * DIGIT_SCALE + offset_x;
                        let pixel_y = y
                            + u32::try_from(glyph_y).expect("glyph row fits u32") * DIGIT_SCALE
                            + offset_y;
                        if let Some(pixel) = canvas.get_pixel_mut_checked(pixel_x, pixel_y) {
                            *pixel = Rgba([43, 48, 56, 255]);
                        }
                    }
                }
            }
        }
    }
}

const DIGITS: [[u8; 7]; 10] = [
    [
        0b01110, 0b10001, 0b10011, 0b10101, 0b11001, 0b10001, 0b01110,
    ],
    [
        0b00100, 0b01100, 0b00100, 0b00100, 0b00100, 0b00100, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b00001, 0b00010, 0b00100, 0b01000, 0b11111,
    ],
    [
        0b11110, 0b00001, 0b00001, 0b01110, 0b00001, 0b00001, 0b11110,
    ],
    [
        0b00010, 0b00110, 0b01010, 0b10010, 0b11111, 0b00010, 0b00010,
    ],
    [
        0b11111, 0b10000, 0b10000, 0b11110, 0b00001, 0b00001, 0b11110,
    ],
    [
        0b01110, 0b10000, 0b10000, 0b11110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b11111, 0b00001, 0b00010, 0b00100, 0b01000, 0b01000, 0b01000,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01110, 0b10001, 0b10001, 0b01110,
    ],
    [
        0b01110, 0b10001, 0b10001, 0b01111, 0b00001, 0b00001, 0b01110,
    ],
];
