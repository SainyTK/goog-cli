use super::image_fit::{exact_size_preserves_aspect_ratio, SourceImageDimensions};

#[test]
fn exact_size_only_accepts_the_three_decimal_aspect_preserving_boundary() {
    let source = SourceImageDimensions {
        width_px: 1_440,
        height_px: 2_534,
    };

    assert!(exact_size_preserves_aspect_ratio(source, 284.136, 500.0));
    assert!(!exact_size_preserves_aspect_ratio(source, 284.135, 500.0));
    assert!(!exact_size_preserves_aspect_ratio(source, 284.0, 500.0));
}
