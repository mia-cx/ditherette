//! Scalar production bilinear resize.
//!
//! This is the initial production copy of the spec bilinear oracle, specialized
//! to packed RGBA8 so it can become the optimization target without importing
//! `spec`. It intentionally keeps the direct triangle-filter math and exact
//! rounding behavior before later benchmark-driven production changes.

use crate::{
    image::{ImageDimensions, ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

pub mod alignment;
mod coordinates;
mod filter;
mod kernel;

use alignment::ResizeAnchor;
use coordinates::{clamp_i64, map_axis_position, support_range};
use filter::triangle_weight;

/// Reusable bilinear resize metadata for one source/output shape.
pub struct BilinearResizePlan {
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    anchor: ResizeAnchor,
    x_taps: Vec<Vec<AxisTap>>,
    y_taps: Vec<Vec<AxisTap>>,
}

#[derive(Clone, Copy)]
struct AxisTap {
    index: usize,
    weight: f64,
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

// TODO(perf:harness, rank=14): Add anisotropic bilinear cases where only width
// or height changes before optimizing dimension-preserving branches; the current
// `bilinear` partial scale group exercises square scale factors only and
// cannot judge horizontal-only or vertical-only fast paths. Benchmark with
// `ditherette-bench run bilinear --oracle spec:resize:bilinear:scalar`
// once the case matrix can express non-uniform output sizes.
// NOTE(perf): Packed RGBA8 assertions are debug-only tripwires, so splitting
// them out of the release hot path has no benchmarkable upside under `run
// bilinear`; keep validation at the public prod entrypoints.

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with a triangle filter.
///
/// This mirrors `spec::resize::scalar::bilinear` without importing it. The
/// current implementation is deliberately direct and exact; production-specific
/// plans and fast paths should land as benchmarked follow-up changes.
pub fn resize_bilinear_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
) {
    let plan = BilinearResizePlan::new(source.dimensions(), output.dimensions(), anchor);
    resize_bilinear_rgba8_with_plan_into(source, output, &plan);
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with cached metadata.
pub fn resize_bilinear_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
) {
    common::rgba8::assert_packed_source(source, "bilinear");
    common::rgba8::assert_packed_output(&output, "bilinear");
    debug_assert_eq!(source.dimensions(), plan.source_dimensions);
    debug_assert_eq!(output.dimensions(), plan.output_dimensions);

    kernel::resize_packed_rgba8_with_triangle_filter_into(source, output, plan);
}
