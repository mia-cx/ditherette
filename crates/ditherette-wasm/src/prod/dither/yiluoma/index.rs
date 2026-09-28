//! Exact ordered-mixture lookup for every production matching policy.

use super::{best_matched_mix, PaletteMix};
use crate::prod::{
    color::lab_ciede2000::{lightness_lower_bound, unprimed_chroma, Ciede2000Pair},
    contract::request::MatchPolicy,
    quantize::{
        matcher::PaletteMatcher,
        metric::{
            circular_hue3_squared, euclidean3_squared, hue_arc3_squared, weighted_rgb_squared,
            WeightedRgbMetric,
        },
    },
};
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

#[derive(Clone, Copy, Default)]
struct Bounds {
    min: [f32; 3],
    max: [f32; 3],
    max_chroma: f32,
}

/// Balanced spatial search over the same rounded mixtures as the literal scan.
/// The original enumeration order resolves equal scores, even after tree reordering.
pub(crate) struct MixIndex {
    candidates: Vec<Candidate>,
    bounds: Vec<Bounds>,
    ciede_chroma: Vec<f32>,
    scoring_coordinates: Vec<[f32; 3]>,
    first: Candidate,
    levels: u32,
    matching: MatchPolicy,
}

impl MixIndex {
    pub(crate) fn required_bytes(matcher: &PaletteMatcher, levels: u32) -> Option<u64> {
        let count = matcher.colors().len() as u64;
        let pairs = count.checked_mul(count.checked_add(1)?)?.checked_div(2)?;
        let candidates = pairs.checked_mul(u64::from(levels).checked_add(1)?)?;
        let bytes_per_candidate = size_of::<Candidate>()
            + if needs_bounds(matcher.matching) {
                size_of::<Bounds>()
            } else {
                0
            }
            + if matcher.matching == MatchPolicy::CielabCiede2000 {
                size_of::<f32>()
            } else {
                0
            }
            + if is_hue_metric(matcher.matching) {
                size_of::<[f32; 3]>()
            } else {
                0
            };
        candidates.checked_mul(bytes_per_candidate as u64)
    }

    /// Return None when the index cannot preserve the metric's bounds or fit the budget.
    pub(crate) fn try_new(
        matcher: &PaletteMatcher,
        levels: u32,
        available_bytes: u64,
    ) -> Option<Self> {
        if matcher.colors().is_empty()
            || matcher.colors().iter().any(|color| {
                color
                    .coordinates
                    .iter()
                    .any(|coordinate| !coordinate.is_finite())
            })
            || !supported_coordinates(matcher)
        {
            return None;
        }
        let bytes = Self::required_bytes(matcher, levels)?;
        if bytes > available_bytes || bytes > isize::MAX as u64 {
            return None;
        }
        let palette_len = matcher.colors().len() as u64;
        let count = usize::try_from(
            palette_len
                .checked_mul(palette_len.checked_add(1)?)?
                .checked_div(2)?
                .checked_mul(u64::from(levels).checked_add(1)?)?,
        )
        .ok()?;
        let mut candidates = Vec::new();
        candidates.try_reserve_exact(count).ok()?;
        if (candidates.capacity() * size_of::<Candidate>()) as u64 > available_bytes {
            return None;
        }
        for low in 0..matcher.colors().len() {
            for high in low..matcher.colors().len() {
                let low_color = matcher.colors()[low];
                let high_color = matcher.colors()[high];
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
        let first = candidates[0];
        let scoring_coordinates = if is_hue_metric(matcher.matching) {
            let remaining = available_bytes
                .checked_sub((candidates.capacity() * size_of::<Candidate>()) as u64)?;
            let mut coordinates = Vec::new();
            coordinates.try_reserve_exact(count).ok()?;
            if (coordinates.capacity() * size_of::<[f32; 3]>()) as u64 > remaining {
                return None;
            }
            coordinates.extend(candidates.iter().map(|candidate| candidate.coordinates));
            for candidate in &mut candidates {
                candidate.coordinates = polar_coordinates(candidate.coordinates);
            }
            coordinates
        } else {
            Vec::new()
        };
        build(&mut candidates, 3);

        let mut bounds = if needs_bounds(matcher.matching) {
            let remaining = available_bytes
                .checked_sub((candidates.capacity() * size_of::<Candidate>()) as u64)?;
            let mut bounds = Vec::new();
            bounds.try_reserve_exact(count).ok()?;
            if (bounds.capacity() * size_of::<Bounds>()) as u64 > remaining {
                return None;
            }
            bounds.resize(count, Bounds::default());
            bounds
        } else {
            Vec::new()
        };
        let ciede_chroma = if matcher.matching == MatchPolicy::CielabCiede2000 {
            let remaining = available_bytes
                .checked_sub((candidates.capacity() * size_of::<Candidate>()) as u64)?
                .checked_sub((bounds.capacity() * size_of::<Bounds>()) as u64)?;
            let mut chroma = Vec::new();
            chroma.try_reserve_exact(count).ok()?;
            if (chroma.capacity() * size_of::<f32>()) as u64 > remaining {
                return None;
            }
            chroma.extend(
                candidates
                    .iter()
                    .map(|candidate| unprimed_chroma(candidate.coordinates)),
            );
            chroma
        } else {
            Vec::new()
        };
        if !bounds.is_empty() {
            if is_hue_metric(matcher.matching) {
                build_scoring_bounds(&candidates, &scoring_coordinates, &mut bounds);
            } else {
                build_bounds(&candidates, &ciede_chroma, &mut bounds);
            }
        }

        Some(Self {
            candidates,
            bounds,
            ciede_chroma,
            scoring_coordinates,
            first,
            levels,
            matching: matcher.matching,
        })
    }

    pub(crate) fn capacity_bytes(&self) -> u64 {
        (self.candidates.capacity() * size_of::<Candidate>()
            + self.bounds.capacity() * size_of::<Bounds>()
            + self.ciede_chroma.capacity() * size_of::<f32>()) as u64
            + (self.scoring_coordinates.capacity() * size_of::<[f32; 3]>()) as u64
    }

    pub(crate) fn best(&self, color: [f32; 3], matcher: &PaletteMatcher) -> PaletteMix {
        if !color.iter().all(|coordinate| coordinate.is_finite())
            || !supported_target(color, self.matching)
        {
            return best_matched_mix(color, matcher, self.levels);
        }
        debug_assert_eq!(self.matching, matcher.matching);
        match self.matching {
            MatchPolicy::SrgbEuclidean
            | MatchPolicy::LinearRgbEuclidean
            | MatchPolicy::OklabEuclidean
            | MatchPolicy::OklchEuclidean
            | MatchPolicy::CielabEuclidean
            | MatchPolicy::CielchEuclidean
            | MatchPolicy::YcbcrEuclidean => self.best_metric(color, euclidean3_squared, [1.0; 3]),
            MatchPolicy::OklchCircularHue | MatchPolicy::CielchCircularHue => {
                self.best_polar_metric(color, circular_hue3_squared, 1.0, circular_box_lower_bound)
            }
            MatchPolicy::OklchHueArc | MatchPolicy::CielchHueArc => self.best_polar_metric(
                color,
                hue_arc3_squared,
                1.0 / (1.0 + std::f32::consts::PI * 0.5),
                hue_arc_box_lower_bound,
            ),
            MatchPolicy::SrgbCompuphase => self.best_metric(
                color,
                |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase),
                [2.0, 4.0, 2.0],
            ),
            MatchPolicy::SrgbRec601 => self.best_metric(
                color,
                |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601),
                [0.299, 0.587, 0.114],
            ),
            MatchPolicy::SrgbRec709 => self.best_metric(
                color,
                |a, b| weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709),
                [0.2126, 0.7152, 0.0722],
            ),
            MatchPolicy::CielabCiede2000 => self.best_ciede2000(color),
        }
    }

    fn best_metric(
        &self,
        color: [f32; 3],
        distance: impl Fn([f32; 3], [f32; 3]) -> f32 + Copy,
        axis_weights: [f32; 3],
    ) -> PaletteMix {
        let mut best = self.first;
        let mut score = distance(color, best.coordinates);
        if !score.is_finite() {
            return best.mix;
        }
        search_metric(
            &self.candidates,
            color,
            &mut best,
            &mut score,
            distance,
            axis_weights,
        );
        best.mix
    }

    fn best_ciede2000(&self, color: [f32; 3]) -> PaletteMix {
        let source_chroma = unprimed_chroma(color);
        let mut best = self.first;
        let mut euclidean_score = f32::INFINITY;
        search_metric(
            &self.candidates,
            color,
            &mut best,
            &mut euclidean_score,
            euclidean3_squared,
            [1.0; 3],
        );
        let mut score = Ciede2000Pair::new(
            color,
            source_chroma,
            best.coordinates,
            unprimed_chroma(best.coordinates),
        )
        .distance();
        if !score.is_finite() {
            return best.mix;
        }
        search_ciede2000(
            &self.candidates,
            &self.ciede_chroma,
            &self.bounds,
            color,
            source_chroma,
            &mut best,
            &mut score,
        );
        best.mix
    }

    fn best_polar_metric(
        &self,
        color: [f32; 3],
        distance: impl Fn([f32; 3], [f32; 3]) -> f32 + Copy,
        lower_bound_scale: f32,
        lower_bound: impl Fn([f32; 3], Bounds) -> f32 + Copy,
    ) -> PaletteMix {
        let mut best = self.first;
        let mut score = distance(color, best.coordinates);
        if !score.is_finite() {
            return best.mix;
        }
        search_indexed_metric(
            &self.candidates,
            &self.scoring_coordinates,
            &self.bounds,
            polar_coordinates(color),
            color,
            &mut best,
            &mut score,
            distance,
            [lower_bound_scale; 3],
            lower_bound,
        );
        best.mix
    }
}

fn needs_bounds(matching: MatchPolicy) -> bool {
    matching == MatchPolicy::CielabCiede2000 || is_hue_metric(matching)
}

fn is_hue_metric(matching: MatchPolicy) -> bool {
    matches!(
        matching,
        MatchPolicy::OklchCircularHue
            | MatchPolicy::OklchHueArc
            | MatchPolicy::CielchCircularHue
            | MatchPolicy::CielchHueArc
    )
}

fn supported_coordinates(matcher: &PaletteMatcher) -> bool {
    match matcher.matching {
        MatchPolicy::SrgbCompuphase => matcher.colors().iter().all(|color| {
            color
                .coordinates
                .iter()
                .all(|value| (0.0..=1.0).contains(value))
        }),
        MatchPolicy::CielabCiede2000 => matcher
            .colors()
            .iter()
            .all(|color| (0.0..=100.0).contains(&color.coordinates[0])),
        MatchPolicy::OklchCircularHue
        | MatchPolicy::OklchHueArc
        | MatchPolicy::CielchCircularHue
        | MatchPolicy::CielchHueArc => matcher.colors().iter().all(|color| {
            color.coordinates[1] >= 0.0
                && (0.0..=std::f32::consts::TAU).contains(&color.coordinates[2])
        }),
        _ => true,
    }
}

fn supported_target(color: [f32; 3], matching: MatchPolicy) -> bool {
    match matching {
        MatchPolicy::SrgbCompuphase => color.iter().all(|value| (0.0..=1.0).contains(value)),
        MatchPolicy::CielabCiede2000 => (0.0..=100.0).contains(&color[0]),
        MatchPolicy::OklchCircularHue
        | MatchPolicy::OklchHueArc
        | MatchPolicy::CielchCircularHue
        | MatchPolicy::CielchHueArc => {
            color[1] >= 0.0 && (0.0..=std::f32::consts::TAU).contains(&color[2])
        }
        _ => true,
    }
}

fn build(candidates: &mut [Candidate], axes: usize) {
    if candidates.is_empty() {
        return;
    }
    let mut min = candidates[0].coordinates;
    let mut max = min;
    for candidate in &candidates[1..] {
        for axis in 0..axes {
            min[axis] = min[axis].min(candidate.coordinates[axis]);
            max[axis] = max[axis].max(candidate.coordinates[axis]);
        }
    }
    let axis = (1..axes)
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
    build(lower, axes);
    build(&mut rest[1..], axes);
}

fn search_metric(
    candidates: &[Candidate],
    color: [f32; 3],
    best: &mut Candidate,
    score: &mut f32,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32 + Copy,
    axis_weights: [f32; 3],
) {
    if candidates.is_empty() {
        return;
    }
    let middle = candidates.len() / 2;
    let candidate = candidates[middle];
    let candidate_score = distance(color, candidate.coordinates);
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
    search_metric(near, color, best, score, distance, axis_weights);
    let gap = f64::from(axis_weights[axis]) * f64::from(delta) * f64::from(delta);
    let margin = 16.0 * f64::from(f32::EPSILON) * (1.0 + f64::from(*score));
    if gap <= f64::from(*score) + margin {
        search_metric(far, color, best, score, distance, axis_weights);
    }
}

fn build_bounds(candidates: &[Candidate], chroma: &[f32], bounds: &mut [Bounds]) -> Bounds {
    let middle = candidates.len() / 2;
    let candidate = candidates[middle];
    let mut node = Bounds {
        min: candidate.coordinates,
        max: candidate.coordinates,
        max_chroma: chroma.get(middle).copied().unwrap_or_default(),
    };
    if middle > 0 {
        let child = build_bounds(
            &candidates[..middle],
            chroma.get(..middle).unwrap_or_default(),
            &mut bounds[..middle],
        );
        extend_bounds(&mut node, child);
    }
    if middle + 1 < candidates.len() {
        let child = build_bounds(
            &candidates[middle + 1..],
            chroma.get(middle + 1..).unwrap_or_default(),
            &mut bounds[middle + 1..],
        );
        extend_bounds(&mut node, child);
    }
    bounds[middle] = node;
    node
}

fn build_scoring_bounds(
    candidates: &[Candidate],
    scoring_coordinates: &[[f32; 3]],
    bounds: &mut [Bounds],
) -> Bounds {
    let middle = candidates.len() / 2;
    let coordinates = scoring_coordinates[candidates[middle].order as usize];
    let mut node = Bounds {
        min: coordinates,
        max: coordinates,
        max_chroma: 0.0,
    };
    if middle > 0 {
        let child = build_scoring_bounds(
            &candidates[..middle],
            scoring_coordinates,
            &mut bounds[..middle],
        );
        extend_bounds(&mut node, child);
    }
    if middle + 1 < candidates.len() {
        let child = build_scoring_bounds(
            &candidates[middle + 1..],
            scoring_coordinates,
            &mut bounds[middle + 1..],
        );
        extend_bounds(&mut node, child);
    }
    bounds[middle] = node;
    node
}

#[allow(clippy::too_many_arguments)]
fn search_indexed_metric(
    candidates: &[Candidate],
    scoring_coordinates: &[[f32; 3]],
    bounds: &[Bounds],
    indexed_color: [f32; 3],
    color: [f32; 3],
    best: &mut Candidate,
    score: &mut f32,
    distance: impl Fn([f32; 3], [f32; 3]) -> f32 + Copy,
    axis_weights: [f32; 3],
    lower_bound: impl Fn([f32; 3], Bounds) -> f32 + Copy,
) {
    if candidates.is_empty()
        || lower_bound(color, bounds[candidates.len() / 2])
            > *score + 32.0 * f32::EPSILON * (1.0 + *score)
    {
        return;
    }
    let middle = candidates.len() / 2;
    let candidate = candidates[middle];
    let candidate_score = distance(color, scoring_coordinates[candidate.order as usize]);
    if candidate_score < *score || (candidate_score == *score && candidate.order < best.order) {
        *best = candidate;
        *score = candidate_score;
    }
    let axis = candidate.axis as usize;
    let delta = indexed_color[axis] - candidate.coordinates[axis];
    let (lower_candidates, upper_with_middle) = candidates.split_at(middle);
    let upper_candidates = &upper_with_middle[1..];
    let (lower_bounds, upper_with_middle) = bounds.split_at(middle);
    let upper_bounds = &upper_with_middle[1..];
    let (near, far) = if delta < 0.0 {
        (
            (lower_candidates, lower_bounds),
            (upper_candidates, upper_bounds),
        )
    } else {
        (
            (upper_candidates, upper_bounds),
            (lower_candidates, lower_bounds),
        )
    };
    search_indexed_metric(
        near.0,
        scoring_coordinates,
        near.1,
        indexed_color,
        color,
        best,
        score,
        distance,
        axis_weights,
        lower_bound,
    );
    let gap = f64::from(axis_weights[axis]) * f64::from(delta) * f64::from(delta);
    let margin = 32.0 * f64::from(f32::EPSILON) * (1.0 + f64::from(*score));
    if 0.99 * gap <= f64::from(*score) + margin {
        search_indexed_metric(
            far.0,
            scoring_coordinates,
            far.1,
            indexed_color,
            color,
            best,
            score,
            distance,
            axis_weights,
            lower_bound,
        );
    }
}

fn polar_coordinates([lightness, chroma, hue]: [f32; 3]) -> [f32; 3] {
    let (sin, cos) = hue.sin_cos();
    [lightness, chroma * cos, chroma * sin]
}

fn box_axis_distance(value: f32, min: f32, max: f32) -> f64 {
    if value < min {
        f64::from(min - value)
    } else if value > max {
        f64::from(value - max)
    } else {
        0.0
    }
}

fn periodic_box_distance(value: f32, min: f32, max: f32) -> f64 {
    let value = f64::from(value);
    let min = f64::from(min);
    let max = f64::from(max);
    let tau = f64::from(std::f32::consts::TAU);
    [-tau, 0.0, tau]
        .into_iter()
        .map(|shift| {
            if value < min + shift {
                min + shift - value
            } else if value > max + shift {
                value - max - shift
            } else {
                0.0
            }
        })
        .fold(f64::INFINITY, f64::min)
}

fn circular_box_lower_bound(color: [f32; 3], bounds: Bounds) -> f32 {
    let dl = box_axis_distance(color[0], bounds.min[0], bounds.max[0]);
    let dc = box_axis_distance(color[1], bounds.min[1], bounds.max[1]);
    let dh = periodic_box_distance(color[2], bounds.min[2], bounds.max[2]);
    let hue = 4.0 * f64::from((color[1] * bounds.min[1]).max(0.0)) * (dh * 0.5).sin().powi(2);
    (0.99 * (dl * dl + dc * dc + hue)) as f32
}

fn hue_arc_box_lower_bound(color: [f32; 3], bounds: Bounds) -> f32 {
    let dl = box_axis_distance(color[0], bounds.min[0], bounds.max[0]);
    let dc = box_axis_distance(color[1], bounds.min[1], bounds.max[1]);
    let dh = periodic_box_distance(color[2], bounds.min[2], bounds.max[2]);
    let chroma = f64::from(color[1].min(bounds.min[1]));
    (0.99 * (dl * dl + dc * dc + chroma * chroma * dh * dh)) as f32
}

fn extend_bounds(node: &mut Bounds, child: Bounds) {
    for axis in 0..3 {
        node.min[axis] = node.min[axis].min(child.min[axis]);
        node.max[axis] = node.max[axis].max(child.max[axis]);
    }
    node.max_chroma = node.max_chroma.max(child.max_chroma);
}

#[allow(clippy::too_many_arguments)]
fn search_ciede2000(
    candidates: &[Candidate],
    chroma: &[f32],
    bounds: &[Bounds],
    color: [f32; 3],
    source_chroma: f32,
    best: &mut Candidate,
    score: &mut f32,
) {
    let permitted_score = *score + 32.0 * f32::EPSILON * (1.0 + *score);
    if candidates.is_empty()
        || ciede_box_lower_bound_squared(color, source_chroma, bounds[candidates.len() / 2])
            > f64::from(permitted_score) * f64::from(permitted_score)
    {
        return;
    }
    let middle = candidates.len() / 2;
    let candidate = candidates[middle];
    let candidate_chroma = chroma[middle];
    let dl = (color[0] - candidate.coordinates[0]) / 1.75;
    let da = color[1] - candidate.coordinates[1];
    let db = color[2] - candidate.coordinates[2];
    let c_bar_prime_bound = 0.75 * (source_chroma + candidate_chroma);
    let scale_bound = 1.0 + 0.045 * c_bar_prime_bound;
    let coarse_squared = dl * dl + 0.13 * (da * da + db * db) / (scale_bound * scale_bound);
    if candidate.order != best.order && coarse_squared <= *score * *score {
        let lightness_bound = lightness_lower_bound(color[0], candidate.coordinates[0]);
        if lightness_bound > *score || (lightness_bound == *score && candidate.order > best.order) {
            search_ciede2000_children(
                candidates,
                chroma,
                bounds,
                color,
                source_chroma,
                best,
                score,
            );
            return;
        }
        let pair = Ciede2000Pair::new(
            color,
            source_chroma,
            candidate.coordinates,
            candidate_chroma,
        );
        let pair_bound = pair.lower_bound();
        if pair_bound < *score || (pair_bound == *score && candidate.order < best.order) {
            let hue = pair.prepare_hue();
            let hue_bound = hue.lower_bound();
            if hue_bound < *score || (hue_bound == *score && candidate.order < best.order) {
                let candidate_score = hue.distance();
                if candidate_score < *score
                    || (candidate_score == *score && candidate.order < best.order)
                {
                    *best = candidate;
                    *score = candidate_score;
                }
            }
        }
    }

    search_ciede2000_children(
        candidates,
        chroma,
        bounds,
        color,
        source_chroma,
        best,
        score,
    );
}

#[allow(clippy::too_many_arguments)]
fn search_ciede2000_children(
    candidates: &[Candidate],
    chroma: &[f32],
    bounds: &[Bounds],
    color: [f32; 3],
    source_chroma: f32,
    best: &mut Candidate,
    score: &mut f32,
) {
    let middle = candidates.len() / 2;
    let candidate = candidates[middle];
    let axis = candidate.axis as usize;
    let delta = color[axis] - candidate.coordinates[axis];
    let (lower_candidates, upper_with_middle) = candidates.split_at(middle);
    let upper_candidates = &upper_with_middle[1..];
    let (lower_chroma, upper_with_middle) = chroma.split_at(middle);
    let upper_chroma = &upper_with_middle[1..];
    let (lower_bounds, upper_with_middle) = bounds.split_at(middle);
    let upper_bounds = &upper_with_middle[1..];
    let (near, far) = if delta < 0.0 {
        (
            (lower_candidates, lower_chroma, lower_bounds),
            (upper_candidates, upper_chroma, upper_bounds),
        )
    } else {
        (
            (upper_candidates, upper_chroma, upper_bounds),
            (lower_candidates, lower_chroma, lower_bounds),
        )
    };
    search_ciede2000(near.0, near.1, near.2, color, source_chroma, best, score);
    search_ciede2000(far.0, far.1, far.2, color, source_chroma, best, score);
}

fn ciede_box_lower_bound_squared(color: [f32; 3], source_chroma: f32, bounds: Bounds) -> f64 {
    let axis_distance = |axis: usize| {
        if color[axis] < bounds.min[axis] {
            f64::from(bounds.min[axis] - color[axis])
        } else if color[axis] > bounds.max[axis] {
            f64::from(color[axis] - bounds.max[axis])
        } else {
            0.0
        }
    };
    let dl = axis_distance(0) / 1.75;
    let da = axis_distance(1);
    let db = axis_distance(2);
    let scale = 1.0 + 0.045 * 0.75 * f64::from(source_chroma + bounds.max_chroma);
    0.9801 * (dl * dl + 0.13 * (da * da + db * db) / (scale * scale))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        image::contracts::PaletteEntry,
        prod::{
            color::packed::{Converter, OrdinarySpace},
            contract::request::{AlphaPolicy, MatchPolicy},
            palette::{allocation::Budget, PreparedPalette},
        },
    };

    const POLICIES: [MatchPolicy; 15] = [
        MatchPolicy::SrgbEuclidean,
        MatchPolicy::SrgbCompuphase,
        MatchPolicy::SrgbRec601,
        MatchPolicy::SrgbRec709,
        MatchPolicy::LinearRgbEuclidean,
        MatchPolicy::OklabEuclidean,
        MatchPolicy::OklchEuclidean,
        MatchPolicy::OklchCircularHue,
        MatchPolicy::OklchHueArc,
        MatchPolicy::CielabEuclidean,
        MatchPolicy::CielabCiede2000,
        MatchPolicy::CielchEuclidean,
        MatchPolicy::CielchCircularHue,
        MatchPolicy::CielchHueArc,
        MatchPolicy::YcbcrEuclidean,
    ];

    fn matcher(entries: &[PaletteEntry], matching: MatchPolicy) -> (Converter, PaletteMatcher) {
        let mut budget = Budget::new(1 << 28, 0).unwrap();
        let palette =
            PreparedPalette::prepare(entries, AlphaPolicy::Premultiplied {}, &mut budget).unwrap();
        let converter = Converter::new(OrdinarySpace::from_matching(matching).unwrap());
        let matcher = PaletteMatcher::prepare(&palette, &converter, matching, &mut budget).unwrap();
        (converter, matcher)
    }

    #[test]
    fn bounded_index_preserves_exact_ties_and_nonfinite_fallback() {
        let entries = [
            PaletteEntry::Color { rgb: [0, 0, 0] },
            PaletteEntry::Color {
                rgb: [128, 128, 128],
            },
            PaletteEntry::Color {
                rgb: [255, 255, 255],
            },
            PaletteEntry::Color {
                rgb: [255, 255, 255],
            },
        ];
        for matching in POLICIES {
            let (converter, matcher) = matcher(&entries, matching);
            let bytes = MixIndex::required_bytes(&matcher, 16).unwrap();
            assert!(MixIndex::try_new(&matcher, 16, bytes - 1).is_none());
            let index = MixIndex::try_new(&matcher, 16, bytes).unwrap();
            assert!(index.capacity_bytes() <= bytes);
            for color in [
                converter.coordinates([0, 0, 0]),
                converter.coordinates([64, 64, 64]),
                converter.coordinates([255, 255, 255]),
                [f32::NAN, 0.0, 0.0],
            ] {
                assert_eq!(
                    index.best(color, &matcher),
                    best_matched_mix(color, &matcher, 16),
                    "{matching:?}/{color:?}"
                );
            }
            let outside_bound_domain = match matching {
                MatchPolicy::SrgbCompuphase => Some([-0.25, 0.5, 0.5]),
                MatchPolicy::CielabCiede2000 => Some([101.0, 0.0, 0.0]),
                MatchPolicy::OklchCircularHue
                | MatchPolicy::OklchHueArc
                | MatchPolicy::CielchCircularHue
                | MatchPolicy::CielchHueArc => Some([0.5, -0.1, 7.0]),
                _ => None,
            };
            if let Some(color) = outside_bound_domain {
                assert_eq!(
                    index.best(color, &matcher),
                    best_matched_mix(color, &matcher, 16),
                    "finite fallback for {matching:?}/{color:?}"
                );
            }
        }
    }
}
