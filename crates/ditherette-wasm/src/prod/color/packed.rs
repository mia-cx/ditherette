//! Packed f32 triplets using the landed production conversion helpers.
//! Alpha remains in the corresponding RGBA8 source, never a fourth float channel.

use super::{convert_rgb, ColorSpaceF32, ColorTables};
use crate::{
    image::{ImageView, Rgba8},
    prod::contract::request::MatchPolicy,
};

/// Ordinary coordinate spaces accepted by direct Euclidean quantization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinarySpace {
    Srgb,
    LinearRgb,
    Oklab,
    Cielab,
    Ycbcr,
}

impl OrdinarySpace {
    /// Rejects recipes outside this implementation's five Euclidean pairs.
    pub const fn from_matching(matching: MatchPolicy) -> Option<Self> {
        match matching {
            MatchPolicy::SrgbEuclidean => Some(Self::Srgb),
            MatchPolicy::LinearRgbEuclidean => Some(Self::LinearRgb),
            MatchPolicy::OklabEuclidean => Some(Self::Oklab),
            MatchPolicy::CielabEuclidean => Some(Self::Cielab),
            MatchPolicy::YcbcrEuclidean => Some(Self::Ycbcr),
            _ => None,
        }
    }
}

/// One call's existing byte tables. No heap allocation or new conversion arithmetic.
pub struct Converter {
    target: ColorSpaceF32,
    tables: ColorTables,
}

impl Converter {
    pub fn new(space: OrdinarySpace) -> Self {
        let target = match space {
            OrdinarySpace::Srgb => ColorSpaceF32::Srgb,
            OrdinarySpace::LinearRgb => ColorSpaceF32::LinearSrgb,
            OrdinarySpace::Oklab => ColorSpaceF32::Oklab,
            OrdinarySpace::Cielab => ColorSpaceF32::Cielab,
            OrdinarySpace::Ycbcr => ColorSpaceF32::YCbCr,
        };
        Self {
            target,
            tables: ColorTables::new(),
        }
    }

    /// Converts bytes through the unchanged landed forward conversion.
    pub fn coordinates(&self, [r, g, b]: [u8; 3]) -> [f32; 3] {
        convert_rgb(r, g, b, self.target, &self.tables)
    }

    /// Writes three floats per pixel in row order, leaving the source and alpha untouched.
    pub fn rgba8_into(&self, source: ImageView<'_, Rgba8>, output: &mut [f32]) {
        let dimensions = source.dimensions();
        assert_eq!(
            output.len(),
            dimensions.pixel_count().expect("valid dimensions") * 3
        );
        for y in 0..dimensions.height() {
            let source_row = source.row(y).expect("valid source row");
            let output_start = y as usize * dimensions.width_usize() * 3;
            for x in 0..dimensions.width_usize() {
                let input = x * 4;
                let output = &mut output[output_start + x * 3..output_start + x * 3 + 3];
                output.copy_from_slice(&self.coordinates([
                    source_row[input],
                    source_row[input + 1],
                    source_row[input + 2],
                ]));
            }
        }
    }
}
