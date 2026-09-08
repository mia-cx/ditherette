//! Fallible prepared ownership and allocation-free indexed execution.

use super::matcher::{PaletteColor, PaletteMatcher};
use crate::{
    image::{
        contracts::{IndexedImage, PaletteEntry},
        ImageBuf, ImageView, PaletteIndex8, Rgba8,
    },
    prod::{
        color::packed::{Converter, OrdinarySpace},
        contract::request::{AlphaPolicy, MatchPolicy},
        palette::{allocation::Budget, PalettePixel, PreparationError, PreparedPalette},
    },
};
use std::mem::size_of;

/// Owned palette and matching data. No source buffer or alpha float plane is retained.
pub struct PreparedQuantizer {
    palette: PreparedPalette,
    matcher: PaletteMatcher,
    converter: Converter,
}

impl PreparedQuantizer {
    /// Predicts this record and its heap reservations before constructing tables or allocating.
    pub fn required_capacity_bytes(
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        matching: MatchPolicy,
    ) -> Result<u64, PreparationError> {
        let _ = matching; // Every typed recipe is supported; capacity depends on visible entries.
        let palette = PreparedPalette::required_capacity_bytes(entries, alpha)?;
        let visible = entries
            .iter()
            .take(256)
            .filter(|entry| matches!(entry, PaletteEntry::Color { .. }))
            .count();
        Ok(
            size_of::<Self>() as u64 + palette - size_of::<PreparedPalette>() as u64
                + (visible * size_of::<PaletteColor>()) as u64,
        )
    }

    /// Reserves all palette, warning-text, and matcher capacities without infallible heap growth.
    pub fn try_new(
        entries: &[PaletteEntry],
        alpha: AlphaPolicy,
        matching: MatchPolicy,
        memory_limit: u64,
    ) -> Result<Self, PreparationError> {
        let required = Self::required_capacity_bytes(entries, alpha, matching)?;
        if required > memory_limit {
            return Err(PreparationError::memory());
        }
        let mut budget = Budget::new(memory_limit, size_of::<Self>() as u64)?;
        let palette = PreparedPalette::prepare(entries, alpha, &mut budget)?;
        let converter = Converter::new(
            OrdinarySpace::from_matching(matching).expect("every matching tag has coordinates"),
        );
        let matcher = PaletteMatcher::prepare(&palette, &converter, matching, &mut budget)?;
        Ok(Self {
            palette,
            matcher,
            converter,
        })
    }

    /// Actual record, tables, Vec capacities and warning-string capacities.
    pub fn capacity_bytes(&self) -> u64 {
        size_of::<Self>() as u64 + self.palette.capacity_bytes()
            - size_of::<PreparedPalette>() as u64
            + (self.matcher.colors.capacity() * size_of::<PaletteColor>()) as u64
    }

    pub fn palette(&self) -> &PreparedPalette {
        &self.palette
    }

    pub(crate) fn matcher(&self) -> &PaletteMatcher {
        &self.matcher
    }

    pub(crate) fn converter(&self) -> &Converter {
        &self.converter
    }

    /// Writes one index per source pixel. The caller supplies validated output storage.
    /// Alpha comes from the corresponding source byte; no allocation occurs in this method.
    pub fn quantize_into(&self, source: ImageView<'_, Rgba8>, output: &mut [u8]) {
        let dimensions = source.dimensions();
        assert_eq!(
            output.len(),
            dimensions.pixel_count().expect("valid dimensions")
        );
        for y in 0..dimensions.height() {
            let source = source.row(y).expect("valid source row");
            let row_start = y as usize * dimensions.width_usize();
            let output = &mut output[row_start..row_start + dimensions.width_usize()];
            for (source, output) in source.chunks_exact(4).zip(output) {
                let rgba = [source[0], source[1], source[2], source[3]];
                *output = match self.palette.prepare_pixel(rgba) {
                    PalettePixel::Index(index) => index,
                    PalettePixel::Color(rgb) => {
                        self.matcher.nearest(self.converter.coordinates(rgb)).index
                    }
                };
            }
        }
    }

    /// Moves palette metadata into a complete native result; no copies or allocations occur.
    pub fn into_indexed(self, indices: ImageBuf<PaletteIndex8>) -> IndexedImage {
        self.palette.into_indexed(indices)
    }
}
