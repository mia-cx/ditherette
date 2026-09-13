//! Quantization distance metrics.

use crate::spec::color::lab_ciede2000::ciede2000;
use crate::spec::contract::request::MatchPolicy;

/// RGB-specific weighted distance variants over gamma-encoded sRGB channels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeightedRgbMetric {
    /// CompuPhase red-mean weighted RGB.
    CompuPhase,
    /// Fixed Rec.601 luma weights applied to RGB channel deltas.
    Rec601,
    /// Fixed Rec.709/sRGB luma weights applied to RGB channel deltas.
    Rec709,
}

/// Squared Euclidean distance for ordinary 3-channel coordinate spaces.
pub fn euclidean3_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d0 = a[0] - b[0];
    let d1 = a[1] - b[1];
    let d2 = a[2] - b[2];
    d0 * d0 + d1 * d1 + d2 * d2
}

/// Squared cylindrical distance for L/C/h spaces with hue in radians.
///
/// Hue distance wraps around the circle. The hue term uses the chord length at
/// the geometric-mean chroma so hue matters less near neutral gray, where hue is
/// poorly defined.
pub fn circular_hue3_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let delta_lightness = a[0] - b[0];
    let delta_chroma = a[1] - b[1];
    let delta_hue = (a[2] - b[2] + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU)
        - std::f32::consts::PI;
    let hue_chord = 2.0 * (a[1] * b[1]).max(0.0).sqrt() * (delta_hue * 0.5).sin();

    delta_lightness * delta_lightness + delta_chroma * delta_chroma + hue_chord * hue_chord
}

/// Website cylindrical distance: shortest hue arc scaled by the smaller chroma.
/// Coordinates use nonnegative chroma and hue in radians.
pub fn hue_arc3_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let delta_lightness = a[0] - b[0];
    let delta_chroma = a[1] - b[1];
    let delta_hue = (a[2] - b[2]).abs().rem_euclid(std::f32::consts::TAU);
    let shortest_hue = delta_hue.min(std::f32::consts::TAU - delta_hue);
    let hue_arc = a[1].min(b[1]) * shortest_hue;
    delta_lightness * delta_lightness + delta_chroma * delta_chroma + hue_arc * hue_arc
}

/// RGB-specific weighted distance over normalized gamma-encoded sRGB channels.
pub fn weighted_rgb_squared(a: [f32; 3], b: [f32; 3], metric: WeightedRgbMetric) -> f32 {
    let dr = a[0] - b[0];
    let dg = a[1] - b[1];
    let db = a[2] - b[2];

    match metric {
        WeightedRgbMetric::CompuPhase => {
            let red_mean_bytes = (a[0] + b[0]) * 0.5 * 255.0;
            (2.0 + red_mean_bytes / 256.0) * dr * dr
                + 4.0 * dg * dg
                + (2.0 + (255.0 - red_mean_bytes) / 256.0) * db * db
        }
        WeightedRgbMetric::Rec601 => 0.299 * dr * dr + 0.587 * dg * dg + 0.114 * db * db,
        WeightedRgbMetric::Rec709 => 0.2126 * dr * dr + 0.7152 * dg * dg + 0.0722 * db * db,
    }
}

/// CIEDE2000 distance over CIELAB coordinates.
pub fn ciede2000_distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ciede2000(a, b)
}

/// Scores two coordinates in the policy's working space. Smaller scores win.
/// Euclidean, circular, and RGB metrics are squared; CIEDE2000 returns delta E.
pub fn distance_score(a: [f32; 3], b: [f32; 3], policy: MatchPolicy) -> f32 {
    match policy {
        MatchPolicy::SrgbEuclidean
        | MatchPolicy::LinearRgbEuclidean
        | MatchPolicy::OklabEuclidean
        | MatchPolicy::OklchEuclidean
        | MatchPolicy::CielabEuclidean
        | MatchPolicy::CielchEuclidean
        | MatchPolicy::YcbcrEuclidean => euclidean3_squared(a, b),
        MatchPolicy::OklchCircularHue | MatchPolicy::CielchCircularHue => {
            circular_hue3_squared(a, b)
        }
        MatchPolicy::OklchHueArc | MatchPolicy::CielchHueArc => hue_arc3_squared(a, b),
        MatchPolicy::SrgbCompuphase => weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase),
        MatchPolicy::SrgbRec601 => weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601),
        MatchPolicy::SrgbRec709 => weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709),
        MatchPolicy::CielabCiede2000 => ciede2000_distance(a, b),
    }
}
