//! Every error-diffusion kernel: the four v1 kernels plus eight later ones.
//!
//! `diffuse` is the v1 `error_diffusion::diffuse` recipe with the kernel as its only
//! new input. Feedback, scan order, alpha sinks, placement, and arithmetic are unchanged.

use serde::{Deserialize, Serialize};

use crate::{
    image::{contracts::IndexedImage, ImageBuf, PaletteIndex8},
    spec::{
        color::rgb8_to_coordinates,
        contract::{
            error::{DitheretteError, ErrorCode},
            request::{
                Diffusion as V1Kernel, DiffusionFeedback, DitherPolicy, DitherQuantizeRequest,
                Placement, QuantizeRequest, Request,
            },
        },
        dither::{
            error_diffusion::{
                DiffusionTap, ATKINSON_TAPS, FLOYD_STEINBERG_TAPS, SIERRA_LITE_TAPS, SIERRA_TAPS,
            },
            placement::placement_mask_at,
        },
        palette::{PalettePixel, PreparedPalette},
        quantize::{
            matcher::{PaletteColor, PaletteMatcher},
            metric::distance_score,
        },
    },
};

/// The public kernel vocabulary. The first four tags and tap tables are the v1 kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kernel {
    FloydSteinberg,
    Sierra,
    SierraLite,
    Atkinson,
    JarvisJudiceNinke,
    Stucki,
    Burkes,
    TwoRowSierra,
    Fan,
    ShiauFan,
    #[serde(rename = "shiau-fan-2")]
    ShiauFan2,
    #[serde(rename = "simple-2d")]
    Simple2d,
}

/// Builds a tap table from `(dx, dy, numerator)` triples over one denominator.
macro_rules! taps {
    ($denominator:literal; $(($dx:literal, $dy:literal, $numerator:literal)),+ $(,)?) => {
        &[$(DiffusionTap {
            dx: $dx,
            dy: $dy,
            weight: $numerator as f32 / $denominator as f32,
        }),+]
    };
}

pub const JARVIS_JUDICE_NINKE_TAPS: &[DiffusionTap] = taps![48;
    (1, 0, 7), (2, 0, 5),
    (-2, 1, 3), (-1, 1, 5), (0, 1, 7), (1, 1, 5), (2, 1, 3),
    (-2, 2, 1), (-1, 2, 3), (0, 2, 5), (1, 2, 3), (2, 2, 1),
];

pub const STUCKI_TAPS: &[DiffusionTap] = taps![42;
    (1, 0, 8), (2, 0, 4),
    (-2, 1, 2), (-1, 1, 4), (0, 1, 8), (1, 1, 4), (2, 1, 2),
    (-2, 2, 1), (-1, 2, 2), (0, 2, 4), (1, 2, 2), (2, 2, 1),
];

pub const BURKES_TAPS: &[DiffusionTap] = taps![32;
    (1, 0, 8), (2, 0, 4),
    (-2, 1, 2), (-1, 1, 4), (0, 1, 8), (1, 1, 4), (2, 1, 2),
];

pub const TWO_ROW_SIERRA_TAPS: &[DiffusionTap] = taps![16;
    (1, 0, 4), (2, 0, 3),
    (-2, 1, 1), (-1, 1, 2), (0, 1, 3), (1, 1, 2), (2, 1, 1),
];

pub const FAN_TAPS: &[DiffusionTap] = taps![16;
    (1, 0, 7),
    (-2, 1, 1), (-1, 1, 3), (0, 1, 5),
];

pub const SHIAU_FAN_TAPS: &[DiffusionTap] = taps![8;
    (1, 0, 4),
    (-2, 1, 1), (-1, 1, 1), (0, 1, 2),
];

pub const SHIAU_FAN_2_TAPS: &[DiffusionTap] = taps![16;
    (1, 0, 8),
    (-3, 1, 1), (-2, 1, 1), (-1, 1, 2), (0, 1, 4),
];

pub const SIMPLE_2D_TAPS: &[DiffusionTap] = taps![2;
    (1, 0, 1),
    (0, 1, 1),
];

impl Kernel {
    /// Ordered taps. Contributions scatter in this order, which fixes f32 rounding.
    pub const fn taps(self) -> &'static [DiffusionTap] {
        match self {
            Self::FloydSteinberg => FLOYD_STEINBERG_TAPS,
            Self::Sierra => SIERRA_TAPS,
            Self::SierraLite => SIERRA_LITE_TAPS,
            Self::Atkinson => ATKINSON_TAPS,
            Self::JarvisJudiceNinke => JARVIS_JUDICE_NINKE_TAPS,
            Self::Stucki => STUCKI_TAPS,
            Self::Burkes => BURKES_TAPS,
            Self::TwoRowSierra => TWO_ROW_SIERRA_TAPS,
            Self::Fan => FAN_TAPS,
            Self::ShiauFan => SHIAU_FAN_TAPS,
            Self::ShiauFan2 => SHIAU_FAN_2_TAPS,
            Self::Simple2d => SIMPLE_2D_TAPS,
        }
    }
}

/// The v1 diffusion controls with the extended kernel vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiffusionPolicy {
    pub kernel: Kernel,
    pub strength: f32,
    pub placement: Placement,
    pub serpentine: bool,
    pub feedback: DiffusionFeedback,
}

/// Validates and executes one diffusion call with readable full-image f32 work storage.
/// Byte feedback rounds/clips before matching; matching feedback keeps unrounded coordinates.
/// Representational overflow returns a structured error and exposes no partial output.
pub fn diffuse(
    quantize: QuantizeRequest<'_>,
    policy: DiffusionPolicy,
) -> Result<IndexedImage, DitheretteError> {
    // v1 validation never reads the kernel tag, so any v1 kernel checks the other controls.
    let layout = Request::DitherAndQuantize(DitherQuantizeRequest {
        quantize,
        dither: DitherPolicy::Diffusion {
            kernel: V1Kernel::FloydSteinberg,
            strength: policy.strength,
            placement: policy.placement,
            serpentine: policy.serpentine,
            feedback: policy.feedback,
        },
    })
    .validate()?;
    let DiffusionPolicy {
        kernel,
        strength,
        placement,
        serpentine,
        feedback,
    } = policy;
    let palette = PreparedPalette::new(quantize.palette, quantize.alpha);
    let matcher = PaletteMatcher::new(&palette, quantize.matching);
    let space = quantize.matching.space();
    let pixels: Vec<PalettePixel> = quantize
        .source
        .data
        .chunks_exact(4)
        .map(|pixel| palette.prepare_pixel([pixel[0], pixel[1], pixel[2], pixel[3]]))
        .collect();
    let mut work: Vec<[f32; 3]> = pixels
        .iter()
        .map(|&pixel| match pixel {
            PalettePixel::Index(_) => [0.0; 3],
            PalettePixel::Color(rgb) => match feedback {
                DiffusionFeedback::SrgbBytes => rgb.map(f32::from),
                DiffusionFeedback::Matching => rgb8_to_coordinates(rgb, space),
            },
        })
        .collect();
    let mut indices = ImageBuf::<PaletteIndex8>::new_packed(layout.output)
        .expect("validated RGBA8 dimensions also fit packed palette indices");
    let width = layout.output.width_usize();
    let height = layout.output.height_usize();
    for y in 0..height {
        let reverse = serpentine && y % 2 == 1;
        for step in 0..width {
            let x = if reverse { width - 1 - step } else { step };
            let offset = y * width + x;
            if let PalettePixel::Index(index) = pixels[offset] {
                indices.data_mut()[offset] = index;
                continue;
            }
            let current = work[offset];
            if current.iter().any(|value| !value.is_finite()) {
                return Err(arithmetic_error(
                    "Diffusion work exceeded finite f32 range.",
                ));
            }
            let (selected, error) = match feedback {
                DiffusionFeedback::SrgbBytes => {
                    let rgb =
                        current.map(|channel| f64::from(channel).round().clamp(0.0, 255.0) as u8);
                    let selected = nearest_finite(&matcher, rgb8_to_coordinates(rgb, space))?;
                    let start = usize::from(selected.index) * 4;
                    let error: [f64; 3] = std::array::from_fn(|axis| {
                        f64::from(rgb[axis]) - f64::from(palette.palette.rgba[start + axis])
                    });
                    (selected, error)
                }
                DiffusionFeedback::Matching => {
                    let selected = nearest_finite(&matcher, current)?;
                    let error = std::array::from_fn(|axis| {
                        f64::from(current[axis]) - f64::from(selected.coordinates[axis])
                    });
                    (selected, error)
                }
            };
            indices.data_mut()[offset] = selected.index;
            // The unchanged source and matching space determine placement, even for byte feedback.
            let mask = placement_mask_at(layout.source, x as u32, y as u32, space, placement);
            let strength_mask = f64::from(strength) * f64::from(mask);
            for tap in kernel.taps() {
                let dx = if reverse { -tap.dx } else { tap.dx };
                let Some(target_x) = x.checked_add_signed(dx as isize) else {
                    continue;
                };
                let target_y = y + tap.dy as usize;
                if target_x >= width || target_y >= height {
                    continue;
                }
                let target_offset = target_y * width + target_x;
                // Fixed-index pixels are sinks. Their discarded incoming error cannot overflow useful work.
                if matches!(pixels[target_offset], PalettePixel::Index(_)) {
                    continue;
                }
                let weight = f64::from(tap.weight) * strength_mask;
                for axis in 0..3 {
                    let updated =
                        (f64::from(work[target_offset][axis]) + error[axis] * weight) as f32;
                    if !updated.is_finite() {
                        return Err(arithmetic_error(
                            "Diffusion work exceeded finite f32 range.",
                        ));
                    }
                    work[target_offset][axis] = updated;
                }
            }
        }
    }
    Ok(palette.into_indexed(indices))
}

fn nearest_finite(
    matcher: &PaletteMatcher,
    coordinates: [f32; 3],
) -> Result<PaletteColor, DitheretteError> {
    let mut best = matcher.colors[0];
    let mut best_score = f32::INFINITY;
    for &candidate in &matcher.colors {
        let score = distance_score(coordinates, candidate.coordinates, matcher.matching);
        if !score.is_finite() {
            return Err(arithmetic_error(
                "Diffusion matching produced a non-finite distance.",
            ));
        }
        if score < best_score {
            best = candidate;
            best_score = score;
        }
    }
    Ok(best)
}

fn arithmetic_error(message: &str) -> DitheretteError {
    DitheretteError::new(ErrorCode::Runtime, "dither.arithmetic", message)
}
