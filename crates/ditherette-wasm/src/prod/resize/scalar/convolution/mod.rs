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
    image::{rgba8, ImageView, ImageViewMut, Rgba8},
    prod::{
        contract::{
            error::ErrorCode,
            failure::{ErrorPath, Failure},
        },
        resize::common,
    },
};

pub use alignment::{AxisAlignment, ResizeAnchor};
pub use filter::{ReconstructionKernel, SupportPolicy};
pub use plan::ConvolutionResizePlan;

// Convolution perf-search dependency map:
// correctness/profile coverage -> direct vs separable path choice -> tap
// layout/weight metadata -> fixed-support kernels -> arithmetic micro-tuning.
// CLOSE(perf): Cached-plan convolution tuning does not match the current product
// workload. The app performs cold, one-shot resizes rather than repeated
// same-dimension resizes, so benchmark and tune the one-shot public path instead.
// REJECT(perf): Splitting Lanczos3 through a y-then-x scratch row preserved
// bounded correctness but regressed `ditherette-bench run lanczos3` by roughly
// 30-38%; keep the direct 2D convolution path until a cheaper reuse strategy is
// benchmarked.
// ACCEPT(perf): X-then-y separable convolution for scale-aware x-downscales
// preserves exact small-image tests and bounded benchmark correctness, while
// improving scale-aware bicubic/Lanczos downscales by roughly 60-86% for square
// fixtures and up to roughly 645% for strong Lanczos3 minification.
// REJECT(perf): Streaming x-then-y scratch rows preserved bounded correctness
// but regressed `ditherette-bench run lanczos3-scale-aware` representative
// downscales by roughly 30-56% versus the accepted full-scratch x-then-y path.
// ACCEPT(perf): Width-only and height-only convolution resizes now skip the
// identity axis. Correctness passed, and the `*-anisotropic` convolution
// profiles improved from small bicubic gains to large Lanczos wins.

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
    kernel::resize_packed_rgba8_with_convolution_filter_into(source, output, &plan, None);
}

/// Resize one full-width output row range with a separable convolution kernel.
///
/// `full_output_dimensions` is the complete resize target, while `output`
/// stores the local row band starting at absolute output row `y_start`.
pub fn resize_convolution_rgba8_rows_into<K>(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: crate::image::ImageDimensions,
    y_start: u32,
    anchor: ResizeAnchor,
    kernel: K,
    support_policy: SupportPolicy,
) where
    K: ReconstructionKernel,
{
    common::rgba8::assert_packed_source(source, "convolution");
    common::rgba8::assert_packed_output(&output, "convolution");
    assert_row_band_matches_plan(output.dimensions(), full_output_dimensions, y_start);
    let plan = ConvolutionResizePlan::new(
        source.dimensions(),
        full_output_dimensions,
        anchor,
        &kernel,
        support_policy,
    );
    kernel::resize_packed_rgba8_rows_with_convolution_filter_into(source, output, &plan, y_start);
}

/// Resize one full-width output row range with cached convolution metadata.
pub fn resize_convolution_rgba8_rows_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
) {
    common::rgba8::assert_packed_source(source, "convolution");
    common::rgba8::assert_packed_output(&output, "convolution");
    assert_row_band_matches_plan(output.dimensions(), plan.output_dimensions(), y_start);
    // Fallible identity plans need no taps. Copy only the requested logical row band.
    if plan.is_identity() && plan.x_taps.is_empty() {
        assert_eq!(source.dimensions(), plan.source_dimensions());
        let start = y_start as usize * source.stride().elements();
        let end = start + output.data().len();
        output
            .data_mut()
            .copy_from_slice(&source.data()[start..end]);
        return;
    }
    kernel::resize_packed_rgba8_rows_with_convolution_filter_into(source, output, plan, y_start);
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with cached convolution metadata.
pub fn resize_convolution_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
) {
    assert_eq!(source.dimensions(), plan.source_dimensions());
    assert_eq!(output.dimensions(), plan.output_dimensions());

    common::rgba8::assert_packed_source(source, "convolution");
    common::rgba8::assert_packed_output(&output, "convolution");

    if plan.is_identity() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    kernel::resize_packed_rgba8_with_convolution_filter_into(source, output, plan, None);
}

/// Execute the landed full-call paths using caller-owned scratch without allocating.
/// Views must be packed and match the plan. All checks finish before output is modified.
pub fn resize_convolution_rgba8_with_plan_and_scratch_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: &mut [f64],
) -> Result<(), Failure> {
    resize_convolution_with_progress(source, output, plan, scratch, &mut |_, _| Ok(()))
}

/// Counts filtered source rows plus output rows for the existing x-then-y dispatch.
pub(crate) fn resize_convolution_with_progress(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: &mut [f64],
    progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    if source.dimensions() != plan.source_dimensions()
        || !rgba8::is_packed_stride(source.dimensions(), source.stride())
        || source.dimensions().storage_len::<Rgba8>().ok() != Some(source.data().len())
    {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::Source));
    }
    if output.dimensions() != plan.output_dimensions()
        || !rgba8::is_packed_stride(output.dimensions(), output.stride())
        || output.dimensions().storage_len::<Rgba8>().ok() != Some(output.data().len())
    {
        return Err(Failure::new(ErrorCode::InvalidImage, ErrorPath::Output));
    }
    let required = plan.scratch_elements()?;
    if scratch.len() < required {
        return Err(Failure::new(
            ErrorCode::MemoryLimit,
            ErrorPath::MemoryLimitBytes,
        ));
    }
    if plan.is_identity() {
        output.data_mut().copy_from_slice(source.data());
        return progress(output.dimensions().height(), output.dimensions().height());
    }
    let total = kernel::work_rows(plan);
    progress(0, total)?;
    kernel::resize_with_progress(
        source,
        output,
        plan,
        Some(&mut scratch[..required]),
        &mut |completed| progress(completed, total),
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
