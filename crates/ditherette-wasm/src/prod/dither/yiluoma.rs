//! Literal frozen Yliluoma mix search, target adaptation, and ordered selection.
//! Shared palette matching and Bayer ranks remain their existing production implementations.

mod request;
pub mod row_bands;
pub(crate) use request::dither_yiluoma_with_progress;
pub use request::{dither_yiluoma, dither_yiluoma_into};

use super::ordered::{bayer_value, BayerSize};
use crate::prod::quantize::{matcher::PaletteMatcher, metric::distance_score};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteMix {
    pub low_index: u8,
    pub high_index: u8,
    pub high_ratio: f32,
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
