//! Error-diffusion dithering specs.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

mod common;
#[cfg(test)]
mod tests;

use self::common::{
    add_error, assert_dither_inputs, nearest_euclidean, read_color, sub_color, write_index,
    Palette3,
};
use super::placement::placement_mask_at;
use crate::{
    image::{contracts::IndexedImage, ImageBuf},
    prod::{
        color::packed::rgb8_to_coordinates,
        contract::{
            error::{DitheretteError, ErrorCode},
            request::{Diffusion, DiffusionFeedback, DitherPolicy, DitherQuantizeRequest, Request},
        },
        palette::{PalettePixel, PreparedPalette},
        quantize::{
            matcher::{PaletteColor, PaletteMatcher},
            metric::distance_score,
        },
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ErrorDiffusionKernel {
    FloydSteinberg,
    Sierra,
    SierraLite,
    Atkinson,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DiffusionTap {
    pub dx: i32,
    pub dy: u32,
    pub weight: f32,
}

pub const FLOYD_STEINBERG_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 7.0 / 16.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 3.0 / 16.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 5.0 / 16.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 1,
        weight: 1.0 / 16.0,
    },
];

pub const SIERRA_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 5.0 / 32.0,
    },
    DiffusionTap {
        dx: 2,
        dy: 0,
        weight: 3.0 / 32.0,
    },
    DiffusionTap {
        dx: -2,
        dy: 1,
        weight: 2.0 / 32.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 4.0 / 32.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 5.0 / 32.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 1,
        weight: 4.0 / 32.0,
    },
    DiffusionTap {
        dx: 2,
        dy: 1,
        weight: 2.0 / 32.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 2,
        weight: 2.0 / 32.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 2,
        weight: 3.0 / 32.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 2,
        weight: 2.0 / 32.0,
    },
];

pub const SIERRA_LITE_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 2.0 / 4.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 1.0 / 4.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 1.0 / 4.0,
    },
];

pub const ATKINSON_TAPS: &[DiffusionTap] = &[
    DiffusionTap {
        dx: 1,
        dy: 0,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 2,
        dy: 0,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: -1,
        dy: 1,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 1,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 1,
        dy: 1,
        weight: 1.0 / 8.0,
    },
    DiffusionTap {
        dx: 0,
        dy: 2,
        weight: 1.0 / 8.0,
    },
];

impl ErrorDiffusionKernel {
    pub const fn taps(self) -> &'static [DiffusionTap] {
        match self {
            Self::FloydSteinberg => FLOYD_STEINBERG_TAPS,
            Self::Sierra => SIERRA_TAPS,
            Self::SierraLite => SIERRA_LITE_TAPS,
            Self::Atkinson => ATKINSON_TAPS,
        }
    }
}

/// Validates and executes one diffusion request with readable full-image f32 work storage.
/// Byte feedback rounds/clips before matching; matching feedback keeps unrounded coordinates.
/// Representational overflow returns a structured error and exposes no partial output.
pub fn diffuse(request: DitherQuantizeRequest<'_>) -> Result<IndexedImage, DitheretteError> {
    let layout = Request::DitherAndQuantize(request).validate()?;
    let DitherPolicy::Diffusion {
        kernel,
        strength,
        placement,
        serpentine,
        feedback,
    } = request.dither
    else {
        return Err(DitheretteError::new(
            ErrorCode::UnsupportedOperation,
            "dither.family",
            "This reference requires an error-diffusion recipe.",
        ));
    };
    let kernel = match kernel {
        Diffusion::FloydSteinberg => ErrorDiffusionKernel::FloydSteinberg,
        Diffusion::Sierra => ErrorDiffusionKernel::Sierra,
        Diffusion::SierraLite => ErrorDiffusionKernel::SierraLite,
        Diffusion::Atkinson => ErrorDiffusionKernel::Atkinson,
    };
    let palette = prepare_palette(request.quantize.palette, request.quantize.alpha);
    let matcher = prepare_matcher(&palette, request.quantize.matching);
    let space = request.quantize.matching.space();
    let pixels: Vec<PalettePixel> = request
        .quantize
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

pub fn dither_error_diffusion_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    kernel: ErrorDiffusionKernel,
    strength: f32,
    serpentine: bool,
) {
    dither_error_diffusion_by_nearest_into(
        source,
        palette,
        output,
        kernel,
        strength,
        serpentine,
        nearest_euclidean,
    );
}

pub fn dither_error_diffusion_by_nearest_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    mut output: ImageViewMut<'_, PaletteIndex8>,
    kernel: ErrorDiffusionKernel,
    strength: f32,
    serpentine: bool,
    nearest: impl Fn([f32; 3], Palette3<'_>) -> (u8, [f32; 3]) + Copy,
) {
    assert_dither_inputs(&source, palette, &output);
    let dimensions = source.dimensions();
    let width = dimensions.width_usize();
    let height = dimensions.height_usize();
    let mut errors = vec![[0.0; 3]; width * height];

    for y in 0..height {
        let reverse = serpentine && y % 2 == 1;
        let source_row = source
            .row(y as u32)
            .expect("source row should be in bounds");
        let output_row = output
            .row_mut(y as u32)
            .expect("output row should be in bounds");
        let xs: Box<dyn Iterator<Item = usize>> = if reverse {
            Box::new((0..width).rev())
        } else {
            Box::new(0..width)
        };

        for x in xs {
            let offset = y * width + x;
            let color = add_error(read_color::<F>(source_row, x), errors[offset]);
            let (index, quantized) = nearest(color, palette);
            write_index(output_row, x, index);
            let error = sub_color(color, quantized);

            for tap in kernel.taps() {
                let dx = if reverse { -tap.dx } else { tap.dx };
                let Some(target_x) = x.checked_add_signed(dx as isize) else {
                    continue;
                };
                let target_y = y + tap.dy as usize;
                if target_x >= width || target_y >= height {
                    continue;
                }
                let target = &mut errors[target_y * width + target_x];
                target[0] += error[0] * tap.weight * strength;
                target[1] += error[1] * tap.weight * strength;
                target[2] += error[2] * tap.weight * strength;
            }
        }
    }
}

// Baseline-only constructor wiring. This is not a public bounded-memory path.
fn prepare_palette(
    entries: &[crate::image::contracts::PaletteEntry],
    alpha: crate::prod::contract::request::AlphaPolicy,
) -> PreparedPalette {
    PreparedPalette::try_new(entries, alpha, u64::MAX)
        .expect("validated palette and baseline allocation")
}

fn prepare_matcher(
    palette: &PreparedPalette,
    matching: crate::prod::contract::request::MatchPolicy,
) -> PaletteMatcher {
    use crate::prod::{
        color::packed::{Converter, PackedSpace},
        palette::allocation::Budget,
    };
    let converter = Converter::new(
        PackedSpace::from_matching(matching).expect("every matching tag has coordinates"),
    );
    let mut budget = Budget::new(u64::MAX, 0).expect("unbounded baseline budget");
    PaletteMatcher::prepare(palette, &converter, matching, &mut budget)
        .expect("baseline matcher allocation")
}
