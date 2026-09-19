//! Frozen direct-scan matching, wired to the landed production converter.

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
}

impl PaletteMatcher {
    /// Converts visible entries without reordering or removing duplicates.
    pub(crate) fn prepare(
        palette: &PreparedPalette,
        converter: &Converter,
        budget: &mut Budget,
    ) -> Result<Self, PreparationError> {
        let mut colors = Vec::new();
        budget.reserve(&mut colors, palette.visible.len())?;
        colors.extend(palette.visible.iter().map(|entry| PaletteColor {
            index: entry.index,
            coordinates: converter.coordinates(entry.rgb),
        }));
        Ok(Self { colors })
    }

    /// Exact score ties keep the first entry. Transparent-only pixels bypass this scan.
    pub fn nearest(&self, coordinates: [f32; 3]) -> PaletteColor {
        let mut best = self.colors[0];
        let mut best_score = euclidean3_squared(coordinates, best.coordinates);
        for &candidate in &self.colors[1..] {
            let score = euclidean3_squared(coordinates, candidate.coordinates);
            if score < best_score {
                best = candidate;
                best_score = score;
            }
        }
        best
    }
}

/// Squared Euclidean distance, copied without arithmetic changes from the frozen metric.
pub fn euclidean3_squared(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d0 = a[0] - b[0];
    let d1 = a[1] - b[1];
    let d2 = a[2] - b[2];
    d0 * d0 + d1 * d1 + d2 * d2
}
