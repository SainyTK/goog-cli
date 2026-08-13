use std::io::Cursor;

use bytes::Bytes;
use image::{DynamicImage, ImageFormat, Rgba, RgbaImage};

use super::artifacts::create_montage;
use crate::slides::{PageThumbnail, DEFAULT_MAX_THUMBNAIL_BYTES};

fn solid_thumbnail(color: Rgba<u8>) -> PageThumbnail {
    solid_thumbnail_with_dimensions(16, 9, color)
}

fn solid_thumbnail_with_dimensions(width: u32, height: u32, color: Rgba<u8>) -> PageThumbnail {
    let image = RgbaImage::from_pixel(width, height, color);
    let mut bytes = Cursor::new(Vec::new());
    DynamicImage::ImageRgba8(image)
        .write_to(&mut bytes, ImageFormat::Png)
        .unwrap();

    PageThumbnail {
        width,
        height,
        bytes: Bytes::from(bytes.into_inner()),
    }
}

#[test]
fn montage_preserves_source_order_in_a_deterministic_png_grid() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    let repeated_output = temp.path().join("montage-repeated.png");
    let thumbnails = vec![
        solid_thumbnail(Rgba([220, 20, 20, 255])),
        solid_thumbnail(Rgba([20, 180, 20, 255])),
    ];

    let artifact = create_montage(&thumbnails, &output).unwrap();

    assert_eq!(artifact.path, output);
    assert_eq!(artifact.slide_count, 2);
    assert_eq!(artifact.columns, 2);
    assert_eq!(artifact.rows, 1);
    let rendered = image::open(&artifact.path).unwrap().to_rgba8();
    let red_x = rendered
        .enumerate_pixels()
        .find(|(_, _, pixel)| **pixel == Rgba([220, 20, 20, 255]))
        .map(|(x, _, _)| x)
        .unwrap();
    let green_x = rendered
        .enumerate_pixels()
        .find(|(_, _, pixel)| **pixel == Rgba([20, 180, 20, 255]))
        .map(|(x, _, _)| x)
        .unwrap();
    assert!(red_x < green_x);
    assert!(std::fs::read(&artifact.path)
        .unwrap()
        .starts_with(b"\x89PNG\r\n\x1a\n"));
    create_montage(&thumbnails, &repeated_output).unwrap();
    assert_eq!(
        std::fs::read(&artifact.path).unwrap(),
        std::fs::read(repeated_output).unwrap()
    );
}

#[test]
fn montage_rejects_invalid_png_data_without_replacing_an_existing_artifact() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    std::fs::write(&output, b"previous montage").unwrap();
    let thumbnails = vec![PageThumbnail {
        width: 16,
        height: 9,
        bytes: Bytes::from_static(b"not a PNG"),
    }];

    let error = create_montage(&thumbnails, &output).unwrap_err();

    assert!(error.to_string().contains("invalid slide thumbnail PNG"));
    assert_eq!(std::fs::read(&output).unwrap(), b"previous montage");
    assert_eq!(std::fs::read_dir(temp.path()).unwrap().count(), 1);
}

#[test]
fn montage_applies_metadata_dimensions_as_strict_decode_limits() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    let mut thumbnail = solid_thumbnail(Rgba([20, 20, 220, 255]));
    thumbnail.width = 1;
    thumbnail.height = 1;

    let error = create_montage(&[thumbnail], &output).unwrap_err();

    assert!(error.to_string().contains("invalid slide thumbnail PNG"));
    assert!(!output.exists());
}

#[test]
fn montage_uses_a_four_by_four_ordered_grid_for_the_benchmark_deck() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    let thumbnails = (1..=14)
        .map(|slide| solid_thumbnail(Rgba([slide * 10, 80, 160, 255])))
        .collect::<Vec<_>>();

    let artifact = create_montage(&thumbnails, &output).unwrap();

    assert_eq!(artifact.slide_count, 14);
    assert_eq!(artifact.columns, 4);
    assert_eq!(artifact.rows, 4);
    let rendered = image::open(output).unwrap().to_rgba8();
    let positions = (1..=14)
        .map(|slide| {
            rendered
                .enumerate_pixels()
                .find(|(_, _, pixel)| **pixel == Rgba([slide * 10, 80, 160, 255]))
                .map(|(x, y, _)| (y, x))
                .unwrap()
        })
        .collect::<Vec<_>>();
    assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
}

#[test]
fn montage_rejects_an_empty_thumbnail_set() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");

    let error = create_montage(&[], &output).unwrap_err();

    assert!(error.to_string().contains("without slide thumbnails"));
    assert!(!output.exists());
}

#[test]
fn montage_adds_a_slide_number_label_below_each_thumbnail() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    let thumbnail = solid_thumbnail(Rgba([255, 255, 255, 255]));

    create_montage(&[thumbnail], &output).unwrap();

    let rendered = image::open(output).unwrap().to_rgba8();
    let thumbnail_bottom = rendered
        .enumerate_pixels()
        .filter(|(_, _, pixel)| **pixel == Rgba([255, 255, 255, 255]))
        .map(|(_, y, _)| y)
        .max()
        .unwrap();
    assert!(rendered
        .enumerate_pixels()
        .any(|(_, y, pixel)| { y > thumbnail_bottom && *pixel == Rgba([43, 48, 56, 255]) }));
}

#[test]
fn montage_downsamples_large_thumbnails_to_a_compact_cell() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    let thumbnail = solid_thumbnail_with_dimensions(800, 450, Rgba([80, 120, 200, 255]));

    let artifact = create_montage(&[thumbnail], &output).unwrap();

    assert!(artifact.width < 800);
    assert!(artifact.height < 450);
    let rendered = image::open(output).unwrap().to_rgba8();
    assert!(rendered
        .pixels()
        .any(|pixel| *pixel == Rgba([80, 120, 200, 255])));
}

#[test]
fn montage_rechecks_the_thumbnail_byte_limit_before_decoding() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    let thumbnail = PageThumbnail {
        width: 1,
        height: 1,
        bytes: Bytes::from(vec![0_u8; DEFAULT_MAX_THUMBNAIL_BYTES + 1]),
    };

    let error = create_montage(&[thumbnail], &output).unwrap_err();

    assert!(error.to_string().contains("byte decode limit"));
    assert!(!output.exists());
}

#[test]
fn montage_rechecks_the_thumbnail_pixel_limit_before_decoding() {
    let temp = tempfile::tempdir().unwrap();
    let output = temp.path().join("montage.png");
    let thumbnail = PageThumbnail {
        width: 4_001,
        height: 4_000,
        bytes: Bytes::new(),
    };

    let error = create_montage(&[thumbnail], &output).unwrap_err();

    assert!(error.to_string().contains("supported decode limit"));
    assert!(!output.exists());
}
