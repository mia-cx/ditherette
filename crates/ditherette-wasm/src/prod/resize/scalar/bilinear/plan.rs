//! Reusable bilinear resize planning.
//!
//! Plans cache axis taps for one source/output shape and anchor so the hot
//! kernel can iterate compact support lists without recomputing coordinate math.

use crate::image::ImageDimensions;
use crate::prod::{contract::failure::Failure, resize::common::allocation::CapacityBudget};
use std::mem::size_of;

use super::{
    alignment::{self, ResizeAnchor},
    coordinates::{map_axis_position, support_range},
    filter::triangle_weight,
};

// REJECT(perf): Replacing per-tap `{ index, weight: f64 }` storage with an
// old-style per-output `first + normalized f32 weights` layout passed bounded
// correctness but regressed most represented `ditherette-bench run bilinear`
// cases by roughly -2% to -20%, even after removing the dependent per-pixel
// weight sums/divides. Keep indexed taps.
// CLOSE(perf): f32 coordinate planning depended on the rejected normalized f32
// plan shape; keep f64 planning unless a new layout makes planning hot again.
// REJECT(perf): Flattening axis taps into contiguous tap arrays plus per-output
// ranges regressed representative `ditherette-bench run bilinear` cases by
// roughly -4% to -19%, especially anisotropic width-only and height-only cases.
// Keep per-output tap vectors for the scalar path.
// CLOSE(perf): Storing x byte offsets and y row coordinates depended on the
// rejected flat tap layout, so there is no standalone scalar-profile change to
// test here.
// REJECT(perf): Coalescing only duplicate clamped edge taps changed contribution
// grouping and failed the earlier exact oracle output for centered anchors; keep
// duplicate clamped edge taps unless a bounded-profile benchmark proves a win.
/// Reusable bilinear resize metadata for one source/output shape.
pub struct BilinearResizePlan {
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    anchor: ResizeAnchor,
    pub(super) x_taps: Vec<Vec<AxisTap>>,
    pub(super) y_taps: Vec<Vec<AxisTap>>,
}

#[derive(Clone, Copy)]
pub(super) struct AxisTap {
    pub(super) index: usize,
    pub(super) weight: f64,
}

impl BilinearResizePlan {
    /// Complete metadata and one-call scratch reservation, excluding this record.
    pub fn required_bytes(
        source: ImageDimensions,
        output: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> Result<u64, Failure> {
        let (x, y) = anchor.axes();
        let mut bytes =
            u64::from(output.width() + output.height()) * size_of::<Vec<AxisTap>>() as u64;
        for (source_len, output_len, alignment) in [
            (source.width(), output.width(), x),
            (source.height(), output.height(), y),
        ] {
            for coordinate in 0..output_len {
                bytes += taps(source_len, output_len, coordinate, alignment).count() as u64
                    * size_of::<AxisTap>() as u64;
            }
        }
        if source.height() != output.height() {
            bytes += u64::from(source.width()) * 4 * size_of::<f32>() as u64;
        }
        Ok(bytes)
    }

    /// Prepare the existing nested tap layout with fallible, capacity-accounted reservations.
    pub fn try_new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
        budget: &mut CapacityBudget,
    ) -> Result<Self, Failure> {
        budget.check_additional(Self::required_bytes(
            source_dimensions,
            output_dimensions,
            anchor,
        )?)?;
        let (x, y) = anchor.axes();
        let x_taps = try_axis_taps(
            source_dimensions.width(),
            output_dimensions.width(),
            x,
            budget,
        )?;
        let y_taps = try_axis_taps(
            source_dimensions.height(),
            output_dimensions.height(),
            y,
            budget,
        )?;
        Ok(Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_taps,
            y_taps,
        })
    }

    pub fn capacity_bytes(&self) -> u64 {
        let headers = (self.x_taps.capacity() + self.y_taps.capacity()) * size_of::<Vec<AxisTap>>();
        headers as u64
            + self
                .x_taps
                .iter()
                .chain(&self.y_taps)
                .map(|row| row.capacity() as u64 * size_of::<AxisTap>() as u64)
                .sum::<u64>()
    }

    /// The landed height-only and two-axis paths need one source-width f32 row.
    pub fn scratch_elements(&self) -> usize {
        if self.source_dimensions.height() == self.output_dimensions.height() {
            0
        } else {
            self.source_dimensions.width_usize() * 4
        }
    }

    /// Builds reusable coordinate metadata for packed RGBA8 bilinear resize.
    pub fn new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> Self {
        let (x_alignment, y_alignment) = anchor.axes();
        let x_taps = axis_taps(
            source_dimensions.width(),
            output_dimensions.width(),
            x_alignment,
        );
        let y_taps = axis_taps(
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
        );

        Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_taps,
            y_taps,
        }
    }

    /// Return whether this plan was built for the given shape and anchor.
    pub fn matches(
        &self,
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> bool {
        self.source_dimensions == source_dimensions
            && self.output_dimensions == output_dimensions
            && self.anchor == anchor
    }

    pub(super) fn source_dimensions(&self) -> ImageDimensions {
        self.source_dimensions
    }

    pub(super) fn output_dimensions(&self) -> ImageDimensions {
        self.output_dimensions
    }

    pub(super) fn is_identity(&self) -> bool {
        self.source_dimensions == self.output_dimensions
    }
}

fn axis_taps(
    source_len: u32,
    output_len: u32,
    alignment: alignment::AxisAlignment,
) -> Vec<Vec<AxisTap>> {
    (0..output_len)
        .map(|output_coordinate| {
            taps(source_len, output_len, output_coordinate, alignment).collect()
        })
        .collect()
}

fn taps(
    source_len: u32,
    output_len: u32,
    output_coordinate: u32,
    alignment: alignment::AxisAlignment,
) -> impl Iterator<Item = AxisTap> + Clone {
    let scale = (f64::from(source_len) / f64::from(output_len)).max(1.0);
    let position = map_axis_position(output_coordinate, source_len, output_len, alignment);
    support_range(position, scale).filter_map(move |source_coordinate| {
        let weight = triangle_weight((source_coordinate as f64 - position) / scale);
        if weight == 0.0 {
            return None;
        }
        let index = source_coordinate.clamp(0, i64::from(source_len) - 1) as usize;
        Some(AxisTap { index, weight })
    })
}

fn try_axis_taps(
    source_len: u32,
    output_len: u32,
    alignment: alignment::AxisAlignment,
    budget: &mut CapacityBudget,
) -> Result<Vec<Vec<AxisTap>>, Failure> {
    let mut rows = budget.vector(output_len as usize)?;
    for coordinate in 0..output_len {
        let entries = taps(source_len, output_len, coordinate, alignment);
        let mut row = budget.vector(entries.clone().count())?;
        row.extend(entries);
        rows.push(row);
    }
    Ok(rows)
}
