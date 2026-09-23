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
mod sparse;

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
// ACCEPT(perf): Fixed Lanczos3 shrinking by >1x and <=2x on both axes uses
// x-then-y sums. The accepted Celeste 50% browser run gained roughly 12% warm
// speed with 193 changed palette pixels out of 2.7M. Reordered f64 sums can
// change RGBA byte rounding; this path does not promise exact oracle bytes.
// Scalar fixed Lanczos3 uses 64-row output blocks through <=2x shrink and 16-row
// blocks through <=4x. Scratch covers ceil(block_height * source_height /
// output_height) + 6 source rows, capped at source height. Overlapping support
// rows are retained and moved between blocks. Custom fixed kernels use their own
// support diameter instead of six rows. A scratch suffix holds output-width x totals
// and block-height y totals, preserving the raw-sum tap order.
// Row bands retain the accepted <=2x separable dispatch.
// Scale-aware Lanczos2/3 two-axis shrinking uses 64-row blocks, preserving each
// normalized horizontal sum. Scratch covers the largest planned block support;
// preflight uses a conservative bound that includes widened filter support.
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
///
/// See the caller-scratch row adapter when the full output is split between workers.
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

/// Execute an absolute full-width band without allocating. Scratch includes its source support.
/// The caller reserves every simultaneously live band's capacity before dispatch.
pub fn resize_convolution_rgba8_rows_with_plan_and_scratch_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
    scratch: &mut [f64],
) -> Result<(), Failure> {
    resize_convolution_rgba8_rows_with_plan_and_scratch_known_opacity_into(
        source, output, plan, y_start, scratch, false,
    )
}

pub(crate) fn resize_convolution_rgba8_rows_with_plan_and_scratch_known_opacity_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
    scratch: &mut [f64],
    source_opaque: bool,
) -> Result<(), Failure> {
    assert_eq!(source.dimensions(), plan.source_dimensions());
    assert_row_band_matches_plan(output.dimensions(), plan.output_dimensions(), y_start);
    common::rgba8::assert_packed_source(source, "convolution");
    common::rgba8::assert_packed_output(&output, "convolution");
    let required = plan.row_scratch_elements(y_start, output.dimensions().height())?;
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
    kernel::resize_rows_with_scratch_known_opacity_into(
        source,
        output,
        plan,
        y_start,
        Some(&mut scratch[..required]),
        source_opaque,
    );
    Ok(())
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
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: &mut [f64],
    progress: &mut impl FnMut(u32, u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let source_opaque = source
        .data()
        .chunks_exact(rgba8::RGBA8_CHANNELS)
        .all(|pixel| pixel[3] == u8::MAX);
    resize_convolution_with_progress_known_opacity(
        source,
        output,
        plan,
        scratch,
        source_opaque,
        progress,
    )
}

pub(crate) fn resize_convolution_with_progress_known_opacity(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: &mut [f64],
    source_opaque: bool,
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
    kernel::resize_with_progress_known_opacity(
        source,
        output,
        plan,
        Some(&mut scratch[..required]),
        source_opaque,
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
