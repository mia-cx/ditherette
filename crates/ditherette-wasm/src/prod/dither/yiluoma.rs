//! Exact Yliluoma mix search, target adaptation, and ordered selection.
//! Shared palette matching and Bayer ranks remain their existing production implementations.

pub(crate) mod index;
pub(crate) mod policy;
mod request;
pub mod row_bands;
#[cfg(test)]
pub(crate) use request::dither_yiluoma_with_progress;
pub(crate) use request::dither_yiluoma_with_progress_indexed;
pub use request::{dither_yiluoma, dither_yiluoma_into};

use super::ordered::{bayer_value, BayerSize};
use crate::prod::{
    contract::request::MatchPolicy,
    quantize::{
        matcher::PaletteMatcher,
        metric::{
            ciede2000_distance, circular_hue3_squared, euclidean3_squared, hue_arc3_squared,
            weighted_rgb_squared, WeightedRgbMetric,
        },
    },
};

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
/// The metric is chosen once per search, so each candidate runs one monomorphized score.
pub fn best_matched_mix(color: [f32; 3], matcher: &PaletteMatcher, levels: u32) -> PaletteMix {
    match matcher.matching {
        MatchPolicy::SrgbEuclidean
        | MatchPolicy::LinearRgbEuclidean
        | MatchPolicy::OklabEuclidean
        | MatchPolicy::OklchEuclidean
        | MatchPolicy::CielabEuclidean
        | MatchPolicy::CielchEuclidean
        | MatchPolicy::YcbcrEuclidean => search(color, matcher, levels, euclidean3_squared),
        MatchPolicy::OklchCircularHue | MatchPolicy::CielchCircularHue => {
            search(color, matcher, levels, circular_hue3_squared)
        }
        MatchPolicy::OklchHueArc | MatchPolicy::CielchHueArc => {
            search(color, matcher, levels, hue_arc3_squared)
        }
        MatchPolicy::SrgbCompuphase => search(color, matcher, levels, |a, b| {
            weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase)
        }),
        MatchPolicy::SrgbRec601 => search(color, matcher, levels, |a, b| {
            weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601)
        }),
        MatchPolicy::SrgbRec709 => search(color, matcher, levels, |a, b| {
            weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709)
        }),
        MatchPolicy::CielabCiede2000 => search(color, matcher, levels, ciede2000_distance),
    }
}

fn search(
    color: [f32; 3],
    matcher: &PaletteMatcher,
    levels: u32,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32,
) -> PaletteMix {
    find_ordered_mix(
        color,
        matcher.colors.len(),
        |index| {
            let entry = matcher.colors[index];
            (entry.index, entry.coordinates)
        },
        levels,
        distance,
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

const MIX_VALID: u64 = 1 << 24;
const MIX_RGB_MASK: u64 = 0x00ff_ffff;

/// Exact memo of Everywhere mixtures keyed by alpha-prepared RGB, in caller-owned scratch.
/// With Everywhere the target is the source color, so the search is a pure function of RGB
/// for one prepared palette, metric, and Bayer size. Position only picks an index afterward.
pub(crate) struct MixCache<'a> {
    entries: &'a mut [u64],
    levels: u32,
}

impl<'a> MixCache<'a> {
    /// Entries must be a power of two. Rebinding clears any earlier call's mixtures.
    pub(crate) fn new(entries: &'a mut [u64], levels: u32) -> Self {
        assert!(entries.len().is_power_of_two());
        assert!(levels <= 256, "Bayer levels fit the packed high count");
        entries.fill(0);
        Self { entries, levels }
    }

    /// Returns the cached mixture for `rgb`, running `search` only on a miss.
    #[inline]
    pub(crate) fn mix(&mut self, rgb: [u8; 3], search: impl FnOnce() -> PaletteMix) -> PaletteMix {
        let key = u32::from(rgb[0]) << 16 | u32::from(rgb[1]) << 8 | u32::from(rgb[2]);
        let slot = (key.wrapping_mul(0x9e37_79b1) >> 8) as usize & (self.entries.len() - 1);
        let entry = self.entries[slot];
        if entry & (MIX_VALID | MIX_RGB_MASK) == MIX_VALID | u64::from(key) {
            let high_count = (entry >> 41) as u32;
            return PaletteMix {
                low_index: (entry >> 25) as u8,
                high_index: (entry >> 33) as u8,
                // The search derives every ratio with this same expression.
                high_ratio: high_count as f32 / self.levels as f32,
            };
        }
        let mix = search();
        let high_count = (mix.high_ratio * self.levels as f32).round() as u64;
        debug_assert_eq!(
            (high_count as f32 / self.levels as f32).to_bits(),
            mix.high_ratio.to_bits()
        );
        self.entries[slot] = MIX_VALID
            | u64::from(key)
            | u64::from(mix.low_index) << 25
            | u64::from(mix.high_index) << 33
            | high_count << 41;
        mix
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_high_counts_restore_every_ratio_bit_for_bit() {
        for levels in [4, 16, 64, 256] {
            let mut entries = vec![0; 1024];
            let mut cache = MixCache::new(&mut entries, levels);
            for high_count in 0..=levels {
                let rgb = [high_count as u8, (high_count >> 8) as u8, 7];
                let mix = PaletteMix {
                    low_index: 3,
                    high_index: 250,
                    high_ratio: high_count as f32 / levels as f32,
                };
                assert_eq!(cache.mix(rgb, || mix), mix);
                assert_eq!(cache.mix(rgb, || unreachable!("hit")), mix);
            }
        }
    }
}
