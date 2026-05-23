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
// TODO(perf:api, rank=20, after perf:harness area-cached-plan-profile): Retest
// cached area plans now `AreaResizePlan` owns x/y overlap spans instead of only
// dimensions and scale factors. Benchmark `ditherette-bench run area-cached-plan`
// against bounded area correctness.

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

    planned::resize_with_plan_into(source, output, plan);
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
