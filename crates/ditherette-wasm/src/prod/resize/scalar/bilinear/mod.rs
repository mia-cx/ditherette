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
// NOTE(perf): The bench result/baseline model now distinguishes anisotropic
// scale pairs, and the `bilinear` manifest profile includes width-only and
// height-only cases. Exact correctness passed for the expanded matrix in
// `ditherette-bench run bilinear`.
// NOTE(perf): `prod:resize:bilinear:fast` and the `bilinear-fast` manifest
// profile provide a bounded-correctness lane for future f32, separable, or
// approximate minify paths without weakening the exact `bilinear` profile.
// NOTE(perf): `prod:resize:bilinear:cold` and the `bilinear-cold` manifest
// profile measure `resize_bilinear_rgba8_into` plan construction plus execution
// so uncached API-path changes have coverage outside the hot reused-plan profile.
// NOTE(perf): `resize_bilinear_rgba8_into` checks same-size output before
// building `BilinearResizePlan`; `bilinear-cold` showed identity cases improve
// while represented non-identity cases stayed within noise.
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
    mut output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
) {
    common::rgba8::assert_packed_source(source, "bilinear");
    common::rgba8::assert_packed_output(&output, "bilinear");

    if source.dimensions() == output.dimensions() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

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
