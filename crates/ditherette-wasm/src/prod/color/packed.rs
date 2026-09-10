//! Packed f32 triplets using the landed production conversion helpers.
//! Alpha remains in the corresponding RGBA8 source, never a fourth float channel.

use super::{convert_rgb, ColorSpaceF32, ColorTables};
use crate::{
    image::{ImageView, Rgba8},
    prod::contract::request::{MatchPolicy, WorkingSpace},
};

/// Uses the landed packed forward converter for palette-free working coordinates.
pub fn rgb8_to_coordinates(rgb: [u8; 3], space: WorkingSpace) -> [f32; 3] {
    Converter::new(PackedSpace::from_working(space)).coordinates(rgb)
}

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
    /// Maps palette-free working coordinates to the existing packed converter.
    pub(crate) const fn from_working(space: WorkingSpace) -> Self {
        match space {
            WorkingSpace::Srgb => Self::Srgb,
            WorkingSpace::LinearRgb => Self::LinearRgb,
            WorkingSpace::Oklab => Self::Oklab,
            WorkingSpace::Oklch => Self::Oklch,
            WorkingSpace::Cielab => Self::Cielab,
            WorkingSpace::Cielch => Self::Cielch,
            WorkingSpace::Ycbcr => Self::Ycbcr,
        }
    }

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
        match self.target {
            ColorSpaceF32::Srgb => Self::write_rgba8(source, output, |[r, g, b]| {
                [
                    self.tables.unit(r),
                    self.tables.unit(g),
                    self.tables.unit(b),
                ]
            }),
            ColorSpaceF32::YCbCr => Self::write_rgba8(source, output, |[r, g, b]| {
                super::srgb8_to_ycbcr(r, g, b, &self.tables)
            }),
            _ => Self::write_rgba8(source, output, |rgb| self.coordinates(rgb)),
        }
    }

    fn write_rgba8(
        source: ImageView<'_, Rgba8>,
        output: &mut [f32],
        coordinates: impl Fn([u8; 3]) -> [f32; 3],
    ) {
        let dimensions = source.dimensions();
        assert_eq!(
            output.len(),
            dimensions.pixel_count().expect("valid dimensions") * 3
        );
        for (y, output_row) in output
            .chunks_exact_mut(dimensions.width_usize() * 3)
            .enumerate()
        {
            let source_row = source.row(y as u32).expect("valid source row");
            for (pixel, triplet) in source_row
                .chunks_exact(4)
                .zip(output_row.chunks_exact_mut(3))
            {
                triplet.copy_from_slice(&coordinates([pixel[0], pixel[1], pixel[2]]));
            }
        }
    }
}
