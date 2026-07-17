use std::io::Cursor;

use image::{DynamicImage, ImageFormat, RgbaImage};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::image_metadata::{dimensions_from_prefix, inspect_remote_image_dimensions};

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
