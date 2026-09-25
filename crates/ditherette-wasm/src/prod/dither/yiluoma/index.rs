//! Exact ordered-mixture lookup for bounded Oklab palettes.

use super::{best_matched_mix, PaletteMix};
use crate::prod::{contract::request::MatchPolicy, quantize::matcher::PaletteMatcher};
use std::mem::size_of;

/// Preparing the tree costs more than scanning a few dozen pixels.
pub(crate) const MIN_INDEX_PIXELS: usize = 1_024;

#[derive(Clone, Copy)]
struct Candidate {
    coordinates: [f32; 3],
    mix: PaletteMix,
    order: u32,
    axis: u8,
}

/// Balanced spatial search over the same rounded mixtures as the literal scan.
/// The original enumeration order resolves equal scores, even after tree reordering.
pub(crate) struct MixIndex {
    candidates: Vec<Candidate>,
    levels: u32,
}

impl MixIndex {
    pub(crate) fn required_bytes(matcher: &PaletteMatcher, levels: u32) -> Option<u64> {
        let count = matcher.colors.len() as u64;
        let pairs = count.checked_mul(count.checked_add(1)?)?.checked_div(2)?;
        pairs
            .checked_mul(u64::from(levels).checked_add(1)?)?
            .checked_mul(size_of::<Candidate>() as u64)
    }

    /// Return None when the fast path is unsupported or cannot fit its caller's budget.
    pub(crate) fn try_new(
        matcher: &PaletteMatcher,
        levels: u32,
        available_bytes: u64,
    ) -> Option<Self> {
        if matcher.matching != MatchPolicy::OklabEuclidean || matcher.colors.is_empty() {
            return None;
        }
        let bytes = Self::required_bytes(matcher, levels)?;
        if bytes > available_bytes || bytes > isize::MAX as u64 {
            return None;
        }
        let count = usize::try_from(bytes / size_of::<Candidate>() as u64).ok()?;
        let mut candidates = Vec::new();
        candidates.try_reserve_exact(count).ok()?;
        if (candidates.capacity() * size_of::<Candidate>()) as u64 > available_bytes {
            return None;
        }
        if matcher.colors.iter().any(|color| {
            color
                .coordinates
                .iter()
                .any(|coordinate| !coordinate.is_finite())
        }) {
            return None;
        }
        for low in 0..matcher.colors.len() {
            for high in low..matcher.colors.len() {
                let low_color = matcher.colors[low];
                let high_color = matcher.colors[high];
                for high_count in 0..=levels {
                    let high_ratio = high_count as f32 / levels as f32;
                    let low_ratio = 1.0 - high_ratio;
                    let coordinates = std::array::from_fn(|axis| {
                        low_color.coordinates[axis] * low_ratio
                            + high_color.coordinates[axis] * high_ratio
                    });
                    candidates.push(Candidate {
                        coordinates,
                        mix: PaletteMix {
                            low_index: low_color.index,
                            high_index: high_color.index,
                            high_ratio,
                        },
                        order: candidates.len() as u32,
                        axis: 0,
                    });
                }
            }
        }
        build(&mut candidates);
        Some(Self { candidates, levels })
    }

    pub(crate) fn capacity_bytes(&self) -> u64 {
        (self.candidates.capacity() * size_of::<Candidate>()) as u64
    }

    pub(crate) fn best(&self, color: [f32; 3], matcher: &PaletteMatcher) -> PaletteMix {
        if !color.iter().all(|coordinate| coordinate.is_finite()) {
            return best_matched_mix(color, matcher, self.levels);
        }
        let first = matcher.colors[0];
        let mut best = Candidate {
            coordinates: first.coordinates,
            mix: PaletteMix {
                low_index: first.index,
                high_index: first.index,
                high_ratio: 0.0,
            },
            order: 0,
            axis: 0,
        };
        let mut score = squared(color, first.coordinates);
        search(&self.candidates, color, &mut best, &mut score);
        best.mix
    }
}

fn squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d0 = a[0] - b[0];
    let d1 = a[1] - b[1];
    let d2 = a[2] - b[2];
    d0 * d0 + d1 * d1 + d2 * d2
}

fn build(candidates: &mut [Candidate]) {
    if candidates.is_empty() {
        return;
    }
    let mut min = candidates[0].coordinates;
    let mut max = min;
    for candidate in &candidates[1..] {
        for axis in 0..3 {
            min[axis] = min[axis].min(candidate.coordinates[axis]);
            max[axis] = max[axis].max(candidate.coordinates[axis]);
        }
    }
    let axis = (1..3)
        .max_by(|&a, &b| (max[a] - min[a]).total_cmp(&(max[b] - min[b])))
        .filter(|&a| max[a] - min[a] > max[0] - min[0])
        .unwrap_or(0);
    let middle = candidates.len() / 2;
    candidates.select_nth_unstable_by(middle, |a, b| {
        a.coordinates[axis]
            .total_cmp(&b.coordinates[axis])
            .then(a.order.cmp(&b.order))
    });
    candidates[middle].axis = axis as u8;
    let (lower, rest) = candidates.split_at_mut(middle);
    build(lower);
    build(&mut rest[1..]);
}

fn search(candidates: &[Candidate], color: [f32; 3], best: &mut Candidate, score: &mut f32) {
    if candidates.is_empty() {
        return;
    }
    let middle = candidates.len() / 2;
    let candidate = candidates[middle];
    let candidate_score = squared(color, candidate.coordinates);
    if candidate_score < *score || (candidate_score == *score && candidate.order < best.order) {
        *best = candidate;
        *score = candidate_score;
    }
    let axis = candidate.axis as usize;
    let delta = color[axis] - candidate.coordinates[axis];
    let (lower, upper_with_middle) = candidates.split_at(middle);
    let upper = &upper_with_middle[1..];
    let (near, far) = if delta < 0.0 {
        (lower, upper)
    } else {
        (upper, lower)
    };
    search(near, color, best, score);
    // The widened bound accounts for f32 subtraction and score rounding.
    let gap = f64::from(delta) * f64::from(delta);
    let margin = 16.0 * f64::from(f32::EPSILON) * (1.0 + f64::from(*score));
    if gap <= f64::from(*score) + margin {
        search(far, color, best, score);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image::contracts::PaletteEntry,
        prod::{
            color::packed::Converter,
            contract::request::{AlphaPolicy, MatchPolicy},
            palette::{allocation::Budget, PreparedPalette},
        },
    };

    #[test]
    fn indexed_mix_matches_literal_scan_for_all_sizes_and_ties() {
        let entries = (0..63)
            .map(|index| PaletteEntry::Color {
                rgb: [
                    (index * 4) as u8,
                    ((index * 57) % 256) as u8,
                    ((index * 91) % 256) as u8,
                ],
            })
            .collect::<Vec<_>>();
        let mut budget = Budget::new(1 << 25, 0).unwrap();
        let palette =
            PreparedPalette::prepare(&entries, AlphaPolicy::Premultiplied {}, &mut budget).unwrap();
        let converter = Converter::new(
            crate::prod::color::packed::OrdinarySpace::from_matching(MatchPolicy::OklabEuclidean)
                .unwrap(),
        );
        let matcher = PaletteMatcher::prepare(
            &palette,
            &converter,
            MatchPolicy::OklabEuclidean,
            &mut budget,
        )
        .unwrap();
        let mut state = 0x9183_26a5u32;
        for levels in [4, 16, 64, 256] {
            let index = MixIndex::try_new(&matcher, levels, 1 << 26).unwrap();
            let samples = if levels <= 16 { 1_000 } else { 80 };
            for _ in 0..samples {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                let rgb = [state as u8, (state >> 8) as u8, (state >> 16) as u8];
                let color = converter.coordinates(rgb);
                assert_eq!(
                    index.best(color, &matcher),
                    best_matched_mix(color, &matcher, levels)
                );
            }
        }
    }

    #[test]
    fn bounded_index_preserves_duplicate_first_ties_and_nonfinite_fallback() {
        let entries = [
            PaletteEntry::Color { rgb: [20, 30, 40] },
            PaletteEntry::Color { rgb: [20, 30, 40] },
            PaletteEntry::Color {
                rgb: [240, 220, 200],
            },
        ];
        let mut budget = Budget::new(1 << 20, 0).unwrap();
        let palette =
            PreparedPalette::prepare(&entries, AlphaPolicy::Premultiplied {}, &mut budget).unwrap();
        let converter = Converter::new(
            crate::prod::color::packed::OrdinarySpace::from_matching(MatchPolicy::OklabEuclidean)
                .unwrap(),
        );
        let matcher = PaletteMatcher::prepare(
            &palette,
            &converter,
            MatchPolicy::OklabEuclidean,
            &mut budget,
        )
        .unwrap();
        let bytes = MixIndex::required_bytes(&matcher, 16).unwrap();
        assert!(MixIndex::try_new(&matcher, 16, bytes - 1).is_none());
        let index = MixIndex::try_new(&matcher, 16, bytes).unwrap();
        assert!(index.capacity_bytes() <= bytes);
        for color in [
            converter.coordinates([20, 30, 40]),
            converter.coordinates([240, 220, 200]),
            [f32::NAN, 0.0, 0.0],
        ] {
            assert_eq!(
                index.best(color, &matcher),
                best_matched_mix(color, &matcher, 16)
            );
        }
    }
}
