//! Yliluoma-style ordered palette-mixing dithering spec.
//!
//! For each source color, the spec finds the best two-color palette mix for the
//! current Bayer matrix size, then chooses between those two palette entries by
//! comparing the pixel threshold with the selected mix ratio. This keeps the
//! operation deterministic and palette-index based while preserving the core
//! Yliluoma idea: approximate colors by ordered mixtures of palette colors.

use crate::image::{ImageFormat, ImageView, ImageViewMut, PaletteIndex8};

use super::{
    common::{assert_dither_inputs, squared_distance, write_index, Palette3},
    ordered::{bayer_value, BayerSize},
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteMix {
    pub low_index: u8,
    pub high_index: u8,
    pub high_ratio: f32,
}

pub fn dither_yiluoma_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    output: ImageViewMut<'_, PaletteIndex8>,
    size: BayerSize,
) {
    dither_yiluoma_by_distance_into(source, palette, output, size, squared_distance);
}

pub fn dither_yiluoma_by_distance_into<F: ImageFormat<Storage = f32>>(
    source: ImageView<'_, F>,
    palette: Palette3<'_>,
    mut output: ImageViewMut<'_, PaletteIndex8>,
    size: BayerSize,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32 + Copy,
) {
    assert_dither_inputs(&source, palette, &output);
    let dimensions = source.dimensions();
    let width = size.width();
    let denominator = (width * width) as f32;

    for y in 0..dimensions.height() {
        let source_row = source.row(y).expect("source row should be in bounds");
        let output_row = output.row_mut(y).expect("output row should be in bounds");

        for x in 0..dimensions.width_usize() {
            let source_start = x * F::CHANNEL_COUNT;
            let color = [
                source_row[source_start],
                source_row[source_start + 1],
                source_row[source_start + 2],
            ];
            let mix = best_ordered_mix_by_distance(color, palette, denominator as u32, distance);
            let threshold =
                (bayer_value(x % width, y as usize % width, width) as f32 + 0.5) / denominator;
            let index = if threshold < mix.high_ratio {
                mix.high_index
            } else {
                mix.low_index
            };
            write_index(output_row, x, index);
        }
    }
}

pub fn best_ordered_mix(color: [f32; 3], palette: Palette3<'_>, levels: u32) -> PaletteMix {
    best_ordered_mix_by_distance(color, palette, levels, squared_distance)
}

pub fn best_ordered_mix_by_distance(
    color: [f32; 3],
    palette: Palette3<'_>,
    levels: u32,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32,
) -> PaletteMix {
    assert!(
        !palette.is_empty(),
        "palette must contain at least one color"
    );
    assert!(palette.len() <= 256, "palette indices are stored as u8");
    assert!(
        levels > 0,
        "ordered mix requires at least one threshold level"
    );

    let mut best = PaletteMix {
        low_index: 0,
        high_index: 0,
        high_ratio: 0.0,
    };
    let mut best_distance = distance(color, palette[0]);

    for low_index in 0..palette.len() {
        for high_index in low_index..palette.len() {
            for high_count in 0..=levels {
                let high_ratio = high_count as f32 / levels as f32;
                let low_ratio = 1.0 - high_ratio;
                let mixed = [
                    palette[low_index][0] * low_ratio + palette[high_index][0] * high_ratio,
                    palette[low_index][1] * low_ratio + palette[high_index][1] * high_ratio,
                    palette[low_index][2] * low_ratio + palette[high_index][2] * high_ratio,
                ];
                let candidate_distance = distance(color, mixed);
                if candidate_distance < best_distance {
                    best = PaletteMix {
                        low_index: low_index as u8,
                        high_index: high_index as u8,
                        high_ratio,
                    };
                    best_distance = candidate_distance;
                }
            }
        }
    }

    best
}
