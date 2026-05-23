//! Scalar production bilinear resize.
//!
//! This is an independent production implementation specialized to packed RGBA8.
//! It uses a separable triangle-filter kernel for throughput while staying within
//! the benchmark profile's bounded color-distance correctness contract.

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
// harness shape -> one-shot API coverage -> tap layout -> exact path splits ->
// kernel/local arithmetic.
// NOTE(perf): The bench result/baseline model now distinguishes anisotropic
// scale pairs, and the `bilinear` manifest profile includes width-only and
// height-only cases. Bounded correctness passed for the expanded matrix in
// `ditherette-bench run bilinear`.
// NOTE(perf): `prod:resize:bilinear:scalar` measures
// `resize_bilinear_rgba8_into` plan construction plus execution because the app
// performs one-shot resizes rather than repeated resizes with cached dimensions.
// CLOSE(perf): Cached bilinear-plan tuning does not match the current cold
// one-shot product workload; keep scalar bilinear optimization on
// `ditherette-bench run bilinear` unless repeated same-dimension resizing
// becomes product-representative.
// NOTE(perf): `resize_bilinear_rgba8_into` checks same-size output before
// building `BilinearResizePlan`; the one-shot benchmark covers that API-path
// optimization directly.
// NOTE(perf): Packed RGBA8 assertions are debug-only tripwires, so splitting
// them out of the release hot path has no benchmarkable upside under `run
// bilinear`; keep validation at the public prod entrypoints.

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with a triangle filter.
///
/// This stays independent from `spec::resize::scalar::bilinear`: the spec keeps
/// the direct f64 oracle, while prod uses the separable packed-RGBA8 kernel
/// measured by the bounded bilinear benchmark profile.
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
