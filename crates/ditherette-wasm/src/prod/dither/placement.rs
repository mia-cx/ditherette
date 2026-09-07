//! Palette-independent adaptive placement. Fixed domain derivations live in `placement.md`.

use crate::{
    image::{ImageView, Rgba8},
    prod::{
        color::packed::rgb8_to_coordinates,
        contract::request::{Placement, WorkingSpace},
    },
};

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

/// Placement distance between forward-converted source colors, not the matching metric.
/// Cylindrical spaces use minimum chroma times the shortest wrapped hue angle.
pub fn placement_distance(space: WorkingSpace, left: [f32; 3], right: [f32; 3]) -> f64 {
    let [l0, l1, l2] = left.map(f64::from);
    let [r0, r1, r2] = right.map(f64::from);
    let dx = l0 - r0;
    let dy = l1 - r1;
    let dz = match space {
        WorkingSpace::Oklch | WorkingSpace::Cielch => {
            let angle = (l2 - r2).abs();
            let wrapped = angle.min(std::f64::consts::TAU - angle);
            l1.min(r1) * wrapped
        }
        _ => l2 - r2,
    };
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Eight-neighbor average distance divided by the fixed domain diagonal, times 100.
/// Coordinates must name a source pixel; radius comes from a validated request.
/// Clamped duplicate neighbors still count. Source alpha and row padding do not.
pub fn contrast_at(
    source: ImageView<'_, Rgba8>,
    x: u32,
    y: u32,
    space: WorkingSpace,
    radius: u32,
) -> f64 {
    let center = source_color(source, x, y, space);
    let radius = i64::from(radius);
    let offsets = [
        (-radius, 0),
        (radius, 0),
        (0, -radius),
        (0, radius),
        (-radius, -radius),
        (radius, -radius),
        (-radius, radius),
        (radius, radius),
    ];
    let dimensions = source.dimensions();
    let mut total = 0.0;
    for (dx, dy) in offsets {
        let nx = (i64::from(x) + dx).clamp(0, i64::from(dimensions.width()) - 1) as u32;
        let ny = (i64::from(y) + dy).clamp(0, i64::from(dimensions.height()) - 1) as u32;
        let neighbor = source_color(source, nx, ny, space);
        total += placement_distance(space, center, neighbor);
    }
    let [r0, r1, r2] = coordinate_domain(space).ranges().map(f64::from);
    let diagonal = (r0 * r0 + r1 * r1 + r2 * r2).sqrt();
    total / 8.0 / diagonal * 100.0
}

/// Returns an everywhere or adaptive mask in [0,1] for a validated placement request.
/// Adaptive threshold and softness use normalized contrast percentage points.
pub fn placement_mask_at(
    source: ImageView<'_, Rgba8>,
    x: u32,
    y: u32,
    space: WorkingSpace,
    placement: Placement,
) -> f32 {
    let Placement::Adaptive {
        radius,
        threshold,
        softness,
    } = placement
    else {
        return 1.0;
    };
    let contrast = contrast_at(source, x, y, space, radius);
    let threshold = f64::from(threshold);
    let softness = f64::from(softness);
    let lower = threshold - softness;
    let upper = threshold + softness;
    if lower == upper {
        return if contrast >= upper { 1.0 } else { 0.0 };
    }
    let t = ((contrast - lower) / (upper - lower)).clamp(0.0, 1.0);
    (t * t * (3.0 - 2.0 * t)) as f32
}

fn source_color(source: ImageView<'_, Rgba8>, x: u32, y: u32, space: WorkingSpace) -> [f32; 3] {
    let pixel = source
        .pixel(x, y)
        .expect("source coordinates are in bounds");
    let rgb = [pixel[0], pixel[1], pixel[2]];
    rgb8_to_coordinates(rgb, space)
}
