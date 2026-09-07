//! Inherited optimized candidate. Historical performance notes below are not S22 promotion evidence.
//! Packed RGBA8 production Lanczos resize.
//!
//! Lanczos is represented as a windowed-sinc reconstruction kernel applied
//! through the production convolution engine.

mod filter;

use std::num::NonZeroU32;

use crate::image::{ImageDimensions, ImageView, ImageViewMut, Rgba8};

use super::convolution_candidate::{
    resize_convolution_rgba8_into, resize_convolution_rgba8_rows_into,
    resize_convolution_rgba8_rows_with_plan_into, resize_convolution_rgba8_with_plan_into,
    ConvolutionResizePlan, ResizeAnchor, SupportPolicy,
};

// Lanczos perf-search dependency map:
// baseline coverage -> Lanczos-specific fixed/scale-aware plan shape -> separable
// path choice -> tap pruning/approximation -> fixed-radius kernels.
// TODO(perf:harness, rank=29): Add an explicit Lanczos comparison profile or
// artifact against the fastest available external CPU Lanczos baseline so the
// current gap is measurable without relying on missing old crate code. Keep the
// existing bounded oracle checks, then benchmark the four manifest Lanczos
// profiles plus the comparison profile.
// ACCEPT(perf): Fixed-policy Lanczos2/Lanczos3 wrappers now dispatch through
// const-radius kernels instead of the dynamic-radius generic path. Correctness
// passed, and `ditherette-bench run lanczos2`/`lanczos3` improved representative
// cases by roughly 6-16%, with larger identity-baseline noise left unweighted.
// TODO(perf:path, rank=31): Add a Lanczos-specific scale-aware downscale path
// selector after the separability experiments settle; strong minification may
// need a different algorithm than generic direct convolution. Verify bounded
// correctness, then benchmark `ditherette-bench run lanczos2-scale-aware` and
// `ditherette-bench run lanczos3-scale-aware`.

/// Reusable Lanczos resize metadata for one source/output shape and radius.
pub struct LanczosResizePlan {
    inner: ConvolutionResizePlan,
}

impl LanczosResizePlan {
    /// Builds reusable coordinate metadata for packed RGBA8 Lanczos resize.
    pub fn new(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
        radius: NonZeroU32,
        support_policy: SupportPolicy,
    ) -> Self {
        let kernel = filter::Lanczos::new(radius);
        Self {
            inner: ConvolutionResizePlan::new(
                source_dimensions,
                output_dimensions,
                anchor,
                &kernel,
                support_policy,
            ),
        }
    }
}

/// Resizes packed RGBA8 `source` into packed RGBA8 `output` with a Lanczos kernel.
pub fn resize_lanczos_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    radius: NonZeroU32,
    support_policy: SupportPolicy,
) {
    resize_convolution_rgba8_into(
        source,
        output,
        anchor,
        filter::Lanczos::new(radius),
        support_policy,
    );
}

/// Resize one full-width output row range with a Lanczos kernel.
///
/// `full_output_dimensions` is the complete resize target, while `output`
/// stores the local row band starting at absolute output row `y_start`.
pub fn resize_lanczos_rgba8_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: ImageDimensions,
    y_start: u32,
    anchor: ResizeAnchor,
    radius: NonZeroU32,
    support_policy: SupportPolicy,
) {
    resize_convolution_rgba8_rows_into(
        source,
        output,
        full_output_dimensions,
        y_start,
        anchor,
        filter::Lanczos::new(radius),
        support_policy,
    );
}

/// Resize one full-width output row range with a cached Lanczos plan.
pub fn resize_lanczos_rgba8_rows_with_plan_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &LanczosResizePlan,
    y_start: u32,
) {
    resize_convolution_rgba8_rows_with_plan_into(source, output, &plan.inner, y_start);
}

/// Resizes packed RGBA8 `source` into packed RGBA8 `output` with a cached Lanczos plan.
pub fn resize_lanczos_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &LanczosResizePlan,
) {
    resize_convolution_rgba8_with_plan_into(source, output, &plan.inner);
}

/// Lanczos2 convenience wrapper for packed RGBA8.
pub fn resize_lanczos2_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    match support_policy {
        SupportPolicy::Fixed => resize_convolution_rgba8_into(
            source,
            output,
            anchor,
            filter::FixedLanczos::<2>::new(),
            support_policy,
        ),
        SupportPolicy::ScaleAware => resize_lanczos_rgba8_into(
            source,
            output,
            anchor,
            NonZeroU32::new(2).unwrap(),
            support_policy,
        ),
    }
}

/// Lanczos3 convenience wrapper for packed RGBA8.
pub fn resize_lanczos3_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    match support_policy {
        SupportPolicy::Fixed => resize_convolution_rgba8_into(
            source,
            output,
            anchor,
            filter::FixedLanczos::<3>::new(),
            support_policy,
        ),
        SupportPolicy::ScaleAware => resize_lanczos_rgba8_into(
            source,
            output,
            anchor,
            NonZeroU32::new(3).unwrap(),
            support_policy,
        ),
    }
}

/// Lanczos2 row-band convenience wrapper for packed RGBA8.
pub fn resize_lanczos2_rgba8_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: ImageDimensions,
    y_start: u32,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    match support_policy {
        SupportPolicy::Fixed => resize_convolution_rgba8_rows_into(
            source,
            output,
            full_output_dimensions,
            y_start,
            anchor,
            filter::FixedLanczos::<2>::new(),
            support_policy,
        ),
        SupportPolicy::ScaleAware => resize_lanczos_rgba8_rows_into(
            source,
            output,
            full_output_dimensions,
            y_start,
            anchor,
            NonZeroU32::new(2).unwrap(),
            support_policy,
        ),
    }
}

/// Lanczos3 row-band convenience wrapper for packed RGBA8.
pub fn resize_lanczos3_rgba8_rows_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    full_output_dimensions: ImageDimensions,
    y_start: u32,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    match support_policy {
        SupportPolicy::Fixed => resize_convolution_rgba8_rows_into(
            source,
            output,
            full_output_dimensions,
            y_start,
            anchor,
            filter::FixedLanczos::<3>::new(),
            support_policy,
        ),
        SupportPolicy::ScaleAware => resize_lanczos_rgba8_rows_into(
            source,
            output,
            full_output_dimensions,
            y_start,
            anchor,
            NonZeroU32::new(3).unwrap(),
            support_policy,
        ),
    }
}
