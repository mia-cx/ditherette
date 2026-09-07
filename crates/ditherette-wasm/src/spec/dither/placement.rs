//! Palette-independent adaptive placement. Fixed domain derivations live in `placement.md`.

use crate::spec::contract::request::WorkingSpace;

/// Fixed coordinate enclosure of source RGB8 in one working space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoordinateDomain {
    pub minimum: [f32; 3],
    pub maximum: [f32; 3],
}

impl CoordinateDomain {
    /// Axis widths used by the reference contrast normalization and field scaling.
    pub fn ranges(self) -> [f32; 3] {
        std::array::from_fn(|axis| self.maximum[axis] - self.minimum[axis])
    }
}

/// Returns the version-one fixed domain, independent of image content and palette selection.
pub fn coordinate_domain(space: WorkingSpace) -> CoordinateDomain {
    let (minimum, maximum) = match space {
        WorkingSpace::Srgb | WorkingSpace::LinearRgb => ([0.0; 3], [1.0; 3]),
        WorkingSpace::Ycbcr => ([0.0, -0.000001, -0.000001], [1.0, 1.000001, 1.000001]),
        WorkingSpace::Oklab => ([0.0, -0.55, -0.37], [1.01, 0.60, 0.21]),
        WorkingSpace::Oklch => ([0.0; 3], [1.01, 0.71, std::f32::consts::TAU]),
        WorkingSpace::Cielab => ([0.0, -87.0, -108.0], [101.0, 99.0, 95.0]),
        WorkingSpace::Cielch => ([0.0; 3], [101.0, 147.0, std::f32::consts::TAU]),
    };
    CoordinateDomain { minimum, maximum }
}
