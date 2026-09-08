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

/// Resize one full-width output row range with a triangle filter.
///
/// `full_output_dimensions` is the complete resize target, while `output`
/// stores the local row band starting at absolute output row `y_start`.
pub fn resize_bilinear_rgba8_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: crate::image::ImageDimensions,
    y_start: u32,
    anchor: ResizeAnchor,
) {
    let plan = BilinearResizePlan::new(source.dimensions(), full_output_dimensions, anchor);
    resize_bilinear_rgba8_rows_with_plan_into(source, output, &plan, y_start);
}

/// Resize one full-width output row range with cached bilinear metadata.
pub fn resize_bilinear_rgba8_rows_with_plan_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    y_start: u32,
) {
    common::rgba8::assert_packed_source(source, "bilinear");
    common::rgba8::assert_packed_output(&output, "bilinear");
    assert_eq!(source.dimensions(), plan.source_dimensions());
    assert_row_band_matches_plan(output.dimensions(), plan.output_dimensions(), y_start);
    kernel::resize_packed_rgba8_rows_with_triangle_filter_into(source, output, plan, y_start);
}

/// Execute an absolute row band using only the caller's reserved f32 scratch.
/// Insufficient scratch fails before writes; identity plans need no taps or scratch.
pub fn resize_bilinear_rgba8_rows_with_plan_and_scratch_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    y_start: u32,
    scratch: &mut [f32],
) -> Result<(), crate::prod::contract::failure::Failure> {
    use crate::prod::contract::{
        error::ErrorCode,
        failure::{ErrorPath, Failure},
    };
    common::rgba8::assert_packed_source(source, "bilinear");
    common::rgba8::assert_packed_output(&output, "bilinear");
    assert_eq!(source.dimensions(), plan.source_dimensions());
    assert_row_band_matches_plan(output.dimensions(), plan.output_dimensions(), y_start);
    let required = plan.scratch_elements();
    if scratch.len() < required {
        return Err(Failure::new(
            ErrorCode::MemoryLimit,
            ErrorPath::MemoryLimitBytes,
        ));
    }
    if plan.is_identity() {
        let start = y_start as usize * source.stride().elements();
        let end = start + output.data().len();
        output
            .data_mut()
            .copy_from_slice(&source.data()[start..end]);
        return Ok(());
    }
    kernel::resize_rows_with_scratch_into(source, output, plan, y_start, &mut scratch[..required]);
    Ok(())
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with cached metadata.
pub fn resize_bilinear_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
) {
    common::rgba8::assert_packed_source(source, "bilinear");
    common::rgba8::assert_packed_output(&output, "bilinear");
    assert_eq!(source.dimensions(), plan.source_dimensions());
    assert_eq!(output.dimensions(), plan.output_dimensions());

    if plan.is_identity() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    kernel::resize_packed_rgba8_with_triangle_filter_into(source, output, plan);
}

/// Execute the landed full-call kernel with caller-owned, already-reserved scratch.
pub fn resize_bilinear_rgba8_with_plan_and_scratch_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    scratch: &mut [f32],
) {
    resize_bilinear_with_progress(source, output, plan, scratch, &mut |_| Ok(()))
        .expect("disabled progress cannot fail");
}

/// Reports completed rows from the existing caller-scratch dispatch.
pub(crate) fn resize_bilinear_with_progress(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    scratch: &mut [f32],
    progress: &mut impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
) -> Result<(), crate::prod::contract::failure::Failure> {
    common::rgba8::assert_packed_source(source, "bilinear");
    common::rgba8::assert_packed_output(&output, "bilinear");
    assert_eq!(source.dimensions(), plan.source_dimensions());
    assert_eq!(output.dimensions(), plan.output_dimensions());
    assert_eq!(scratch.len(), plan.scratch_elements());
    if plan.is_identity() {
        output.data_mut().copy_from_slice(source.data());
        return progress(output.dimensions().height());
    }
    kernel::resize_with_progress(source, output, plan, scratch, progress)
}

fn assert_row_band_matches_plan(
    band_dimensions: crate::image::ImageDimensions,
    full_output_dimensions: crate::image::ImageDimensions,
    y_start: u32,
) {
    assert_eq!(band_dimensions.width(), full_output_dimensions.width());
    assert!(
        y_start <= full_output_dimensions.height()
            && band_dimensions.height() <= full_output_dimensions.height() - y_start,
        "row band must fit inside full output dimensions"
    );
}
