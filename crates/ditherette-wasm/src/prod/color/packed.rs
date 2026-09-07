//! Packed f32 triplets using the landed production conversion helpers.
//! Alpha remains in the corresponding RGBA8 source, never a fourth float channel.

use super::{convert_rgb, ColorSpaceF32, ColorTables};
use crate::{
    image::{ImageView, Rgba8},
    prod::contract::request::MatchPolicy,
};

/// Packed coordinate spaces accepted by direct palette matching.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackedSpace {
    Srgb,
    LinearRgb,
    Oklab,
    Oklch,
    Cielab,
    Cielch,
    Ycbcr,
}

/// Compatibility name for the original five-space native adapter.
pub type OrdinarySpace = PackedSpace;

impl PackedSpace {
    /// Maps every valid tagged matching recipe to its coordinate space.
    /// The optional return preserves the S24 native adapter interface.
    pub const fn from_matching(matching: MatchPolicy) -> Option<Self> {
        match matching {
            MatchPolicy::SrgbEuclidean
            | MatchPolicy::SrgbCompuphase
            | MatchPolicy::SrgbRec601
            | MatchPolicy::SrgbRec709 => Some(Self::Srgb),
            MatchPolicy::LinearRgbEuclidean => Some(Self::LinearRgb),
            MatchPolicy::OklabEuclidean => Some(Self::Oklab),
            MatchPolicy::OklchEuclidean
            | MatchPolicy::OklchCircularHue
            | MatchPolicy::OklchHueArc => Some(Self::Oklch),
            MatchPolicy::CielabEuclidean | MatchPolicy::CielabCiede2000 => Some(Self::Cielab),
            MatchPolicy::CielchEuclidean
            | MatchPolicy::CielchCircularHue
            | MatchPolicy::CielchHueArc => Some(Self::Cielch),
            MatchPolicy::YcbcrEuclidean => Some(Self::Ycbcr),
        }
    }
}

/// One call's existing byte tables. No heap allocation or new conversion arithmetic.
pub struct Converter {
    target: ColorSpaceF32,
    tables: ColorTables,
}

impl Converter {
    pub fn new(space: PackedSpace) -> Self {
        let target = match space {
            OrdinarySpace::Srgb => ColorSpaceF32::Srgb,
            OrdinarySpace::LinearRgb => ColorSpaceF32::LinearSrgb,
            OrdinarySpace::Oklab => ColorSpaceF32::Oklab,
            OrdinarySpace::Oklch => ColorSpaceF32::Oklch,
            OrdinarySpace::Cielab => ColorSpaceF32::Cielab,
            OrdinarySpace::Cielch => ColorSpaceF32::Cielch,
            OrdinarySpace::Ycbcr => ColorSpaceF32::YCbCr,
        };
        Self {
            target,
            tables: ColorTables::new(),
        }
    }

    /// Converts bytes through the unchanged landed forward conversion.
    pub fn coordinates(&self, [r, g, b]: [u8; 3]) -> [f32; 3] {
        let [lightness, chroma, hue] = convert_rgb(r, g, b, self.target, &self.tables);
        if !matches!(self.target, ColorSpaceF32::Oklch | ColorSpaceF32::Cielch) {
            return [lightness, chroma, hue];
        }
        // Frozen packed semantics canonicalize exact byte grays and the hue endpoint.
        // The legacy four-channel writer keeps its existing conversion unchanged.
        if r == g && g == b {
            return [lightness, 0.0, 0.0];
        }
        let hue = if hue == 0.0 || hue == std::f32::consts::TAU {
            0.0
        } else {
            hue
        };
        [lightness, chroma, hue]
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
