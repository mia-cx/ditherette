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

pub use plan::NearestResizePlan;

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with nearest sampling.
///
/// This is the convenience entrypoint for one-off production nearest calls. It
/// builds a plan, validates the shared packed-RGBA8 resize boundary, then copies
/// each output pixel from its nearest source pixel.
pub fn resize_nearest_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: alignment::ResizeAnchor,
) {
    let plan = NearestResizePlan::new(source.dimensions(), output.dimensions(), anchor);
    resize_nearest_rgba8_with_plan_into(source, output, &plan);
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with a cached plan.
///
/// This is the benchmark and hot-loop entrypoint. The plan must match the input
/// and output dimensions. Packed-row assertions are intentionally kept here so
/// all callers hit the same production resize boundary before entering the
/// nearest word-copy kernel.
pub fn resize_nearest_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &NearestResizePlan,
) {
    debug_assert_eq!(source.dimensions(), plan.source_dimensions);
    debug_assert_eq!(output.dimensions(), plan.output_dimensions);

    common::rgba8::assert_packed_source(source, "nearest");
    common::rgba8::assert_packed_output(&output, "nearest");

    packed::resize_with_plan_into(source.data(), source.dimensions(), output.data_mut(), plan);
}
