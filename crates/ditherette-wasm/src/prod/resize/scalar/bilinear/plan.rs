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

// REJECT(perf): Flattening axis taps into contiguous tap arrays plus per-output
// ranges preserved exactness but regressed representative `ditherette-bench run
// bilinear` cases by roughly -4% to -19%, especially anisotropic width-only and
// height-only cases. Keep per-output tap vectors for the hot exact scalar path.
// CLOSE(perf): Storing x byte offsets and y row coordinates depended on the
// rejected flat tap layout, so there is no standalone exact-profile change to
// test here.
// REJECT(perf): Coalescing only duplicate clamped edge taps changed f64 grouping
// and failed exact oracle output in `prod_resize_bilinear` for centered anchors;
// keep duplicate clamped edge taps in direct contribution order.
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
