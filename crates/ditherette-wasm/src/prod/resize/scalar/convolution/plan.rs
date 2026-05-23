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

// TODO(perf:layout, rank=5, after perf:path convolution-direct-vs-separable):
// If the direct convolution kernel remains hot, flatten `Vec<Vec<AxisTap>>` into
// contiguous tap storage plus per-output ranges to reduce allocation count and
// pointer chasing. Benchmark fixed and scale-aware bicubic/Lanczos profiles.
// TODO(perf:layout, rank=6, after perf:layout convolution-flat-taps): Store x
// byte offsets and y row offsets in planned taps, or retest on the current tap
// layout if flattening is rejected, to remove per-contribution offset
// multiplication from the direct kernel. Benchmark the six convolution profiles.
// TODO(perf:layout, rank=7, after perf:layout convolution-flat-taps): Precompute
// per-axis weight sums or per-output reciprocal weights in the plan so the hot
// pixel loop does not rebuild `total_weight`; verify exact or bounded oracle
// output per the selected correctness contract, then benchmark the six profiles.
// TODO(perf:kernel, rank=10, after perf:layout convolution-flat-taps): Coalesce
// duplicate clamped edge taps during planning to avoid repeated edge-pixel
// contributions; accept only if the configured convolution oracles still pass,
// then benchmark all fixed and scale-aware convolution profiles.
// TODO(perf:layout, rank=13, after perf:harness convolution-correctness-contract):
// If convolution filters move to bounded correctness, test compact tap storage
// such as `u32` offsets plus `f32` weights/reciprocals to reduce plan memory and
// cache pressure. Benchmark all six convolution profiles against bounded oracles.

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
