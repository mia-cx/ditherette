//! Packed RGBA8 production convolution resize.
//!
//! This is the shared production engine for finite-support separable filters
//! such as bicubic and Lanczos. It duplicates the spec formulas instead of
//! importing the oracle, while specializing the implementation to the production
//! packed-RGBA8 boundary.

pub mod alignment;
mod coordinates;
pub mod filter;
mod kernel;
mod plan;

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

pub use alignment::{AxisAlignment, ResizeAnchor};
pub use filter::{ReconstructionKernel, SupportPolicy};
pub use plan::ConvolutionResizePlan;

// Convolution perf-search dependency map:
// correctness/profile coverage -> one-shot vs cached-plan API -> direct vs
// separable path choice -> tap layout/weight metadata -> fixed-support kernels
// -> arithmetic micro-tuning.
// DEFER(perf): Cached-plan convolution benchmarking needs a manifest profile
// that isolates repeated same-dimension plan reuse; the current six convolution
// profiles measure the one-shot public path, so do not tune cached-plan API here.
// REJECT(perf): Splitting Lanczos3 through a y-then-x scratch row preserved
// bounded correctness but regressed `ditherette-bench run lanczos3` by roughly
// 30-38%; keep the direct 2D convolution path until a cheaper reuse strategy is
// benchmarked.
// DEFER(perf): Width-only/height-only convolution specialization needs a
// dedicated anisotropic benchmark profile first; the current six profiles do
// not isolate identity-axis resizes, so a path change would be unmeasured.

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with a separable kernel.
pub fn resize_convolution_rgba8_into<K>(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    kernel: K,
    support_policy: SupportPolicy,
) where
    K: ReconstructionKernel,
{
    common::rgba8::assert_packed_source(source, "convolution");
    common::rgba8::assert_packed_output(&output, "convolution");

    if source.dimensions() == output.dimensions() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    let plan = ConvolutionResizePlan::new(
        source.dimensions(),
        output.dimensions(),
        anchor,
        &kernel,
        support_policy,
    );
    kernel::resize_packed_rgba8_with_convolution_filter_into(source, output, &plan);
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with cached convolution metadata.
pub fn resize_convolution_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
) {
    debug_assert_eq!(source.dimensions(), plan.source_dimensions());
    debug_assert_eq!(output.dimensions(), plan.output_dimensions());

    common::rgba8::assert_packed_source(source, "convolution");
    common::rgba8::assert_packed_output(&output, "convolution");

    if plan.is_identity() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    kernel::resize_packed_rgba8_with_convolution_filter_into(source, output, plan);
}
