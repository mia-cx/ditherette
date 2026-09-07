//! Scalar production nearest-neighbor resize.
//!
//! This module defines the optimized nearest path for production resize while
//! keeping the readable `spec` oracle independent. Inputs cross the shared prod
//! resize boundary as normalized packed RGBA8; see `prod/resize/README.md` for
//! the no-striding prod rule. This module owns only nearest orchestration.
//! Planning lives in `plan`, and nearest-specific word-copy kernels live in
//! `packed`.

pub mod alignment;
mod packed;
mod plan;
mod scale;

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

pub use plan::{NearestResizePlan, PlanAllocationError};

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with nearest sampling.
///
/// This is the convenience entrypoint for one-off production nearest calls. It
/// builds a plan, validates the shared packed-RGBA8 resize boundary, then copies
/// each output pixel from its nearest source pixel.
pub fn resize_nearest_rgba8_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    anchor: alignment::ResizeAnchor,
) {
    common::rgba8::assert_packed_source(source, "nearest");
    common::rgba8::assert_packed_output(&output, "nearest");

    if source.dimensions() == output.dimensions() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    let plan = NearestResizePlan::new(source.dimensions(), output.dimensions(), anchor);
    packed::resize_with_plan_into(source.data(), source.dimensions(), output.data_mut(), &plan);
}

/// Resize one full-width output row range with nearest sampling.
///
/// `full_output_dimensions` is the complete resize target, while `output`
/// stores the local row band starting at absolute output row `y_start`.
pub fn resize_nearest_rgba8_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: crate::image::ImageDimensions,
    y_start: u32,
    anchor: alignment::ResizeAnchor,
) {
    let plan = NearestResizePlan::new(source.dimensions(), full_output_dimensions, anchor);
    resize_nearest_rgba8_rows_with_plan_into(source, output, &plan, y_start);
}

/// Resize one full-width output row range with cached nearest metadata.
///
/// The plan must match `source` and the complete output dimensions. The local
/// `output` band must have the full output width and fit within `y_start..` of
/// the complete output height.
pub fn resize_nearest_rgba8_rows_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &NearestResizePlan,
    y_start: u32,
) {
    assert_eq!(source.dimensions(), plan.source_dimensions);
    assert_row_band_matches_plan(output.dimensions(), plan.output_dimensions, y_start);
    common::rgba8::assert_packed_source(source, "nearest");
    common::rgba8::assert_packed_output(&output, "nearest");
    let y_end = y_start + output.dimensions().height();
    packed::resize_rows_with_plan_into(
        source.data(),
        source.dimensions(),
        output.data_mut(),
        plan,
        y_start,
        y_end,
    );
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with cached nearest metadata.
///
/// The plan must match the input and output dimensions.
pub fn resize_nearest_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &NearestResizePlan,
) {
    assert_eq!(source.dimensions(), plan.source_dimensions);
    assert_eq!(output.dimensions(), plan.output_dimensions);

    common::rgba8::assert_packed_source(source, "nearest");
    common::rgba8::assert_packed_output(&output, "nearest");

    if source.dimensions() == output.dimensions() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    packed::resize_with_plan_into(source.data(), source.dimensions(), output.data_mut(), plan);
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
