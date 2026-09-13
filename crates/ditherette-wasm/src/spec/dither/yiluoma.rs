//! Yliluoma-style ordered palette-mixing dithering spec.
//!
//! For each source color, the spec finds the best two-color palette mix for the
//! current Bayer matrix size, then chooses between those two palette entries by
//! comparing the pixel threshold with the selected mix ratio. This keeps the
//! operation deterministic and palette-index based while preserving the core
//! Yliluoma idea: approximate colors by ordered mixtures of palette colors.

use crate::image::{
    contracts::IndexedImage, ImageBuf, ImageFormat, ImageView, ImageViewMut, PaletteIndex8,
};
use crate::spec::{
    color::rgb8_to_coordinates,
    contract::{
        error::{DitheretteError, ErrorCode},
        request::{BayerSize as RequestBayerSize, DitherPolicy, DitherQuantizeRequest, Request},
    },
    palette::{PalettePixel, PreparedPalette},
    quantize::{matcher::PaletteMatcher, metric::distance_score},
};

use super::{
    common::{assert_dither_inputs, squared_distance, write_index, Palette3},
    ordered::{bayer_value, BayerSize},
    placement::placement_mask_at,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteMix {
    pub low_index: u8,
    pub high_index: u8,
    pub high_ratio: f32,
}

/// Validates a Yliluoma request and returns durable indexed output with normalized palette metadata.
/// Alpha preparation precedes target conversion; adaptive placement reads the original source RGB.
pub fn dither_yiluoma(request: DitherQuantizeRequest<'_>) -> Result<IndexedImage, DitheretteError> {
    let layout = Request::DitherAndQuantize(request).validate()?;
    let DitherPolicy::Yliluoma { size, placement } = request.dither else {
        return Err(DitheretteError::new(
            ErrorCode::UnsupportedOperation,
            "dither.family",
            "This reference requires the Yliluoma family.",
        ));
    };
    let size = match size {
        RequestBayerSize::Two => BayerSize::Two,
        RequestBayerSize::Four => BayerSize::Four,
        RequestBayerSize::Eight => BayerSize::Eight,
        RequestBayerSize::Sixteen => BayerSize::Sixteen,
    };
    let quantize = request.quantize;
    let palette = PreparedPalette::new(quantize.palette, quantize.alpha);
    let matcher = PaletteMatcher::new(&palette, quantize.matching);
    let mut indices = ImageBuf::<PaletteIndex8>::new_packed(layout.output)
        .expect("validated RGBA8 dimensions also fit packed palette indices");
    for y in 0..layout.output.height() {
        for x in 0..layout.output.width() {
            let source = layout
                .source
                .pixel(x, y)
                .expect("source pixel is in bounds");
            let rgba = [source[0], source[1], source[2], source[3]];
            let index = match palette.prepare_pixel(rgba) {
                PalettePixel::Index(index) => index,
                PalettePixel::Color(rgb) => {
                    let coordinates = rgb8_to_coordinates(rgb, quantize.matching.space());
                    let nearest = matcher.nearest(coordinates);
                    let mask = placement_mask_at(
                        layout.source,
                        x,
                        y,
                        quantize.matching.space(),
                        placement,
                    );
                    let target = adaptive_target(coordinates, nearest.coordinates, mask);
                    let mix =
                        best_matched_mix(target, &matcher, (size.width() * size.width()) as u32);
                    ordered_mix_index(mix, x, y, size)
                }
            };
            indices.data_mut()[y as usize * layout.output.width_usize() + x as usize] = index;
        }
    }
    Ok(palette.into_indexed(indices))
}

/// Componentwise nearest-to-source target adaptation for a placement mask in [0,1].
/// Endpoint branches retain exact coordinates; they do not bypass mixture search.
pub fn adaptive_target(source: [f32; 3], nearest: [f32; 3], mask: f32) -> [f32; 3] {
    if mask == 0.0 {
        return nearest;
    }
    if mask == 1.0 {
        return source;
    }
    std::array::from_fn(|axis| nearest[axis] + mask * (source[axis] - nearest[axis]))
}

/// Selects the high entry only when the global Bayer cell-center threshold is below its ratio.
pub fn ordered_mix_index(mix: PaletteMix, x: u32, y: u32, size: BayerSize) -> u8 {
    let width = size.width();
    let threshold = (f32::from(bayer_value(x as usize % width, y as usize % width, width)) + 0.5)
        / (width * width) as f32;
    if threshold < mix.high_ratio {
        mix.high_index
    } else {
        mix.low_index
    }
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
            let index = ordered_mix_index(mix, x as u32, y, size);
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
    find_ordered_mix(
        color,
        palette.len(),
        |index| (index as u8, palette[index]),
        levels,
        distance,
    )
}

/// Exhaustively searches visible palette pairs with the selected matching metric.
/// Returned indices are original retained palette indices, not compact visible offsets.
pub fn best_matched_mix(color: [f32; 3], matcher: &PaletteMatcher, levels: u32) -> PaletteMix {
    find_ordered_mix(
        color,
        matcher.colors.len(),
        |index| {
            let entry = matcher.colors[index];
            (entry.index, entry.coordinates)
        },
        levels,
        |left, right| distance_score(left, right, matcher.matching),
    )
}

fn find_ordered_mix(
    color: [f32; 3],
    palette_len: usize,
    palette_color: impl Fn(usize) -> (u8, [f32; 3]),
    levels: u32,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32,
) -> PaletteMix {
    assert!(palette_len > 0, "palette must contain at least one color");
    assert!(palette_len <= 256, "palette indices are stored as u8");
    assert!(
        levels > 0,
        "ordered mix requires at least one threshold level"
    );

    let (first_index, first_color) = palette_color(0);
    let mut best = PaletteMix {
        low_index: first_index,
        high_index: first_index,
        high_ratio: 0.0,
    };
    let mut best_distance = distance(color, first_color);

    for low_offset in 0..palette_len {
        for high_offset in low_offset..palette_len {
            let (low_index, low) = palette_color(low_offset);
            let (high_index, high) = palette_color(high_offset);
            for high_count in 0..=levels {
                let high_ratio = high_count as f32 / levels as f32;
                let low_ratio = 1.0 - high_ratio;
                let mixed = [
                    low[0] * low_ratio + high[0] * high_ratio,
                    low[1] * low_ratio + high[1] * high_ratio,
                    low[2] * low_ratio + high[2] * high_ratio,
                ];
                let candidate_distance = distance(color, mixed);
                if candidate_distance < best_distance {
                    best = PaletteMix {
                        low_index,
                        high_index,
                        high_ratio,
                    };
                    best_distance = candidate_distance;
                }
            }
        }
    }

    best
}
