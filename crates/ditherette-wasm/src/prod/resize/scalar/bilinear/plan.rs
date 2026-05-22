//! Reusable bilinear resize planning.
//!
//! Plans cache axis taps for one source/output shape and anchor so the hot
//! kernel can iterate compact support lists without recomputing coordinate math.

use crate::image::ImageDimensions;

use super::{
    alignment::{self, ResizeAnchor},
    coordinates::{clamp_i64, map_axis_position, support_range},
    filter::triangle_weight,
};

// TODO(perf:layout, rank=5): Flatten `BilinearResizePlan` axis taps into
// contiguous tap arrays plus per-output ranges to remove per-coordinate `Vec`
// allocation and improve metadata locality while preserving duplicate clamped
// edge taps and exact tap order. Verify exactness, then benchmark with
// `ditherette-bench run bilinear`.
// TODO(perf:layout, rank=6, after perf:layout flat-bilinear-axis-taps): Store
// x tap byte offsets and y row coordinates in the flattened plan so the hot
// kernel avoids `index * 4` and repeated integer casts without switching to the
// already-rejected direct packed-source pointer shape. Benchmark with
// `ditherette-bench run bilinear`.
// TODO(perf:layout, rank=7): Test coalescing only duplicate clamped edge taps
// that point at the same source pixel, preserving the direct tap order for
// interior taps. This may fail exact f64 grouping, so gate on exact oracle
// output before benchmarking with `ditherette-bench run bilinear`.
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
    let scale = (f64::from(source_len) / f64::from(output_len)).max(1.0);
    (0..output_len)
        .map(|output_coordinate| {
            let position = map_axis_position(output_coordinate, source_len, output_len, alignment);
            support_range(position, scale)
                .filter_map(|source_coordinate| {
                    let weight = triangle_weight((source_coordinate as f64 - position) / scale);
                    if weight == 0.0 {
                        return None;
                    }

                    let index = clamp_i64(source_coordinate, 0, i64::from(source_len) - 1) as usize;
                    Some(AxisTap { index, weight })
                })
                .collect()
        })
        .collect()
}
