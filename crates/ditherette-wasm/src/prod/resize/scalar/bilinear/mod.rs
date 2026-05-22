//! Scalar production bilinear resize.
//!
//! This is the initial production copy of the spec bilinear oracle, specialized
//! to packed RGBA8 so it can become the optimization target without importing
//! `spec`. It intentionally keeps the direct triangle-filter math and exact
//! rounding behavior before later benchmark-driven production changes.

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

pub mod alignment;
mod coordinates;
mod filter;
mod kernel;
mod plan;

use alignment::ResizeAnchor;
pub use plan::BilinearResizePlan;

// Current-code bilinear perf-search dependency map:
// harness shape -> cold/hot API coverage -> tap layout -> exact path splits ->
// kernel/local arithmetic. The exact profile currently judges hot reused-plan
// uniform-scale RGBA8 resizes only.
// TODO(perf:harness, rank=1): Add anisotropic resize case identity to the bench
// result/baseline model so bilinear width-only and height-only scale classes can
// be represented without colliding with uniform scales. Benchmark with
// `ditherette-bench run bilinear` after the manifest profile gains configured
// anisotropic cases.
// TODO(perf:harness, rank=2): Add a bounded/fast bilinear subject and manifest
// profile before retesting f32, separable, or approximate minify paths that the
// exact byte profile cannot accept. Judge the new subject with a configured
// Ditherette manifest profile rather than one-off CLI scale/oracle overrides.
// TODO(perf:harness, rank=3): Add a cold one-off bilinear profile that measures
// `resize_bilinear_rgba8_into` plan construction plus execution, because the
// current `bilinear` subject reuses `BilinearResizePlan` and cannot judge API
// changes that only affect uncached calls.
// TODO(perf:api, rank=4, after perf:harness cold-bilinear-profile): Check for
// same-size output before building `BilinearResizePlan` in
// `resize_bilinear_rgba8_into`. Benchmark with the cold bilinear profile so the
// plan-build bypass is actually measured.
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
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
) {
    common::rgba8::assert_packed_source(source, "bilinear");
    common::rgba8::assert_packed_output(&output, "bilinear");
    debug_assert_eq!(source.dimensions(), plan.source_dimensions());
    debug_assert_eq!(output.dimensions(), plan.output_dimensions());

    if plan.is_identity() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    kernel::resize_packed_rgba8_with_triangle_filter_into(source, output, plan);
}
