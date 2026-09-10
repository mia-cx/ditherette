//! Metric-specialized direct scans over the production converter's coordinates.

pub use super::metric::euclidean3_squared;
use super::metric::{
    ciede2000_distance, circular_hue3_squared, hue_arc3_squared, weighted_rgb_squared,
    WeightedRgbMetric,
};
use crate::prod::contract::request::MatchPolicy;
use crate::prod::palette::{allocation::Budget, PreparationError};
use crate::prod::{color::packed::Converter, palette::PreparedPalette};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteColor {
    pub index: u8,
    pub coordinates: [f32; 3],
}

#[derive(Debug, Clone, PartialEq)]
pub struct PaletteMatcher {
    pub colors: Vec<PaletteColor>,
    pub matching: MatchPolicy,
}

impl PaletteMatcher {
    /// Converts visible entries without reordering or removing duplicates.
    pub(crate) fn prepare(
        palette: &PreparedPalette,
        converter: &Converter,
        matching: MatchPolicy,
        budget: &mut Budget,
    ) -> Result<Self, PreparationError> {
        let mut colors = Vec::new();
        budget.reserve(&mut colors, palette.visible.len())?;
        colors.extend(palette.visible.iter().map(|entry| PaletteColor {
            index: entry.index,
            coordinates: converter.coordinates(entry.rgb),
        }));
        Ok(Self { colors, matching })
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
            MatchPolicy::OklchHueArc | MatchPolicy::CielchHueArc => {
                self.scan(coordinates, hue_arc3_squared)
            }
            MatchPolicy::SrgbCompuphase => self.scan(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::CompuPhase)
            }),
            MatchPolicy::SrgbRec601 => self.scan(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::Rec601)
            }),
            MatchPolicy::SrgbRec709 => self.scan(coordinates, |a, b| {
                weighted_rgb_squared(a, b, WeightedRgbMetric::Rec709)
            }),
            MatchPolicy::CielabCiede2000 => self.scan(coordinates, ciede2000_distance),
        }
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
}
