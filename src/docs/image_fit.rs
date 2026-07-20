//! Aspect-ratio-preserving sizing for Google Docs images.
//!
//! Source pixels use the CSS conversion of 96 pixels per inch and Docs sizes use
//! 72 points per inch. Embedded physical-resolution metadata does not affect the
//! calculation. Reported point dimensions and scale factors are rounded to three
//! decimal places so repeated runs produce stable request JSON.

use anyhow::{bail, Result};

const POINTS_PER_PIXEL: f64 = 72.0 / 96.0;
const ROUNDING_FACTOR: f64 = 1_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SourceImageDimensions {
    pub width_px: u32,
    pub height_px: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ImageFitConstraints {
    pub max_width_pt: Option<f64>,
    pub max_height_pt: Option<f64>,
    pub allow_upscale: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ResolvedImageSize {
    pub native_width_pt: f64,
    pub native_height_pt: f64,
    pub width_pt: f64,
    pub height_pt: f64,
    pub scale: f64,
    pub upscaled: bool,
}

pub(crate) fn fit_image(
    source: SourceImageDimensions,
    constraints: ImageFitConstraints,
) -> Result<ResolvedImageSize> {
    if source.width_px == 0 || source.height_px == 0 {
        bail!("source image width and height must be greater than zero");
    }
    if constraints.max_width_pt.is_none() && constraints.max_height_pt.is_none() {
        bail!("at least one maximum image dimension must be provided");
    }
    for (name, value) in [
        ("maximum image width", constraints.max_width_pt),
        ("maximum image height", constraints.max_height_pt),
    ] {
        if let Some(value) = value {
            if !value.is_finite() || value <= 0.0 {
                bail!("{name} must be finite and greater than zero");
            }
        }
    }

    let native_width_pt = f64::from(source.width_px) * POINTS_PER_PIXEL;
    let native_height_pt = f64::from(source.height_px) * POINTS_PER_PIXEL;
    let width_scale = constraints
        .max_width_pt
        .map(|maximum| maximum / native_width_pt)
        .unwrap_or(f64::INFINITY);
    let height_scale = constraints
        .max_height_pt
        .map(|maximum| maximum / native_height_pt)
        .unwrap_or(f64::INFINITY);
    let mut scale = width_scale.min(height_scale);
    if !constraints.allow_upscale {
        scale = scale.min(1.0);
    }

    Ok(ResolvedImageSize {
        native_width_pt: round_output(native_width_pt),
        native_height_pt: round_output(native_height_pt),
        width_pt: round_output(native_width_pt * scale),
        height_pt: round_output(native_height_pt * scale),
        scale: round_output(scale),
        upscaled: scale > 1.0,
    })
}

pub(crate) fn exact_size_preserves_aspect_ratio(
    source: SourceImageDimensions,
    width_pt: f64,
    height_pt: f64,
) -> bool {
    let width_from_height =
        round_output(height_pt * f64::from(source.width_px) / f64::from(source.height_px));
    let height_from_width =
        round_output(width_pt * f64::from(source.height_px) / f64::from(source.width_px));
    width_pt == width_from_height || height_pt == height_from_width
}

fn round_output(value: f64) -> f64 {
    (value * ROUNDING_FACTOR).round() / ROUNDING_FACTOR
}

#[cfg(test)]
mod tests {
    use super::{fit_image, ImageFitConstraints, SourceImageDimensions};

    #[test]
    fn portrait_image_fits_inside_both_maximum_dimensions() {
        let fit = fit_image(
            SourceImageDimensions {
                width_px: 1_440,
                height_px: 2_534,
            },
            ImageFitConstraints {
                max_width_pt: Some(468.0),
                max_height_pt: Some(500.0),
                allow_upscale: false,
            },
        )
        .unwrap();

        assert_eq!(fit.width_pt, 284.136);
        assert_eq!(fit.height_pt, 500.0);
        assert_eq!(fit.scale, 0.263);
        assert!(!fit.upscaled);
    }

    #[test]
    fn landscape_image_fits_inside_both_maximum_dimensions() {
        let fit = fit_image(
            SourceImageDimensions {
                width_px: 1_440,
                height_px: 1_047,
            },
            ImageFitConstraints {
                max_width_pt: Some(468.0),
                max_height_pt: Some(500.0),
                allow_upscale: false,
            },
        )
        .unwrap();

        assert_eq!(fit.native_width_pt, 1_080.0);
        assert_eq!(fit.native_height_pt, 785.25);
        assert_eq!(fit.width_pt, 468.0);
        assert_eq!(fit.height_pt, 340.275);
        assert_eq!(fit.scale, 0.433);
        assert!(!fit.upscaled);
    }

    #[test]
    fn small_image_is_only_upscaled_when_allowed() {
        let source = SourceImageDimensions {
            width_px: 400,
            height_px: 200,
        };
        let constraints = ImageFitConstraints {
            max_width_pt: Some(468.0),
            max_height_pt: Some(500.0),
            allow_upscale: false,
        };

        let natural = fit_image(source, constraints).unwrap();
        assert_eq!((natural.width_pt, natural.height_pt), (300.0, 150.0));
        assert_eq!(natural.scale, 1.0);
        assert!(!natural.upscaled);

        let enlarged = fit_image(
            source,
            ImageFitConstraints {
                allow_upscale: true,
                ..constraints
            },
        )
        .unwrap();
        assert_eq!((enlarged.width_pt, enlarged.height_pt), (468.0, 234.0));
        assert_eq!(enlarged.scale, 1.56);
        assert!(enlarged.upscaled);
    }

    #[test]
    fn one_maximum_dimension_constrains_the_other() {
        let fit = fit_image(
            SourceImageDimensions {
                width_px: 100,
                height_px: 200,
            },
            ImageFitConstraints {
                max_width_pt: None,
                max_height_pt: Some(75.0),
                allow_upscale: false,
            },
        )
        .unwrap();

        assert_eq!((fit.width_pt, fit.height_pt), (37.5, 75.0));
        assert_eq!(fit.scale, 0.5);
    }

    #[test]
    fn fitting_rejects_missing_or_invalid_dimensions() {
        let source = SourceImageDimensions {
            width_px: 1_440,
            height_px: 1_047,
        };

        for constraints in [
            ImageFitConstraints {
                max_width_pt: None,
                max_height_pt: None,
                allow_upscale: false,
            },
            ImageFitConstraints {
                max_width_pt: Some(0.0),
                max_height_pt: None,
                allow_upscale: false,
            },
            ImageFitConstraints {
                max_width_pt: Some(f64::NAN),
                max_height_pt: None,
                allow_upscale: false,
            },
            ImageFitConstraints {
                max_width_pt: None,
                max_height_pt: Some(f64::INFINITY),
                allow_upscale: false,
            },
        ] {
            assert!(fit_image(source, constraints).is_err());
        }

        assert!(fit_image(
            SourceImageDimensions {
                width_px: 0,
                height_px: 1_047,
            },
            ImageFitConstraints {
                max_width_pt: Some(468.0),
                max_height_pt: None,
                allow_upscale: false,
            },
        )
        .is_err());
    }
}
