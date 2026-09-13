//! Exhaustive visible-color matching with stable original palette indices.

use crate::spec::{
    color::rgb8_to_coordinates, contract::request::MatchPolicy, palette::PreparedPalette,
};

use super::metric::distance_score;

/// A visible entry's original index and coordinates in the selected working space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PaletteColor {
    pub index: u8,
    pub coordinates: [f32; 3],
}

/// Ordered coordinates for naive matching. No cache, tree, or lookup table.
#[derive(Debug, Clone, PartialEq)]
pub struct PaletteMatcher {
    pub colors: Vec<PaletteColor>,
    pub matching: MatchPolicy,
}

impl PaletteMatcher {
    /// Converts every visible entry, retaining duplicates and original indices.
    /// A transparent-only palette has no colors; its pixels bypass `nearest`.
    pub fn new(palette: &PreparedPalette, matching: MatchPolicy) -> Self {
        Self {
            colors: palette
                .visible
                .iter()
                .map(|entry| PaletteColor {
                    index: entry.index,
                    coordinates: rgb8_to_coordinates(entry.rgb, matching.space()),
                })
                .collect(),
            matching,
        }
    }

    /// Scans all visible entries. Exact score ties keep the first entry.
    /// Requires a nonempty visible palette and finite coordinates in its working space.
    pub fn nearest(&self, coordinates: [f32; 3]) -> PaletteColor {
        let mut best = self.colors[0];
        let mut best_score = distance_score(coordinates, best.coordinates, self.matching);
        for &candidate in &self.colors[1..] {
            let score = distance_score(coordinates, candidate.coordinates, self.matching);
            if score < best_score {
                best = candidate;
                best_score = score;
            }
        }
        best
    }
}
