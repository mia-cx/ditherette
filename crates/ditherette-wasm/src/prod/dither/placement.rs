//! Palette-independent adaptive placement. Fixed domain derivations live in `placement.md`.

use crate::{
    image::{ImageView, Rgba8},
    prod::{
        color::packed::{Converter, PackedSpace},
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
    let converter = Converter::new(PackedSpace::from_working(space));
    contrast_with_converter(source, x, y, space, radius, &converter)
}

fn contrast_with_converter(
    source: ImageView<'_, Rgba8>,
    x: u32,
    y: u32,
    space: WorkingSpace,
    radius: u32,
    converter: &Converter,
) -> f64 {
    let center = source_color(source, x, y, converter);
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
        let neighbor = source_color(source, nx, ny, converter);
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
    if matches!(placement, Placement::Everywhere {}) {
        return 1.0;
    }
    let converter = Converter::new(PackedSpace::from_working(space));
    placement_mask_with_converter(source, x, y, space, placement, &converter)
}

/// Uses the caller's converter for this same working space without rebuilding its tables.
pub(crate) fn placement_mask_with_converter(
    source: ImageView<'_, Rgba8>,
    x: u32,
    y: u32,
    space: WorkingSpace,
    placement: Placement,
    converter: &Converter,
) -> f32 {
    let Placement::Adaptive {
        radius,
        threshold,
        softness,
    } = placement
    else {
        return 1.0;
    };
    let contrast = contrast_with_converter(source, x, y, space, radius, converter);
    adaptive_mask(contrast, threshold, softness)
}

fn adaptive_mask(contrast: f64, threshold: f32, softness: f32) -> f32 {
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

/// Optional call-owned coordinates. Reservation failure keeps the direct converter path usable.
pub(crate) struct AdaptivePlacementWork {
    coordinates: Vec<[f32; 3]>,
}

impl AdaptivePlacementWork {
    const RECORD_BYTES: u64 = (std::mem::size_of::<Self>()
        + std::mem::size_of::<AdaptivePlacementRows<'_>>()
        + std::mem::size_of::<AdaptivePlacementRow<'_>>()) as u64;

    pub(crate) fn try_new(width: u32, placement: Placement, available: u64) -> Option<Self> {
        if !matches!(placement, Placement::Adaptive { .. }) {
            return None;
        }
        let heap = available.checked_sub(Self::RECORD_BYTES)?;
        let count = (width as usize).checked_mul(AdaptivePlacementRows::ROW_COUNT)?;
        let mut coordinates = crate::prod::resize::common::allocation::CapacityBudget::new(heap)
            .vector(count)
            .ok()?;
        coordinates.resize(count, [0.0; 3]);
        Some(Self { coordinates })
    }

    pub(crate) fn capacity_bytes(&self) -> u64 {
        Self::RECORD_BYTES
            + self.coordinates.capacity() as u64 * std::mem::size_of::<[f32; 3]>() as u64
    }

    pub(crate) fn scratch(&mut self) -> &mut [[f32; 3]] {
        &mut self.coordinates
    }
}

/// Reuses converted source rows in caller-owned scratch for one source, space, and radius.
/// The caller budgets this record and exactly three width-sized coordinate rows, or keeps
/// `placement_mask_with_converter` as its allocation-free fallback. No helper method allocates.
pub(crate) struct AdaptivePlacementRows<'a> {
    source: ImageView<'a, Rgba8>,
    converter: &'a Converter,
    coordinates: &'a mut [[f32; 3]],
    tags: [Option<u32>; 3],
    space: WorkingSpace,
    radius: u32,
    diagonal: f64,
}

impl<'a> AdaptivePlacementRows<'a> {
    pub(crate) const ROW_COUNT: usize = 3;

    /// Binds scratch to an unchanged source and its matching working-space converter.
    /// Scratch contents need no initialization; new row tags invalidate any earlier binding.
    pub(crate) fn new(
        source: ImageView<'a, Rgba8>,
        converter: &'a Converter,
        space: WorkingSpace,
        radius: u32,
        coordinates: &'a mut [[f32; 3]],
    ) -> Self {
        assert_eq!(
            coordinates.len(),
            source.dimensions().width_usize() * Self::ROW_COUNT
        );
        let [r0, r1, r2] = coordinate_domain(space).ranges().map(f64::from);
        Self {
            source,
            converter,
            coordinates,
            tags: [None; Self::ROW_COUNT],
            space,
            radius,
            diagonal: (r0 * r0 + r1 * r1 + r2 * r2).sqrt(),
        }
    }

    /// Prepares the clamped rows above, at, and below y without evicting any needed row.
    /// Radius-one forward scans convert each source row once; other row orders remain exact.
    pub(crate) fn prepare_row(&mut self, y: u32) -> AdaptivePlacementRow<'_> {
        let dimensions = self.source.dimensions();
        let width = dimensions.width_usize();
        let needed = [
            y.saturating_sub(self.radius),
            y,
            y.saturating_add(self.radius).min(dimensions.height() - 1),
        ];
        let slots = needed.map(|row| {
            if let Some(slot) = self.tags.iter().position(|tag| *tag == Some(row)) {
                return slot;
            }
            let slot = self
                .tags
                .iter()
                .position(|tag| tag.is_none_or(|cached| !needed.contains(&cached)))
                .expect("three slots fit every distinct requested row");
            let source = self.source.row(row).expect("validated source row");
            for (pixel, target) in source
                .chunks_exact(4)
                .zip(&mut self.coordinates[slot * width..(slot + 1) * width])
            {
                *target = self.converter.coordinates([pixel[0], pixel[1], pixel[2]]);
            }
            self.tags[slot] = Some(row);
            slot
        });
        let [above, center, below] =
            slots.map(|slot| &self.coordinates[slot * width..(slot + 1) * width]);
        AdaptivePlacementRow {
            above,
            center,
            below,
            space: self.space,
            radius: self.radius,
            diagonal: self.diagonal,
        }
    }
}

/// Borrowed coordinates for one prepared row; x may be visited in either scan direction.
pub(crate) struct AdaptivePlacementRow<'a> {
    above: &'a [[f32; 3]],
    center: &'a [[f32; 3]],
    below: &'a [[f32; 3]],
    space: WorkingSpace,
    radius: u32,
    diagonal: f64,
}

impl AdaptivePlacementRow<'_> {
    /// The unchanged center coordinates can also feed a field's perturbation arithmetic.
    pub(crate) fn center_at(&self, x: u32) -> [f32; 3] {
        self.center[x as usize]
    }

    /// Retains all eight clamped neighbors and the frozen f64 accumulation order.
    pub(crate) fn contrast_at(&self, x: u32) -> f64 {
        let left = x.saturating_sub(self.radius) as usize;
        let right = x
            .saturating_add(self.radius)
            .min(self.center.len() as u32 - 1) as usize;
        let x = x as usize;
        let center = self.center[x];
        let mut total = 0.0;
        for neighbor in [
            self.center[left],
            self.center[right],
            self.above[x],
            self.below[x],
            self.above[left],
            self.above[right],
            self.below[left],
            self.below[right],
        ] {
            total += placement_distance(self.space, center, neighbor);
        }
        total / 8.0 / self.diagonal * 100.0
    }

    /// Applies validated adaptive threshold and softness to this source pixel's contrast.
    pub(crate) fn mask_at(&self, x: u32, threshold: f32, softness: f32) -> f32 {
        adaptive_mask(self.contrast_at(x), threshold, softness)
    }
}

fn source_color(source: ImageView<'_, Rgba8>, x: u32, y: u32, converter: &Converter) -> [f32; 3] {
    let pixel = source
        .pixel(x, y)
        .expect("source coordinates are in bounds");
    let rgb = [pixel[0], pixel[1], pixel[2]];
    converter.coordinates(rgb)
}

#[cfg(test)]
mod tests;
