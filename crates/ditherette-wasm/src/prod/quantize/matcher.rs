//! Frozen direct-scan matching, wired to the landed production converter.

use super::metric::distance_score;
pub use super::metric::euclidean3_squared;
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
