//! Scalar production area resize.
//!
//! This production area kernel enforces the production resize boundary:
//! normalized packed RGBA8 input and output. Exact integer down/up scales stay
//! byte-identical to the spec oracle; fractional planned coverage uses f32
//! accumulation because visual review and bounded RGBA color-distance checks
//! showed negligible drift for a large throughput win.

mod coverage;
mod downscale;
mod exact;
mod plan;
mod planned;

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

pub use plan::AreaResizePlan;

// REJECT(perf): A cached `AreaResizePlan` that only stored dimensions and scale
// factors was neutral overall and regressed represented small Celeste box-art
// minification cases by -5.08%/-8.81% in `ditherette-bench run area`. Introduce
// a plan only with layout metadata that removes hot-loop
// work, not as an API-only wrapper.
// CLOSE(perf): Cached area-plan tuning does not match the current cold one-shot
// product workload; keep area optimization focused on `ditherette-bench run area`
// unless repeated same-dimension resizing becomes product-representative.

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with production area averaging.
///
/// Each output pixel covers a rectangle in source-pixel space. Exact integer
/// scale factors use byte-identical fast paths; fractional scale factors use
/// the same coverage rule with f32 accumulation and bounded oracle drift. This
/// function duplicates the formula instead of importing the spec so production
/// remains independent from the oracle.
pub fn resize_area_rgba8_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, Rgba8>) {
    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");

    if resize_area_fast_path_into(source, &mut output) {
        return;
    }

    let plan = AreaResizePlan::new(source.dimensions(), output.dimensions());
    planned::resize_with_plan_into(source, output, &plan);
}

/// Resize one full-width output row range with production area averaging.
///
/// `full_output_dimensions` is the complete resize target, while `output`
/// stores the local row band starting at absolute output row `y_start`.
pub fn resize_area_rgba8_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: crate::image::ImageDimensions,
    y_start: u32,
) {
    let plan = AreaResizePlan::new(source.dimensions(), full_output_dimensions);
    resize_area_rgba8_rows_with_plan_into(source, output, &plan, y_start);
}

/// Resize one full-width output row range with cached area metadata.
///
/// The plan must match `source` and the complete output dimensions. The local
/// `output` band must have the full output width and fit within `y_start..` of
/// the complete output height.
pub fn resize_area_rgba8_rows_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
    y_start: u32,
) {
    assert_eq!(source.dimensions(), plan.source_dimensions);
    assert_row_band_matches_plan(output.dimensions(), plan.output_dimensions, y_start);
    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");

    if resize_area_rows_fast_path_into(source, &mut output, plan.output_dimensions, y_start) {
        return;
    }

    planned::resize_rows_with_plan_into(source, output, plan, y_start);
}

/// Execute a full-width row band without allocating. The source and plan stay shared.
/// Insufficient caller-owned scratch fails before any output bytes change.
pub fn resize_area_rgba8_rows_with_plan_and_scratch_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
    y_start: u32,
    scratch: &mut [f32],
) -> Result<(), crate::prod::contract::failure::Failure> {
    use crate::prod::contract::{
        error::ErrorCode,
        failure::{ErrorPath, Failure},
    };
    assert_eq!(source.dimensions(), plan.source_dimensions);
    assert_row_band_matches_plan(output.dimensions(), plan.output_dimensions, y_start);
    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");
    let required = plan.scratch_elements();
    if scratch.len() < required {
        return Err(Failure::new(
            ErrorCode::MemoryLimit,
            ErrorPath::MemoryLimitBytes,
        ));
    }
    if !resize_area_rows_fast_path_into(source, &mut output, plan.output_dimensions, y_start) {
        planned::resize_rows_with_scratch_into(
            source,
            output,
            plan,
            y_start,
            &mut scratch[..required],
        );
    }
    Ok(())
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with cached area metadata.
///
/// The plan must match the input and output dimensions. Packed-row assertions
/// are development tripwires for the shared production resize boundary.
pub fn resize_area_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    assert_eq!(source.dimensions(), plan.source_dimensions);
    assert_eq!(output.dimensions(), plan.output_dimensions);

    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");

    if resize_area_fast_path_into(source, &mut output) {
        return;
    }

    planned::resize_with_plan_into(source, output, plan);
}

/// Execute the landed full-call kernel with caller-owned, already-reserved scratch.
pub fn resize_area_rgba8_with_plan_and_scratch_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
    scratch: &mut [f32],
) {
    resize_area_with_progress(source, output, plan, scratch, &mut |_| Ok(()))
        .expect("disabled progress cannot fail");
}

/// Keeps integer fast paths and caller-owned scratch; reports completed fractional rows.
pub(crate) fn resize_area_with_progress(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
    scratch: &mut [f32],
    progress: &mut impl FnMut(u32) -> Result<(), crate::prod::contract::failure::Failure>,
) -> Result<(), crate::prod::contract::failure::Failure> {
    assert_eq!(source.dimensions(), plan.source_dimensions);
    assert_eq!(output.dimensions(), plan.output_dimensions);
    assert_eq!(scratch.len(), plan.scratch_elements());
    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");
    if resize_area_fast_path_into(source, &mut output) {
        return progress(output.dimensions().height());
    }
    planned::resize_with_progress(source, output, plan, scratch, progress)
}

fn resize_area_fast_path_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) -> bool {
    if source.dimensions() == output.dimensions() {
        output.data_mut().copy_from_slice(source.data());
        return true;
    }

    exact::resize_exact_integer_downscale_into(source, output)
        || exact::resize_exact_integer_upscale_into(source, output)
}

fn resize_area_rows_fast_path_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    full_output_dimensions: crate::image::ImageDimensions,
    y_start: u32,
) -> bool {
    if source.dimensions() == full_output_dimensions {
        let row_byte_len =
            full_output_dimensions.width_usize() * crate::image::rgba8::RGBA8_CHANNELS;
        let byte_start = y_start as usize * row_byte_len;
        let byte_end = byte_start + output.dimensions().height_usize() * row_byte_len;
        output
            .data_mut()
            .copy_from_slice(&source.data()[byte_start..byte_end]);
        return true;
    }

    exact::resize_exact_integer_downscale_rows_into(source, output, full_output_dimensions, y_start)
        || exact::resize_exact_integer_upscale_rows_into(
            source,
            output,
            full_output_dimensions,
            y_start,
        )
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
