//! Reusable convolution resize planning.
//!
//! Plans cache axis taps for one source/output shape, anchor, support policy,
//! and reconstruction kernel so the pixel kernel can iterate compact support
//! lists without recomputing coordinate math.

use crate::image::ImageDimensions;

use super::{
    alignment::{self, ResizeAnchor},
    coordinates::{clamp_i64, map_axis_position, support_range},
    filter::{axis_kernel_scale, ReconstructionKernel, SupportPolicy},
};

// REJECT(perf): Flattening `Vec<Vec<AxisTap>>` into contiguous taps with
// per-output ranges kept correctness but was neutral for most fixed cases and
// regressed several large scale-aware cases, including `lanczos3-scale-aware`
// 1x/large-upscale by roughly 3-11%.
// REJECT(perf): Storing x byte offsets and y row offsets in nested planned
// taps kept correctness but regressed `bicubic` large cases by roughly 2-9% and
// `lanczos2` fixed cases by roughly 2-5%; keep offset multiplication in the
// direct kernel until a broader layout change makes it free.
// REJECT(perf): Precomputing per-axis weight sums and using their product as
// the pixel denominator changed floating-point rounding versus the current
// contribution-order accumulation; `prod_resize_convolution` failed exact
// Lanczos2 scale-aware output before benchmarking.
// REJECT(perf): Coalescing duplicate clamped edge taps during planning changed
// contribution grouping enough to fail exact `prod_resize_convolution` outputs
// for bicubic, Lanczos2, and Lanczos3 before benchmarking.
// REJECT(perf): Narrowing tap weights to f32 failed exact
// `prod_resize_convolution` outputs for bicubic, Lanczos2, and Lanczos3 before
// benchmarking; keep f64 tap weights while exact unit coverage remains.
// REJECT(perf): Narrowing tap indices to u32 kept correctness and improved
// Lanczos profiles, but regressed bicubic scale-aware and several bicubic cases
// in `ditherette-bench run bicubic*`; keep usize tap indices.
// CLOSE(perf): Compact tap storage attempts either failed exact unit coverage or
// regressed representative bicubic/convolution profiles; keep nested usize/f64
// taps until a different parent layout exists.
// CLOSE(perf): Pre-normalized per-axis f32 weights or per-output reciprocals
// depended on compact tap storage and relaxed precision; compact layout is
// rejected, and f32 tap weights fail exact unit coverage.

/// Reusable convolution resize metadata for one source/output shape and kernel.
pub struct ConvolutionResizePlan {
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
    pub(super) x_taps: Vec<Vec<AxisTap>>,
    pub(super) y_taps: Vec<Vec<AxisTap>>,
}

#[derive(Clone, Copy)]
pub(super) struct AxisTap {
    pub(super) index: usize,
    pub(super) weight: f64,
}

impl ConvolutionResizePlan {
    /// Builds reusable coordinate metadata for packed RGBA8 convolution resize.
    pub fn new<K>(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
        kernel: &K,
        support_policy: SupportPolicy,
    ) -> Self
    where
        K: ReconstructionKernel,
    {
        let (x_alignment, y_alignment) = anchor.axes();
        let x_taps = axis_taps(
            source_dimensions.width(),
            output_dimensions.width(),
            x_alignment,
            kernel,
            support_policy,
        );
        let y_taps = axis_taps(
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
            kernel,
            support_policy,
        );

        Self {
            source_dimensions,
            output_dimensions,
            anchor,
            support_policy,
            x_taps,
            y_taps,
        }
    }

    pub(super) fn source_dimensions(&self) -> ImageDimensions {
        self.source_dimensions
    }

    pub(super) fn output_dimensions(&self) -> ImageDimensions {
        self.output_dimensions
    }

    pub(super) fn is_identity(&self) -> bool {
        self.source_dimensions == self.output_dimensions
    }

    pub(super) fn same_width(&self) -> bool {
        self.source_dimensions.width() == self.output_dimensions.width()
    }

    pub(super) fn same_height(&self) -> bool {
        self.source_dimensions.height() == self.output_dimensions.height()
    }

    #[allow(dead_code)]
    pub(super) fn anchor(&self) -> ResizeAnchor {
        self.anchor
    }

    #[allow(dead_code)]
    pub(super) fn support_policy(&self) -> SupportPolicy {
        self.support_policy
    }
}

fn axis_taps<K>(
    source_len: u32,
    output_len: u32,
    alignment: alignment::AxisAlignment,
    kernel: &K,
    support_policy: SupportPolicy,
) -> Vec<Vec<AxisTap>>
where
    K: ReconstructionKernel,
{
    let scale = axis_kernel_scale(source_len, output_len, support_policy);
    let support = kernel.radius() * scale;

    (0..output_len)
        .map(|output_coordinate| {
            let position = map_axis_position(output_coordinate, source_len, output_len, alignment);
            support_range(position, support)
                .filter_map(|source_coordinate| {
                    let weight = kernel.weight((source_coordinate as f64 - position) / scale);
                    if weight == 0.0 {
                        return None;
                    }

                    let index = clamp_i64(source_coordinate, 0, i64::from(source_len) - 1) as usize;
                    Some(AxisTap { index, weight })
                })
                .collect()
        })
        .collect()
}
