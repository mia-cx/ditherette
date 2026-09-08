//! Borrowed field bindings sharing the existing processor and caught output helpers.

use super::{
    processor::{dimension, restore_ready, status, take_ready, JsBoundary},
    quantize::{parse_alpha, parse_matching, read_palette, JsQuantizeBoundary, PALETTE_SLOTS},
};
use crate::{
    image::contracts::PaletteEntry,
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
            request::{
                BayerSize, Diffusion, DiffusionFeedback, DitherPolicy, Field, PerturbPolicy,
                Placement, WorkingSpace, MAX_SOURCE_SIDE,
            },
        },
        pipeline::{perturb::PerturbRequest, quantize::QuantizeRequest},
    },
};
use js_sys::Uint8Array;
use wasm_bindgen::prelude::*;

/// Private field tags: Bayer 0, random 1. The field parameter is matrix width or the u32 seed.
/// All raw numbers enter as f64 so validation precedes any truncation or f32 narrowing.
#[wasm_bindgen(js_name = privatePerturb)]
pub fn private_perturb(
    input: &Uint8Array,
    width: f64,
    height: f64,
    field: f64,
    parameter: f64,
    space: f64,
    strength: f64,
    placement: f64,
    radius: f64,
    threshold: f64,
    softness: f64,
    result_sink: &JsValue,
) -> u32 {
    let mut processor = match take_ready() {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    let result = (|| {
        let (source_width, source_height) = source_dimensions(width, height)?;
        let perturb = parse_policy(
            field, parameter, space, strength, placement, radius, threshold, softness,
        )?;
        processor.perturb(
            PerturbRequest {
                source_width,
                source_height,
                perturb,
            },
            &mut JsBoundary { input, result_sink },
        )
    })();
    restore_ready(processor);
    result.map_or_else(status, |_| 0)
}

/// Families 0/1 select direct/separable matching. Family 2 uses field/parameter/space for kernel/feedback/serpentine.
/// The completed indexed result uses the same caught void sink helper as direct quantize.
#[wasm_bindgen(js_name = privateDitherAndQuantize)]
pub fn private_dither_and_quantize(
    input: &Uint8Array,
    width: f64,
    height: f64,
    palette: &JsValue,
    matching: f64,
    alpha_mode: f64,
    alpha_threshold: f64,
    matte: f64,
    family: f64,
    field: f64,
    parameter: f64,
    space: f64,
    strength: f64,
    placement: f64,
    radius: f64,
    threshold: f64,
    softness: f64,
    result_sink: &JsValue,
) -> u32 {
    let mut processor = match take_ready() {
        Ok(processor) => processor,
        Err(error) => return status(error),
    };
    let result = (|| {
        let (source_width, source_height) = source_dimensions(width, height)?;
        let dither = match family {
            0.0 if [
                field, parameter, space, strength, placement, radius, threshold, softness,
            ]
            .into_iter()
            .all(|n| n == 0.0) =>
            {
                DitherPolicy::None {}
            }
            0.0 => return Err(invalid(ErrorPath::Dither)),
            1.0 => DitherPolicy::Separable {
                perturb: parse_policy(
                    field, parameter, space, strength, placement, radius, threshold, softness,
                )?,
            },
            2.0 => DitherPolicy::Diffusion {
                kernel: match field {
                    0.0 => Diffusion::FloydSteinberg,
                    1.0 => Diffusion::Sierra,
                    2.0 => Diffusion::SierraLite,
                    3.0 => Diffusion::Atkinson,
                    _ => return Err(invalid(ErrorPath::DitherKernel)),
                },
                feedback: match parameter {
                    0.0 => DiffusionFeedback::SrgbBytes,
                    1.0 => DiffusionFeedback::Matching,
                    _ => return Err(invalid(ErrorPath::DitherFeedback)),
                },
                serpentine: match space {
                    0.0 => false,
                    1.0 => true,
                    _ => return Err(invalid(ErrorPath::DitherSerpentine)),
                },
                strength: scalar(strength, ErrorPath::DitherStrength)?,
                placement: parse_placement(
                    placement,
                    radius,
                    threshold,
                    softness,
                    [
                        ErrorPath::DitherPlacement,
                        ErrorPath::DitherRadius,
                        ErrorPath::DitherThreshold,
                        ErrorPath::DitherSoftness,
                    ],
                )?,
            },
            _ => {
                return Err(Failure::new(
                    ErrorCode::UnsupportedOperation,
                    ErrorPath::Dither,
                ))
            }
        };
        let matching = parse_matching(matching)?;
        let alpha = parse_alpha(alpha_mode, alpha_threshold, matte)?;
        let mut entries = [PaletteEntry::Transparent {}; PALETTE_SLOTS];
        let count = read_palette(palette, &mut entries)?;
        processor.dither_and_quantize(
            QuantizeRequest {
                source_width,
                source_height,
                palette: &entries[..count],
                matching,
                alpha,
            },
            dither,
            &mut JsQuantizeBoundary { input, result_sink },
        )
    })();
    restore_ready(processor);
    result.map_or_else(status, |_| 0)
}

fn source_dimensions(width: f64, height: f64) -> Result<(u32, u32), Failure> {
    Ok((
        dimension(
            width,
            MAX_SOURCE_SIDE,
            ErrorCode::InvalidImage,
            ErrorPath::SourceWidth,
        )?,
        dimension(
            height,
            MAX_SOURCE_SIDE,
            ErrorCode::InvalidImage,
            ErrorPath::SourceHeight,
        )?,
    ))
}

fn parse_policy(
    field: f64,
    parameter: f64,
    space: f64,
    strength: f64,
    placement: f64,
    radius: f64,
    threshold: f64,
    softness: f64,
) -> Result<PerturbPolicy, Failure> {
    let field = match field {
        0.0 => Field::Bayer {
            size: match parameter {
                2.0 => BayerSize::Two,
                4.0 => BayerSize::Four,
                8.0 => BayerSize::Eight,
                16.0 => BayerSize::Sixteen,
                _ => return Err(invalid(ErrorPath::PerturbField)),
            },
        },
        1.0 if parameter.is_finite()
            && parameter.fract() == 0.0
            && (0.0..=u32::MAX as f64).contains(&parameter) =>
        {
            Field::Random {
                seed: parameter as u32,
            }
        }
        1.0 => return Err(invalid(ErrorPath::PerturbField)),
        2.0 => {
            return Err(Failure::new(
                ErrorCode::UnsupportedOperation,
                ErrorPath::PerturbField,
            ))
        }
        _ => return Err(invalid(ErrorPath::PerturbField)),
    };
    let space = match space {
        0.0 => WorkingSpace::Srgb,
        1.0 => WorkingSpace::LinearRgb,
        2.0 => WorkingSpace::Oklab,
        3.0 => WorkingSpace::Oklch,
        4.0 => WorkingSpace::Cielab,
        5.0 => WorkingSpace::Cielch,
        6.0 => WorkingSpace::Ycbcr,
        _ => return Err(invalid(ErrorPath::PerturbSpace)),
    };
    let strength = scalar(strength, ErrorPath::PerturbStrength)?;
    let placement = parse_placement(
        placement,
        radius,
        threshold,
        softness,
        [
            ErrorPath::PerturbPlacement,
            ErrorPath::PerturbRadius,
            ErrorPath::PerturbThreshold,
            ErrorPath::PerturbSoftness,
        ],
    )?;
    Ok(PerturbPolicy {
        field,
        space,
        strength,
        placement,
    })
}

fn parse_placement(
    placement: f64,
    radius: f64,
    threshold: f64,
    softness: f64,
    [placement_path, radius_path, threshold_path, softness_path]: [ErrorPath; 4],
) -> Result<Placement, Failure> {
    Ok(match placement {
        0.0 if radius == 0.0 && threshold == 0.0 && softness == 0.0 => Placement::Everywhere {},
        0.0 => return Err(invalid(placement_path)),
        1.0 => Placement::Adaptive {
            radius: dimension(
                radius,
                MAX_SOURCE_SIDE,
                ErrorCode::InvalidSettings,
                radius_path,
            )?,
            threshold: scalar(threshold, threshold_path)?,
            softness: scalar(softness, softness_path)?,
        },
        _ => return Err(invalid(placement_path)),
    })
}

fn scalar(value: f64, path: ErrorPath) -> Result<f32, Failure> {
    if value.is_finite() && (0.0..=f32::MAX as f64).contains(&value) {
        Ok(value as f32)
    } else {
        Err(invalid(path))
    }
}

fn invalid(path: ErrorPath) -> Failure {
    Failure::new(ErrorCode::InvalidSettings, path)
}
