//! Reusable convolution resize planning.
//!
//! Plans cache axis taps for one source/output shape, support policy, and
//! reconstruction kernel so the pixel kernel can iterate compact support lists
//! without recomputing coordinate math.

use crate::image::ImageDimensions;
use crate::prod::{
    contract::{
        error::ErrorCode,
        failure::{ErrorPath, Failure},
    },
    resize::common::allocation::CapacityBudget,
};

use super::{
    alignment::{self, ResizeAnchor},
    coordinates::{map_axis_position, support_range},
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
// TODO(perf:layout, rank=32, after perf:path lanczos-fixed-dispatch): Store
// fixed-policy Lanczos2/Lanczos3 taps in fixed-size per-output arrays instead of
// nested heap-allocated vectors. This is narrower than the rejected flattened
// generic layout; verify bounded correctness, then benchmark `ditherette-bench
// run lanczos2` and `ditherette-bench run lanczos3`.
// TODO(perf:layout, rank=33): Test bounded Lanczos tap pruning for tiny absolute
// weights in scale-aware downscales, followed by per-output renormalization.
// This intentionally changes exact output, so judge it only with bounded
// correctness in `ditherette-bench run lanczos2-scale-aware` and
// `ditherette-bench run lanczos3-scale-aware`.
// TODO(perf:layout, rank=34, after perf:layout lanczos-fixed-arrays): Test
// precomputing byte offsets for fixed-size Lanczos array taps only. Generic
// nested offsets regressed bicubic/Lanczos2, but fixed arrays may remove the
// extra indirection; benchmark `ditherette-bench run lanczos2` and
// `ditherette-bench run lanczos3`.

/// Reusable convolution resize metadata for one source/output shape and kernel.
pub struct ConvolutionResizePlan {
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
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
    /// Conservative capacity for nested plan vectors and the landed full-call scratch path.
    /// The caller accounts for this plan's inline header and source/output buffers separately.
    pub fn required_bytes<K: ReconstructionKernel>(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        kernel: &K,
        support_policy: SupportPolicy,
    ) -> Result<u64, Failure> {
        let shape = Self {
            source_dimensions,
            output_dimensions,
            support_policy,
            x_taps: Vec::new(),
            y_taps: Vec::new(),
        };
        if shape.is_identity() {
            return Ok(0);
        }
        let mut bytes = 0_u64;
        for (source_len, output_len) in [
            (source_dimensions.width(), output_dimensions.width()),
            (source_dimensions.height(), output_dimensions.height()),
        ] {
            let taps = max_axis_taps(source_len, output_len, kernel, support_policy)? as u64;
            let row_bytes = taps
                .checked_mul(std::mem::size_of::<AxisTap>() as u64)
                .and_then(|n| n.checked_add(std::mem::size_of::<Vec<AxisTap>>() as u64))
                .ok_or_else(memory_limit)?;
            bytes = row_bytes
                .checked_mul(u64::from(output_len))
                .and_then(|n| bytes.checked_add(n))
                .ok_or_else(memory_limit)?;
        }
        bytes
            .checked_add(
                (shape.scratch_elements()? as u64)
                    .checked_mul(8)
                    .ok_or_else(memory_limit)?,
            )
            .ok_or_else(memory_limit)
    }

    /// Build the existing nested tap layout with fallible, capacity-accounted reservations.
    /// Discard the preparation and its ledger together if any reservation fails.
    pub fn try_new<K: ReconstructionKernel>(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
        kernel: &K,
        support_policy: SupportPolicy,
        budget: &mut CapacityBudget,
    ) -> Result<Self, Failure> {
        budget.check_additional(Self::required_bytes(
            source_dimensions,
            output_dimensions,
            kernel,
            support_policy,
        )?)?;
        if source_dimensions == output_dimensions {
            return Ok(Self {
                source_dimensions,
                output_dimensions,
                support_policy,
                x_taps: Vec::new(),
                y_taps: Vec::new(),
            });
        }
        let (x_alignment, y_alignment) = anchor.axes();
        let x_taps = try_axis_taps(
            source_dimensions.width(),
            output_dimensions.width(),
            x_alignment,
            kernel,
            support_policy,
            budget,
        )?;
        let y_taps = try_axis_taps(
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
            kernel,
            support_policy,
            budget,
        )?;
        Ok(Self {
            source_dimensions,
            output_dimensions,
            support_policy,
            x_taps,
            y_taps,
        })
    }

    /// Actual owned heap capacity, including every nested vector header and tap allocation.
    pub fn capacity_bytes(&self) -> u64 {
        [&self.x_taps, &self.y_taps]
            .into_iter()
            .map(|axis| {
                (axis.capacity() * std::mem::size_of::<Vec<AxisTap>>()) as u64
                    + axis
                        .iter()
                        .map(|row| (row.capacity() * std::mem::size_of::<AxisTap>()) as u64)
                        .sum::<u64>()
            })
            .sum()
    }

    /// Caller-owned f64 elements needed by the unchanged full-call dispatch.
    pub fn scratch_elements(&self) -> Result<usize, Failure> {
        if self.same_height() || self.same_width() || !super::kernel::should_use_x_then_y(self) {
            return Ok(0);
        }
        self.source_dimensions
            .height_usize()
            .checked_mul(self.output_dimensions.width_usize())
            .and_then(|n| n.checked_mul(crate::image::rgba8::RGBA8_CHANNELS))
            .ok_or_else(memory_limit)
    }

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

    pub(super) fn support_policy(&self) -> SupportPolicy {
        self.support_policy
    }
}

fn memory_limit() -> Failure {
    Failure::new(ErrorCode::MemoryLimit, ErrorPath::MemoryLimitBytes)
}

fn max_axis_taps<K: ReconstructionKernel>(
    source_len: u32,
    output_len: u32,
    kernel: &K,
    policy: SupportPolicy,
) -> Result<usize, Failure> {
    let support = kernel.radius() * axis_kernel_scale(source_len, output_len, policy);
    let bound = (2.0 * support).ceil() + 2.0;
    if !support.is_finite() || support <= 0.0 || bound >= usize::MAX as f64 {
        return Err(memory_limit());
    }
    Ok(bound as usize)
}

fn try_axis_taps<K: ReconstructionKernel>(
    source_len: u32,
    output_len: u32,
    alignment: alignment::AxisAlignment,
    kernel: &K,
    support_policy: SupportPolicy,
    budget: &mut CapacityBudget,
) -> Result<Vec<Vec<AxisTap>>, Failure> {
    let scale = axis_kernel_scale(source_len, output_len, support_policy);
    let support = kernel.radius() * scale;
    let capacity = max_axis_taps(source_len, output_len, kernel, support_policy)?;
    let mut axis = budget.vector(output_len as usize)?;
    for output_coordinate in 0..output_len {
        let position = map_axis_position(output_coordinate, source_len, output_len, alignment);
        let mut taps = budget.vector(capacity)?;
        for source_coordinate in support_range(position, support) {
            let weight = kernel.weight((source_coordinate as f64 - position) / scale);
            if weight == 0.0 {
                continue;
            }
            let index = source_coordinate.clamp(0, i64::from(source_len) - 1) as usize;
            taps.push(AxisTap { index, weight });
        }
        axis.push(taps);
    }
    Ok(axis)
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

                    let index = source_coordinate.clamp(0, i64::from(source_len) - 1) as usize;
                    Some(AxisTap { index, weight })
                })
                .collect()
        })
        .collect()
}
