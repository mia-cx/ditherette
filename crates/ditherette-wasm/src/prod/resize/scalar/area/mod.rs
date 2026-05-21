//! Scalar production area resize.
//!
//! This is the first production area kernel. It intentionally mirrors the spec
//! area algorithm while enforcing the production resize boundary: normalized
//! packed RGBA8 input and output. Later benchmark-driven work can precompute
//! spans, reuse accumulators, or split rows without changing the coverage rule.

mod coverage;
mod packed;
mod plan;

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

pub use plan::AreaResizePlan;

// REJECT(perf): A cached `AreaResizePlan` that only stored dimensions and scale
// factors was neutral overall and regressed represented small Celeste box-art
// minification cases by -5.08%/-8.81% in `ditherette-bench run area --baseline
// accepted`. Introduce a plan only with layout metadata that removes hot-loop
// work, not as an API-only wrapper.

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with exact area averaging.
///
/// Each output pixel covers a rectangle in source-pixel space. The output value
/// is the coverage-weighted average of every overlapped source pixel, rounded
/// per channel to `u8`. This function duplicates the spec formula instead of
/// importing it so production remains independent from the oracle.
pub fn resize_area_rgba8_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, Rgba8>) {
    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");

    if resize_area_fast_path_into(source, &mut output) {
        return;
    }

    let plan = AreaResizePlan::new(source.dimensions(), output.dimensions());
    packed::resize_with_plan_into(source, output, &plan);
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with a cached area plan.
///
/// The plan must match the input and output dimensions. Packed-row assertions
/// are development tripwires for the shared production resize boundary.
pub fn resize_area_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    debug_assert_eq!(source.dimensions(), plan.source_dimensions);
    debug_assert_eq!(output.dimensions(), plan.output_dimensions);

    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");

    if resize_area_fast_path_into(source, &mut output) {
        return;
    }

    packed::resize_with_plan_into(source, output, plan);
}

fn resize_area_fast_path_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) -> bool {
    if source.dimensions() == output.dimensions() {
        output.data_mut().copy_from_slice(source.data());
        return true;
    }

    packed::resize_exact_integer_downscale_into(source, output)
        || packed::resize_exact_integer_upscale_into(source, output)
}
