use std::io::Cursor;

use image::{DynamicImage, ImageFormat, RgbaImage};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::image_metadata::{
    dimensions_from_prefix, inspect_local_image, inspect_remote_image_dimensions,
};

#[test]
fn reads_dimensions_from_all_supported_image_formats() {
    for format in [
        ImageFormat::Png,
        ImageFormat::Jpeg,
        ImageFormat::Gif,
        ImageFormat::WebP,
    ] {
        let mut encoded = Vec::new();
        DynamicImage::ImageRgba8(RgbaImage::new(32, 24))
            .write_to(&mut Cursor::new(&mut encoded), format)
            .unwrap();

        let dimensions = dimensions_from_prefix(&encoded).unwrap().unwrap();
        assert_eq!((dimensions.width_px, dimensions.height_px), (32, 24));
    }
}

#[test]
fn jpeg_dimensions_follow_exif_display_orientation() {
    let mut jpeg = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::new(32, 24))
        .write_to(&mut Cursor::new(&mut jpeg), ImageFormat::Jpeg)
        .unwrap();
    let exif_orientation_six = [
        0xff, 0xe1, 0x00, 0x22, b'E', b'x', b'i', b'f', 0x00, 0x00, b'I', b'I', 0x2a, 0x00, 0x08,
        0x00, 0x00, 0x00, 0x01, 0x00, 0x12, 0x01, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x06, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    ];
    jpeg.splice(2..2, exif_orientation_six);

    let dimensions = dimensions_from_prefix(&jpeg).unwrap().unwrap();

    assert_eq!((dimensions.width_px, dimensions.height_px), (24, 32));
}

#[tokio::test]
async fn reads_png_dimensions_from_a_remote_image_without_decoding_pixels() {
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::new(1_440, 2_534))
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap();
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/portrait.png"))
        .respond_with(ResponseTemplate::new(200).set_body_bytes(png))
        .mount(&server)
        .await;

    let dimensions = inspect_remote_image_dimensions(&format!("{}/portrait.png", server.uri()))
        .await
        .unwrap();

    assert_eq!(dimensions.width_px, 1_440);
    assert_eq!(dimensions.height_px, 2_534);
}

#[test]
fn validates_local_png_from_content_and_returns_staging_metadata() {
    let temp_dir = tempfile::tempdir().unwrap();
    let image_path = temp_dir.path().join("dashboard.dat");
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::new(32, 24))
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .unwrap();
    std::fs::write(&image_path, &png).unwrap();

    let inspected = inspect_local_image(&image_path).unwrap();

    assert_eq!(inspected.path, image_path.canonicalize().unwrap());
    assert_eq!(inspected.mime_type, "image/png");
    assert_eq!(inspected.size_bytes, png.len() as u64);
    assert_eq!(inspected.dimensions.width_px, 32);
    assert_eq!(inspected.dimensions.height_px, 24);
}

#[test]
fn rejects_unsupported_local_file_content() {
    let temp_dir = tempfile::tempdir().unwrap();
    let image_path = temp_dir.path().join("not-an-image.png");
    std::fs::write(&image_path, b"plain text").unwrap();

    let error = inspect_local_image(&image_path).unwrap_err();

    assert!(error
        .to_string()
        .contains("local image format is unsupported"));
}

#[test]
fn rejects_local_webp_even_when_image_metadata_is_readable() {
    let temp_dir = tempfile::tempdir().unwrap();
    let image_path = temp_dir.path().join("dashboard.webp");
    let mut webp = Vec::new();
    DynamicImage::ImageRgba8(RgbaImage::new(32, 24))
        .write_to(&mut Cursor::new(&mut webp), ImageFormat::WebP)
        .unwrap();
    std::fs::write(&image_path, webp).unwrap();

    let error = inspect_local_image(&image_path).unwrap_err();

    assert!(error
        .to_string()
        .contains("Google Docs images must be PNG, JPEG, or GIF"));
}

#[test]
fn rejects_missing_empty_and_oversized_local_files_before_inspection() {
    let temp_dir = tempfile::tempdir().unwrap();
    let missing = temp_dir.path().join("missing.png");
    assert!(inspect_local_image(&missing)
        .unwrap_err()
        .to_string()
        .contains("failed to resolve local image file"));

    let empty = temp_dir.path().join("empty.png");
    std::fs::write(&empty, []).unwrap();
    assert!(inspect_local_image(&empty)
        .unwrap_err()
        .to_string()
        .contains("local image file is empty"));

    let oversized = temp_dir.path().join("oversized.png");
    std::fs::File::create(&oversized)
        .unwrap()
        .set_len(50 * 1024 * 1024)
        .unwrap();
    assert!(inspect_local_image(&oversized)
        .unwrap_err()
        .to_string()
        .contains("smaller than 50 MB"));
}
