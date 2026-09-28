//! Metric-specialized direct scans over the production converter's coordinates.

pub use super::metric::euclidean3_squared;
use super::metric::{
    ciede2000_distance, circular_hue3_squared, hue_arc3_squared, hue_remainder,
    weighted_rgb_squared, WeightedRgbMetric,
};
use crate::prod::contract::request::MatchPolicy;
use crate::prod::palette::{allocation::Budget, PreparationError};
use crate::prod::{
    color::{
        lab_ciede2000::{lightness_lower_bound, unprimed_chroma, Ciede2000Pair},
        packed::Converter,
    },
    palette::PreparedPalette,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteColor {
    pub index: u8,
    pub coordinates: [f32; 3],
    pub(crate) ciede2000_chroma: f32,
}

impl PaletteColor {
    /// Prepares the pair-independent terms used by metric-specialized scans.
    pub fn new(index: u8, coordinates: [f32; 3]) -> Self {
        Self {
            index,
            coordinates,
            ciede2000_chroma: unprimed_chroma(coordinates),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaletteMatcher {
    colors: Vec<PaletteColor>,
    pub matching: MatchPolicy,
    lightness_order: [u8; 256],
    bounded_ciede_lightness: bool,
}

impl PaletteMatcher {
    /// Builds metric-specific lookup metadata without reordering palette entries.
    pub fn new(colors: Vec<PaletteColor>, matching: MatchPolicy) -> Self {
        let mut lightness_order = [0; 256];
        for (position, slot) in lightness_order[..colors.len()].iter_mut().enumerate() {
            *slot = position as u8;
        }
        lightness_order[..colors.len()].sort_by(|&a, &b| {
            colors[a as usize].coordinates[0].total_cmp(&colors[b as usize].coordinates[0])
        });
        Self {
            bounded_ciede_lightness: colors
                .iter()
                .all(|color| (0.0..=100.0).contains(&color.coordinates[0])),
            colors,
            matching,
            lightness_order,
        }
    }

    /// Returns the palette entries in their original tie-breaking order.
    pub fn colors(&self) -> &[PaletteColor] {
        &self.colors
    }

    pub(crate) fn colors_capacity(&self) -> usize {
        self.colors.capacity()
    }

    /// Converts visible entries without reordering or removing duplicates.
    pub(crate) fn prepare(
        palette: &PreparedPalette,
        converter: &Converter,
        matching: MatchPolicy,
        budget: &mut Budget,
    ) -> Result<Self, PreparationError> {
        let mut colors = Vec::new();
        budget.reserve(&mut colors, palette.visible.len())?;
        colors.extend(
            palette
                .visible
                .iter()
                .map(|entry| PaletteColor::new(entry.index, converter.coordinates(entry.rgb))),
        );
        Ok(Self::new(colors, matching))
    }

    /// Exact score ties keep the first entry. Transparent-only pixels bypass this scan.
    pub fn nearest(&self, coordinates: [f32; 3]) -> PaletteColor {
        match self.matching {
            MatchPolicy::SrgbEuclidean
            | MatchPolicy::LinearRgbEuclidean
            | MatchPolicy::OklabEuclidean
            | MatchPolicy::OklchEuclidean
            | MatchPolicy::CielabEuclidean
            | MatchPolicy::CielchEuclidean
            | MatchPolicy::YcbcrEuclidean => self.scan(coordinates, euclidean3_squared),
            MatchPolicy::OklchCircularHue | MatchPolicy::CielchCircularHue => {
                self.scan(coordinates, circular_hue3_squared)
            }
            MatchPolicy::OklchHueArc => self.scan_normalized_hue_arc(coordinates),
            MatchPolicy::CielchHueArc => self.scan(coordinates, hue_arc3_squared),
            MatchPolicy::SrgbCompuphase => self.scan(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase)
            }),
            MatchPolicy::SrgbRec601 => self.scan(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601)
            }),
            MatchPolicy::SrgbRec709 => self.scan(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709)
            }),
            MatchPolicy::CielabCiede2000 => self.scan_ciede2000(coordinates),
        }
    }

    /// Keeps the first exact tie and rejects any non-finite candidate score.
    /// Diffusion can exceed the converted source domain, including after an earlier finite match.
    pub(crate) fn nearest_finite(&self, coordinates: [f32; 3]) -> Option<PaletteColor> {
        match self.matching {
            MatchPolicy::SrgbEuclidean
            | MatchPolicy::LinearRgbEuclidean
            | MatchPolicy::OklabEuclidean
            | MatchPolicy::OklchEuclidean
            | MatchPolicy::CielabEuclidean
            | MatchPolicy::CielchEuclidean
            | MatchPolicy::YcbcrEuclidean => self.scan_finite(coordinates, euclidean3_squared),
            MatchPolicy::OklchCircularHue | MatchPolicy::CielchCircularHue => {
                self.scan_finite(coordinates, circular_hue3_squared)
            }
            MatchPolicy::OklchHueArc | MatchPolicy::CielchHueArc => {
                self.scan_finite(coordinates, finite_hue_arc3_squared)
            }
            MatchPolicy::SrgbCompuphase => self.scan_finite(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase)
            }),
            MatchPolicy::SrgbRec601 => self.scan_finite(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601)
            }),
            MatchPolicy::SrgbRec709 => self.scan_finite(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709)
            }),
            MatchPolicy::CielabCiede2000 => self.scan_finite(coordinates, ciede2000_distance),
        }
    }

    fn scan_finite(
        &self,
        coordinates: [f32; 3],
        distance: impl Fn([f32; 3], [f32; 3]) -> f32,
    ) -> Option<PaletteColor> {
        let mut best = self.colors[0];
        let mut best_score = f32::INFINITY;
        for &candidate in &self.colors {
            let score = distance(coordinates, candidate.coordinates);
            if !score.is_finite() {
                return None;
            }
            if score < best_score {
                best = candidate;
                best_score = score;
            }
        }
        Some(best)
    }

    // Direct Oklch coordinates have normalized hues, so the shortest arc only
    // needs one comparison. Diffusion keeps the general finite scan above.
    fn scan_normalized_hue_arc(&self, coordinates: [f32; 3]) -> PaletteColor {
        let mut best = self.colors[0];
        let mut best_score = normalized_hue_arc3_squared(coordinates, best.coordinates);
        for &candidate in &self.colors[1..] {
            let score = normalized_hue_arc3_squared(coordinates, candidate.coordinates);
            if score < best_score {
                best = candidate;
                best_score = score;
            }
        }
        best
    }

    // Each function item/closure produces its own scan, with no per-candidate policy dispatch.
    fn scan(
        &self,
        coordinates: [f32; 3],
        distance: impl Fn([f32; 3], [f32; 3]) -> f32,
    ) -> PaletteColor {
        let mut best = self.colors[0];
        let mut best_score = distance(coordinates, best.coordinates);
        for &candidate in &self.colors[1..] {
            let score = distance(coordinates, candidate.coordinates);
            if score < best_score {
                best = candidate;
                best_score = score;
            }
        }
        best
    }

    /// Seeds from the closest Euclidean Lab colour, then skips candidates that cannot win.
    /// Equal complete scores still resolve to the first palette entry.
    fn scan_ciede2000(&self, coordinates: [f32; 3]) -> PaletteColor {
        let source_chroma = unprimed_chroma(coordinates);
        let mut best_position = 0;
        let mut closest_euclidean = euclidean3_squared(coordinates, self.colors[0].coordinates);
        for (position, candidate) in self.colors[1..].iter().enumerate() {
            let distance = euclidean3_squared(coordinates, candidate.coordinates);
            if distance < closest_euclidean {
                best_position = position + 1;
                closest_euclidean = distance;
            }
        }
        let mut best = self.colors[best_position];
        let best_pair = Ciede2000Pair::new(
            coordinates,
            source_chroma,
            best.coordinates,
            best.ciede2000_chroma,
        );
        let mut best_score = best_pair.distance();
        // RGB-derived Lab lightness is in [0, 100], where S_L stays below 1.75.
        // Everything outside this sorted interval has a lightness term above `best_score`.
        let lightness_order = &self.lightness_order[..self.colors.len()];
        let (start, end) =
            if self.bounded_ciede_lightness && (0.0..=100.0).contains(&coordinates[0]) {
                let lightness_window = next_up(1.75 * pruning_ceiling(best_score));
                let lower_lightness = next_down(coordinates[0] - lightness_window);
                let upper_lightness = next_up(coordinates[0] + lightness_window);
                (
                    lightness_order.partition_point(|&position| {
                        self.colors[position as usize].coordinates[0] < lower_lightness
                    }),
                    lightness_order.partition_point(|&position| {
                        self.colors[position as usize].coordinates[0] <= upper_lightness
                    }),
                )
            } else {
                (0, lightness_order.len())
            };
        for &position in &lightness_order[start..end] {
            let position = position as usize;
            let candidate = self.colors[position];
            if position == best_position {
                continue;
            }
            if self.bounded_ciede_lightness && (0.0..=100.0).contains(&coordinates[0]) {
                let dl = (coordinates[0] - candidate.coordinates[0]) / 1.75;
                let da = coordinates[1] - candidate.coordinates[1];
                let db = coordinates[2] - candidate.coordinates[2];
                let c_bar_prime_bound = 0.75 * (source_chroma + candidate.ciede2000_chroma);
                let scale_bound = 1.0 + 0.045 * c_bar_prime_bound;
                // |R_T| <= sqrt(3), while both S_C and S_H stay below this scale.
                // 0.13 is a conservative lower approximation of 1 - sqrt(3)/2.
                let coarse_squared =
                    dl * dl + 0.13 * (da * da + db * db) / (scale_bound * scale_bound);
                if rounded_bound_exceeds_best(coarse_squared.sqrt(), best_score) {
                    continue;
                }
            }
            let lightness_bound = lightness_lower_bound(coordinates[0], candidate.coordinates[0]);
            if rounded_bound_exceeds_best(lightness_bound, best_score) {
                continue;
            }
            let pair = Ciede2000Pair::new(
                coordinates,
                source_chroma,
                candidate.coordinates,
                candidate.ciede2000_chroma,
            );
            let lower_bound = pair.lower_bound();
            if rounded_bound_exceeds_best(lower_bound, best_score) {
                continue;
            }
            let hue = pair.prepare_hue();
            let hue_bound = hue.lower_bound();
            if rounded_bound_exceeds_best(hue_bound, best_score) {
                continue;
            }
            let score = hue.distance();
            if score < best_score || (score == best_score && position < best_position) {
                best_position = position;
                best = candidate;
                best_score = score;
            }
        }
        best
    }
}

// This exceeds the accumulated f32 error of every CIEDE2000 lower-bound recipe.
// Candidates inside the margin reach the complete frozen-order score and tie check.
const CIEDE2000_BOUND_SAFETY: f32 = 0.999;

// A relative margin is empty at a zero best score, yet a nonzero bound can belong to a pair whose
// complete score underflows to zero: squares below the smallest normal f32 lose their value, so
// scores under about 1e-22 round away. This absolute floor keeps those ties in the scan.
const CIEDE2000_BOUND_FLOOR: f32 = 1e-18;

#[inline(always)]
fn rounded_bound_exceeds_best(bound: f32, best: f32) -> bool {
    bound * CIEDE2000_BOUND_SAFETY > best + CIEDE2000_BOUND_FLOOR
}

#[inline(always)]
fn pruning_ceiling(best: f32) -> f32 {
    next_up((best + CIEDE2000_BOUND_FLOOR) / CIEDE2000_BOUND_SAFETY)
}

#[inline(always)]
fn next_up(value: f32) -> f32 {
    if value.is_nan() || value == f32::INFINITY {
        value
    } else if value == -0.0 {
        f32::from_bits(1)
    } else if value >= 0.0 {
        f32::from_bits(value.to_bits() + 1)
    } else {
        f32::from_bits(value.to_bits() - 1)
    }
}

#[inline(always)]
fn next_down(value: f32) -> f32 {
    -next_up(-value)
}

/// Only for scans that reject every nonfinite score. Nonfinite input coordinates
/// force a nonfinite squared delta or hue term, independent of NaN propagation
/// through these comparisons. Finite signed-zero choices disappear on squaring.
#[inline(always)]
pub(super) fn finite_hue_arc3_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let delta_lightness = a[0] - b[0];
    let delta_chroma = a[1] - b[1];
    let delta_hue = hue_remainder((a[2] - b[2]).abs());
    let other_hue = std::f32::consts::TAU - delta_hue;
    let shortest_hue = if delta_hue < other_hue {
        delta_hue
    } else {
        other_hue
    };
    let chroma = if a[1] < b[1] { a[1] } else { b[1] };
    let hue_arc = chroma * shortest_hue;
    delta_lightness * delta_lightness + delta_chroma * delta_chroma + hue_arc * hue_arc
}

#[inline(always)]
pub(super) fn normalized_hue_arc3_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let delta_lightness = a[0] - b[0];
    let delta_chroma = a[1] - b[1];
    let delta_hue = (a[2] - b[2]).abs();
    let shortest_hue = if delta_hue > std::f32::consts::PI {
        std::f32::consts::TAU - delta_hue
    } else {
        delta_hue
    };
    let hue_arc = a[1].min(b[1]) * shortest_hue;
    delta_lightness * delta_lightness + delta_chroma * delta_chroma + hue_arc * hue_arc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prod::color::packed::{Converter, PackedSpace};

    #[test]
    fn finite_hue_score_preserves_bits_or_rejection_for_arbitrary_coordinates() {
        let check = |a, b| {
            let expected = hue_arc3_squared(a, b);
            let actual = finite_hue_arc3_squared(a, b);
            assert_eq!(actual.is_finite(), expected.is_finite(), "{a:?}, {b:?}");
            if expected.is_finite() {
                assert_eq!(actual.to_bits(), expected.to_bits(), "{a:?}, {b:?}");
            }
        };
        for signs in 0..64 {
            let coordinate = |axis| if signs & (1 << axis) == 0 { 0.0 } else { -0.0 };
            check(
                std::array::from_fn(coordinate),
                std::array::from_fn(|axis| coordinate(axis + 3)),
            );
        }
        for axis in 0..6 {
            for value in [
                f32::INFINITY,
                f32::NEG_INFINITY,
                f32::NAN,
                f32::from_bits(0xff80_0001),
                f32::MAX,
                -f32::MAX,
                f32::from_bits(1),
                -f32::MIN_POSITIVE,
                -0.25,
                0.0,
                -0.0,
            ] {
                let mut pair = [[0.5, -0.25, -12.0], [0.25, 0.1, 24.0]];
                pair[axis / 3][axis % 3] = value;
                check(pair[0], pair[1]);
            }
        }
        let mut bits = 0x5ea4_b731u32;
        let mut next = || {
            bits ^= bits << 13;
            bits ^= bits >> 17;
            bits ^= bits << 5;
            bits
        };
        for n in 0..100_000 {
            let pair: [[f32; 3]; 2] = std::array::from_fn(|_| {
                std::array::from_fn(|_| {
                    let bits = next();
                    if n % 2 == 0 {
                        f32::from_bits(bits)
                    } else {
                        bits as i32 as f32 * 1e-8
                    }
                })
            });
            check(pair[0], pair[1]);
        }
    }

    #[test]
    fn finite_hue_scan_rejects_nonfinite_coordinates_on_either_side() {
        for matching in [MatchPolicy::OklchHueArc, MatchPolicy::CielchHueArc] {
            for axis in 0..3 {
                for invalid in [
                    f32::INFINITY,
                    f32::NEG_INFINITY,
                    f32::NAN,
                    f32::from_bits(0xff80_0001),
                ] {
                    let mut bad = [0.5, 0.25, 1.0];
                    bad[axis] = invalid;
                    let good = PaletteColor::new(0, [0.5, 0.25, 1.0]);
                    let matcher = PaletteMatcher::new(vec![good], matching);
                    assert_eq!(matcher.nearest_finite(bad), None);
                    let matcher =
                        PaletteMatcher::new(vec![good, PaletteColor::new(1, bad)], matching);
                    assert_eq!(matcher.nearest_finite(good.coordinates), None);
                }
            }
        }
    }

    #[test]
    fn finite_hue_scan_preserves_ties_and_late_invalid_scores() {
        let reference = |a: [f32; 3], b: [f32; 3]| {
            let dl = a[0] - b[0];
            let dc = a[1] - b[1];
            let dh = (a[2] - b[2]).abs().rem_euclid(std::f32::consts::TAU);
            let arc = a[1].min(b[1]) * dh.min(std::f32::consts::TAU - dh);
            dl * dl + dc * dc + arc * arc
        };
        let converter = Converter::new(PackedSpace::Oklch);
        let colors = [[255, 0, 0], [0, 255, 0], [0, 0, 255], [255, 0, 0]]
            .into_iter()
            .enumerate()
            .map(|(index, rgb)| PaletteColor::new(index as u8, converter.coordinates(rgb)))
            .collect::<Vec<_>>();
        for matching in [MatchPolicy::OklchHueArc, MatchPolicy::CielchHueArc] {
            let matcher = PaletteMatcher::new(colors.clone(), matching);
            for n in 0..4096_u32 {
                let coordinates = [0.5, n as f32 * 0.001 - 1.0, n as f32 * 0.01 - 20.0];
                assert_eq!(
                    matcher.nearest_finite(coordinates),
                    matcher.scan_finite(coordinates, reference)
                );
            }
            assert_eq!(
                matcher.nearest_finite(colors[0].coordinates).unwrap().index,
                0
            );
            for hue in [f32::INFINITY, f32::NEG_INFINITY, f32::NAN] {
                assert_eq!(matcher.nearest_finite([0.5, 0.2, hue]), None);
            }
            let mut invalid_colors = colors.clone();
            invalid_colors.push(PaletteColor::new(4, [f32::MAX, f32::MAX, 0.0]));
            let matcher = PaletteMatcher::new(invalid_colors, matching);
            assert_eq!(matcher.nearest_finite(colors[0].coordinates), None);
        }
    }

    #[test]
    fn normalized_hue_arc_matches_reference_for_rgb_derived_hue_spaces() {
        for space in [PackedSpace::Oklch, PackedSpace::Cielch] {
            let converter = Converter::new(space);
            for n in 0..4096u32 {
                let a = converter.coordinates([
                    (n * 73) as u8,
                    (n * 31 + n / 256) as u8,
                    (n * 17 + 113) as u8,
                ]);
                let b = converter.coordinates([
                    (n * 19 + 7) as u8,
                    (n * 47 + 29) as u8,
                    (n * 101 + n / 64) as u8,
                ]);
                assert_eq!(
                    normalized_hue_arc3_squared(a, b).to_bits(),
                    hue_arc3_squared(a, b).to_bits(),
                    "RGB-derived pair {n}: {a:?}, {b:?}"
                );
            }
        }
    }

    #[test]
    fn normalized_hue_arc_matches_reference_at_normalized_hue_edges() {
        let below_tau = f32::from_bits(std::f32::consts::TAU.to_bits() - 1);
        let above_pi = f32::from_bits(std::f32::consts::PI.to_bits() + 1);
        for (a, b) in [
            ([0.5, 0.0, 0.0], [0.5, 0.0, below_tau]),
            ([0.25, 0.2, 0.0], [0.75, 0.1, below_tau]),
            ([0.5, 0.2, 0.0], [0.5, 0.1, std::f32::consts::PI]),
            ([0.5, 0.2, 0.0], [0.5, 0.1, above_pi]),
            ([0.5, 0.1, below_tau], [0.5, 0.2, 0.0]),
        ] {
            assert_eq!(
                normalized_hue_arc3_squared(a, b).to_bits(),
                hue_arc3_squared(a, b).to_bits(),
                "{a:?}, {b:?}"
            );
        }
    }

    #[test]
    fn specialized_scan_matches_reference_and_keeps_the_first_tie() {
        let converter = Converter::new(PackedSpace::Oklch);
        let palette_rgbs = [
            [255, 0, 0],
            [0, 255, 0],
            [0, 0, 255],
            [127, 127, 127],
            [255, 0, 0],
        ];
        let colors = palette_rgbs
            .into_iter()
            .enumerate()
            .map(|(index, rgb)| PaletteColor::new(index as u8, converter.coordinates(rgb)))
            .collect::<Vec<_>>();
        let matcher = PaletteMatcher::new(colors.clone(), MatchPolicy::OklchHueArc);

        for n in 0..1024u32 {
            let coordinates = converter.coordinates([
                (n * 73) as u8,
                (n * 31 + n / 256) as u8,
                (n * 17 + 113) as u8,
            ]);
            let mut expected = colors[0];
            let mut expected_score = hue_arc3_squared(coordinates, expected.coordinates);
            for &candidate in &colors[1..] {
                let score = hue_arc3_squared(coordinates, candidate.coordinates);
                if score < expected_score {
                    expected = candidate;
                    expected_score = score;
                }
            }
            assert_eq!(matcher.nearest(coordinates), expected);
        }

        assert_eq!(matcher.nearest(colors[0].coordinates).index, 0);
    }

    #[test]
    fn ciede2000_bounds_stay_below_the_complete_distance() {
        let converter = Converter::new(PackedSpace::Cielab);
        let mut bits = 0x53c9_1e27u32;
        let mut next = || {
            bits ^= bits << 13;
            bits ^= bits >> 17;
            bits ^= bits << 5;
            bits
        };
        for sample in 0..100_000u32 {
            let rgb = |value: u32| [value as u8, (value >> 8) as u8, (value >> 16) as u8];
            let source_rgb = if sample % 8 == 0 {
                let grey = next() as u8;
                [grey, grey, grey.wrapping_add((sample % 3) as u8)]
            } else {
                rgb(next())
            };
            let candidate_rgb = if sample % 11 == 0 {
                let grey = next() as u8;
                [grey, grey.wrapping_add((sample % 2) as u8), grey]
            } else {
                rgb(next())
            };
            let source = converter.coordinates(source_rgb);
            let candidate = converter.coordinates(candidate_rgb);
            let pair = Ciede2000Pair::new(
                source,
                unprimed_chroma(source),
                candidate,
                unprimed_chroma(candidate),
            );
            let expected = pair.distance();
            assert!(
                lightness_lower_bound(source[0], candidate[0]) <= expected,
                "lightness: {source_rgb:?}, {candidate_rgb:?}"
            );
            let pair_bound = pair.lower_bound();
            assert!(
                pair_bound <= expected,
                "pair: {source_rgb:?}, {candidate_rgb:?}: {pair_bound} > {expected}"
            );
            assert!(
                pair.prepare_hue().lower_bound() <= expected,
                "hue: {source_rgb:?}, {candidate_rgb:?}"
            );
        }
    }
}
