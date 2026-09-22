//! Packed RGBA8 convolution kernel.
//!
//! The kernel consumes preplanned x/y support taps. Direct paths preserve the
//! oracle's contribution grouping; separable paths accept bounded rounding differences.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};
use crate::prod::contract::failure::Failure;

use super::{
    filter::SupportPolicy,
    plan::{AxisTap, ConvolutionResizePlan},
};

const X_THEN_Y_MIN_SOURCE_PIXELS: u64 = 10_000;
fn block_height(plan: &ConvolutionResizePlan) -> usize {
    if plan.support_policy() == SupportPolicy::ScaleAware {
        return 64;
    }
    let source = plan.source_dimensions();
    let output = plan.output_dimensions();
    if u64::from(source.width()) <= 2 * u64::from(output.width())
        && u64::from(source.height()) <= 2 * u64::from(output.height())
    {
        64
    } else {
        16
    }
}

/// Fixed raw-sum blocks retain one total per output column and current block row.
pub(super) fn block_weight_elements(plan: &ConvolutionResizePlan) -> usize {
    if should_use_fixed_blocks(plan) {
        plan.output_dimensions().width_usize() + block_height(plan)
    } else {
        0
    }
}

/// Count actual block support, or conservatively bound it before taps are planned.
pub(super) fn block_source_rows(plan: &ConvolutionResizePlan) -> usize {
    let source_height = u64::from(plan.source_dimensions().height());
    let output_height = u64::from(plan.output_dimensions().height());
    let support_rows = if plan.support_policy() == SupportPolicy::ScaleAware {
        if !plan.y_taps.is_empty() {
            return plan
                .y_taps
                .chunks(block_height(plan))
                .map(|taps| source_rows(taps).len())
                .max()
                .unwrap_or(0);
        }
        (2.0 * plan.scale_aware_block_radius * source_height as f64 / output_height as f64).ceil()
            as u64
            + 2
    } else {
        6
    };
    ((source_height * block_height(plan) as u64).div_ceil(output_height) + support_rows)
        .min(source_height) as usize
}

// CLOSE(perf): Scratch ownership tuning depended on a winning separable path;
// the tested Lanczos3 y-then-x scratch row preserved bounded correctness but
// regressed `ditherette-bench run lanczos3` by roughly 30-38%.
// ACCEPT(perf): Fixed 4x4 and 6x6 tap-count dispatch lets LLVM specialize the
// hot convolution loops while preserving exact output; fixed bicubic, Lanczos2,
// and Lanczos3 non-identity cases improved by roughly 3-7%.
// REJECT(perf): Extracting explicit RGBA accumulation/final-rounding helpers
// preserved exact convolution output but regressed `ditherette-bench run bicubic`
// representative non-identity cases by roughly 5-7%, so keep the compact channel
// loops that LLVM optimizes better.
// CLOSE(perf): SIMD convolution remains blocked until a better parent layout
// exists; compact tap storage attempts failed exact coverage or regressed
// representative bicubic/convolution profiles.
// CLOSE(perf): f32 convolution accumulation depends on relaxed precision and an
// explicit compact tap layout; f32 tap weights already failed exact unit
// coverage, so keep the f64 scalar kernel for exact-output callers.
// ACCEPT(perf): One-axis convolution kernels skip the identity axis for
// width-only and height-only resizes. Correctness passed, and the six
// `*-anisotropic` profiles improved from small bicubic gains to large Lanczos
// wins.
// TODO(perf:kernel, rank=36, after perf:layout lanczos-fixed-arrays): Test
// Lanczos2/Lanczos3 fixed-radius kernels that unroll channel and tap loops
// against fixed-size tap arrays. Verify bounded correctness, then benchmark
// `ditherette-bench run lanczos2` and `ditherette-bench run lanczos3`.

pub(super) fn resize_packed_rgba8_with_convolution_filter_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: Option<&mut [f64]>,
) {
    resize_with_progress(source, output, plan, scratch, &mut |_| Ok(()))
        .expect("disabled progress cannot fail");
}

pub(super) fn work_rows(plan: &ConvolutionResizePlan) -> u32 {
    if should_use_blocks(plan) {
        return plan.output_dimensions().height()
            + plan
                .y_taps
                .chunks(block_height(plan))
                .map(|taps| source_rows(taps).len() as u32)
                .sum::<u32>();
    }
    plan.output_dimensions().height()
        + if !plan.same_height() && !plan.same_width() && should_use_x_then_y(plan) {
            plan.source_dimensions().height()
        } else {
            0
        }
}

pub(super) fn resize_with_progress(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: Option<&mut [f64]>,
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let opaque = source
        .data()
        .chunks_exact(rgba8::RGBA8_CHANNELS)
        .all(|pixel| pixel[3] == u8::MAX);
    resize_with_progress_known_opacity(source, output, plan, scratch, opaque, progress)
}

pub(super) fn resize_with_progress_known_opacity(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: Option<&mut [f64]>,
    opaque: bool,
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    if opaque {
        resize_with_progress_channels::<3>(source, output, plan, scratch, progress)
    } else {
        resize_with_progress_channels::<4>(source, output, plan, scratch, progress)
    }
}

fn resize_with_progress_channels<const CHANNELS: usize>(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    scratch: Option<&mut [f64]>,
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let source_row_byte_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    if plan.same_height() {
        resize_horizontal_only_into::<CHANNELS>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.x_taps,
            progress,
        )?;
        return Ok(());
    }

    if plan.same_width() {
        resize_vertical_only_into::<CHANNELS>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.y_taps,
            progress,
        )?;
        return Ok(());
    }

    if should_use_fixed_blocks(plan) {
        resize_x_then_y_blocks_into::<CHANNELS, true>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            plan,
            scratch,
            progress,
        )?;
        return Ok(());
    }

    if should_use_scale_aware_blocks(plan) {
        resize_x_then_y_blocks_into::<CHANNELS, false>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            plan,
            scratch,
            progress,
        )?;
        return Ok(());
    }

    if should_use_x_then_y(plan) {
        resize_x_then_y_into::<CHANNELS>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.x_taps,
            &plan.y_taps,
            scratch,
            progress,
        )?;
        return Ok(());
    }

    for (y, (output_row, y_taps)) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .zip(&plan.y_taps)
        .enumerate()
    {
        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_taps)
        {
            write_convolution_pixel::<CHANNELS>(
                output_pixel,
                source_data,
                source_row_byte_len,
                x_taps,
                y_taps,
            );
        }
        progress(y as u32 + 1)?;
    }
    Ok(())
}

pub(super) fn resize_packed_rgba8_rows_with_convolution_filter_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
) {
    resize_rows_with_scratch_into(source, output, plan, y_start, None);
}

pub(super) fn resize_rows_with_scratch_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
    scratch: Option<&mut [f64]>,
) {
    resize_rows_with_scratch_known_opacity_into(source, output, plan, y_start, scratch, false);
}

pub(super) fn resize_rows_with_scratch_known_opacity_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
    scratch: Option<&mut [f64]>,
    opaque: bool,
) {
    if opaque {
        resize_rows_with_scratch_channels_into::<3>(source, output, plan, y_start, scratch);
    } else {
        resize_rows_with_scratch_channels_into::<{ rgba8::RGBA8_CHANNELS }>(
            source, output, plan, y_start, scratch,
        );
    }
}

fn resize_rows_with_scratch_channels_into<const CHANNELS: usize>(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
    scratch: Option<&mut [f64]>,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = plan.output_dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();
    let y_start = y_start as usize;

    if plan.same_height() {
        let start = y_start * source_row_byte_len;
        let source_slice = &source_data[start..];
        resize_horizontal_only_into::<CHANNELS>(
            source_slice,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.x_taps,
            &mut |_| Ok(()),
        )
        .expect("disabled progress cannot fail");
        return;
    }

    if plan.same_width() {
        let band_height = output.dimensions().height_usize();
        resize_vertical_only_into::<CHANNELS>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.y_taps[y_start..y_start + band_height],
            &mut |_| Ok(()),
        )
        .expect("disabled progress cannot fail");
        return;
    }

    let band_height = output.dimensions().height_usize();
    let y_taps = &plan.y_taps[y_start..y_start + band_height];
    if should_use_x_then_y(plan) {
        resize_x_then_y_rows_into::<CHANNELS, false>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.x_taps,
            y_taps,
            scratch,
            None,
        );
        return;
    }

    for (output_row, y_taps) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .zip(y_taps)
    {
        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_taps)
        {
            write_convolution_pixel::<CHANNELS>(
                output_pixel,
                source_data,
                source_row_byte_len,
                x_taps,
                y_taps,
            );
        }
    }
}

pub(super) fn should_use_x_then_y(plan: &ConvolutionResizePlan) -> bool {
    let source_dimensions = plan.source_dimensions();
    // Accepted Lanczos3 tradeoff: reordered f64 sums can change final byte rounding.
    if fixed_shrink_within(plan, 2) {
        return true;
    }
    plan.support_policy() == SupportPolicy::ScaleAware
        && source_dimensions.width() > plan.output_dimensions().width()
        && u64::from(source_dimensions.width()) * u64::from(source_dimensions.height())
            >= X_THEN_Y_MIN_SOURCE_PIXELS
}

pub(super) fn should_use_fixed_blocks(plan: &ConvolutionResizePlan) -> bool {
    fixed_shrink_within(plan, 4)
}

pub(super) fn should_use_blocks(plan: &ConvolutionResizePlan) -> bool {
    should_use_fixed_blocks(plan) || should_use_scale_aware_blocks(plan)
}

fn should_use_scale_aware_blocks(plan: &ConvolutionResizePlan) -> bool {
    plan.support_policy() == SupportPolicy::ScaleAware
        && plan.scale_aware_block_radius > 0.0
        && plan.source_dimensions().height() > plan.output_dimensions().height()
        && should_use_x_then_y(plan)
}

fn fixed_shrink_within(plan: &ConvolutionResizePlan, max_ratio: u64) -> bool {
    let source = plan.source_dimensions();
    let output = plan.output_dimensions();
    plan.support_policy() == SupportPolicy::Fixed
        && plan.allows_fixed_separable_shrink
        && source.width() > output.width()
        && source.height() > output.height()
        && u64::from(source.width()) <= max_ratio * u64::from(output.width())
        && u64::from(source.height()) <= max_ratio * u64::from(output.height())
}

fn resize_x_then_y_blocks_into<const CHANNELS: usize, const RAW_SUMS: bool>(
    source: &[u8],
    output: &mut [u8],
    source_row_byte_len: usize,
    output_row_byte_len: usize,
    plan: &ConvolutionResizePlan,
    scratch: Option<&mut [f64]>,
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let mut owned_scratch;
    let scratch = match scratch {
        Some(scratch) => scratch,
        None => {
            owned_scratch = vec![0.0; plan.scratch_elements().expect("valid scratch geometry")];
            &mut owned_scratch
        }
    };
    let mut completed = 0;
    let block_height = block_height(plan);
    let weight_elements = if RAW_SUMS {
        plan.x_taps.len() + block_height
    } else {
        0
    };
    let (scratch, weights) = scratch.split_at_mut(scratch.len() - weight_elements);
    let (x_weights, y_weights) = if RAW_SUMS {
        let (x_weights, y_weights) = weights.split_at_mut(plan.x_taps.len());
        for (weight, taps) in x_weights.iter_mut().zip(&plan.x_taps) {
            *weight = taps.iter().map(|tap| tap.weight).sum();
        }
        (x_weights, y_weights)
    } else {
        weights.split_at_mut(0)
    };
    let mut prepared_support = None;
    for (output_block, y_taps) in output
        .chunks_mut(output_row_byte_len * block_height)
        .zip(plan.y_taps.chunks(block_height))
    {
        let support_rows = source_rows(y_taps).len();
        debug_assert!(
            support_rows * plan.output_dimensions().width_usize() * CHANNELS <= scratch.len()
        );
        if RAW_SUMS {
            for (weight, taps) in y_weights.iter_mut().zip(y_taps) {
                *weight = taps.iter().fold(0.0, |total, tap| total + tap.weight);
            }
        }
        let support = source_rows(y_taps);
        prepare_horizontal_scratch_rows::<CHANNELS, RAW_SUMS>(
            source,
            source_row_byte_len,
            &plan.x_taps,
            scratch,
            support.clone(),
            prepared_support,
        );
        prepared_support = Some(support.clone());
        // Fixed Lanczos3 uses raw sums; scale-aware filters retain normalized scratch.
        write_x_then_y_rows_from_scratch::<CHANNELS, RAW_SUMS>(
            output_block,
            output_row_byte_len,
            y_taps,
            scratch,
            support.start,
            RAW_SUMS.then_some((&*x_weights, &*y_weights)),
        );
        completed += (support_rows + y_taps.len()) as u32;
        progress(completed)?;
    }
    Ok(())
}

fn resize_x_then_y_into<const CHANNELS: usize>(
    source: &[u8],
    output: &mut [u8],
    source_row_byte_len: usize,
    output_row_byte_len: usize,
    x_taps_by_output: &[Vec<AxisTap>],
    y_taps_by_output: &[Vec<AxisTap>],
    scratch: Option<&mut [f64]>,
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let source_height = source.len() / source_row_byte_len;
    let output_width = output_row_byte_len / rgba8::RGBA8_CHANNELS;
    let scratch_row_len = output_width * CHANNELS;
    let mut owned_scratch;
    let scratch = match scratch {
        Some(scratch) => scratch,
        None => {
            owned_scratch = vec![0.0; source_height * scratch_row_len];
            &mut owned_scratch
        }
    };

    for (y, (source_row, scratch_row)) in source
        .chunks_exact(source_row_byte_len)
        .zip(scratch.chunks_exact_mut(scratch_row_len))
        .enumerate()
    {
        for (scratch_pixel, x_taps) in scratch_row.chunks_exact_mut(CHANNELS).zip(x_taps_by_output)
        {
            write_horizontal_scratch_pixel::<CHANNELS, false>(scratch_pixel, source_row, x_taps);
        }
        progress(y as u32 + 1)?;
    }

    for (y, (output_row, y_taps)) in output
        .chunks_exact_mut(output_row_byte_len)
        .zip(y_taps_by_output)
        .enumerate()
    {
        for output_x in 0..output_width {
            let output_start = output_x * rgba8::RGBA8_CHANNELS;
            write_vertical_scratch_pixel::<CHANNELS>(
                &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS],
                &scratch,
                scratch_row_len,
                output_x * CHANNELS,
                y_taps,
            );
        }
        progress((source_height + y + 1) as u32)?;
    }
    Ok(())
}

fn resize_x_then_y_rows_into<const CHANNELS: usize, const RAW_SUMS: bool>(
    source: &[u8],
    output: &mut [u8],
    source_row_byte_len: usize,
    output_row_byte_len: usize,
    x_taps_by_output: &[Vec<AxisTap>],
    y_taps_by_output: &[Vec<AxisTap>],
    scratch: Option<&mut [f64]>,
    weights: Option<(&[f64], &[f64])>,
) {
    let output_width = output_row_byte_len / rgba8::RGBA8_CHANNELS;
    let scratch_row_len = output_width * CHANNELS;
    let support = source_rows(y_taps_by_output);
    let scratch_height = support.len();
    let mut owned_scratch;
    let scratch = match scratch {
        Some(scratch) => scratch,
        None => {
            owned_scratch = vec![0.0; scratch_height * scratch_row_len];
            &mut owned_scratch
        }
    };

    prepare_horizontal_scratch_rows::<CHANNELS, RAW_SUMS>(
        source,
        source_row_byte_len,
        x_taps_by_output,
        scratch,
        support.clone(),
        None,
    );
    write_x_then_y_rows_from_scratch::<CHANNELS, RAW_SUMS>(
        output,
        output_row_byte_len,
        y_taps_by_output,
        scratch,
        support.start,
        weights,
    );
}

/// Retain horizontally filtered rows shared with the previous output block and
/// calculate only the newly exposed source rows. Returns the number calculated.
fn prepare_horizontal_scratch_rows<const CHANNELS: usize, const RAW_SUMS: bool>(
    source: &[u8],
    source_row_byte_len: usize,
    x_taps_by_output: &[Vec<AxisTap>],
    scratch: &mut [f64],
    support: std::ops::Range<usize>,
    prepared_support: Option<std::ops::Range<usize>>,
) -> usize {
    let scratch_row_len = x_taps_by_output.len() * CHANNELS;
    let overlap = prepared_support
        .as_ref()
        .map(|prepared| prepared.start.max(support.start)..prepared.end.min(support.end))
        .filter(|range| !range.is_empty());

    if let (Some(prepared), Some(overlap)) = (prepared_support.as_ref(), overlap.as_ref()) {
        let source_start = (overlap.start - prepared.start) * scratch_row_len;
        let source_end = source_start + overlap.len() * scratch_row_len;
        let destination_start = (overlap.start - support.start) * scratch_row_len;
        scratch.copy_within(source_start..source_end, destination_start);
    }

    let mut calculated = 0;
    for source_y in support.clone() {
        if overlap
            .as_ref()
            .is_some_and(|range| range.contains(&source_y))
        {
            continue;
        }
        let source_start = source_y * source_row_byte_len;
        let source_row = &source[source_start..source_start + source_row_byte_len];
        let scratch_start = (source_y - support.start) * scratch_row_len;
        let scratch_row = &mut scratch[scratch_start..scratch_start + scratch_row_len];
        for (scratch_pixel, x_taps) in scratch_row.chunks_exact_mut(CHANNELS).zip(x_taps_by_output)
        {
            write_horizontal_scratch_pixel::<CHANNELS, RAW_SUMS>(scratch_pixel, source_row, x_taps);
        }
        calculated += 1;
    }
    calculated
}

fn write_x_then_y_rows_from_scratch<const CHANNELS: usize, const RAW_SUMS: bool>(
    output: &mut [u8],
    output_row_byte_len: usize,
    y_taps_by_output: &[Vec<AxisTap>],
    scratch: &[f64],
    first_source_y: usize,
    weights: Option<(&[f64], &[f64])>,
) {
    let output_width = output_row_byte_len / rgba8::RGBA8_CHANNELS;
    let scratch_row_len = output_width * CHANNELS;

    for (output_y, (output_row, y_taps)) in output
        .chunks_exact_mut(output_row_byte_len)
        .zip(y_taps_by_output)
        .enumerate()
    {
        for output_x in 0..output_width {
            let output_start = output_x * rgba8::RGBA8_CHANNELS;
            let (x_weight, y_weight) = if RAW_SUMS {
                let (x_weights, y_weights) = weights.expect("raw blocks cache axis weights");
                (x_weights[output_x], y_weights[output_y])
            } else {
                (1.0, 0.0)
            };
            write_vertical_scratch_pixel_with_base::<CHANNELS, RAW_SUMS>(
                &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS],
                &scratch,
                scratch_row_len,
                output_x * CHANNELS,
                y_taps,
                first_source_y,
                x_weight,
                y_weight,
            );
        }
    }
}

/// The existing x-then-y band kernel filters this complete contiguous support interval.
pub(super) fn source_rows(y_taps_by_output: &[Vec<AxisTap>]) -> std::ops::Range<usize> {
    let first = y_taps_by_output
        .iter()
        .flat_map(|taps| taps.iter().map(|tap| tap.index))
        .min()
        .unwrap_or(0);
    let last = y_taps_by_output
        .iter()
        .flat_map(|taps| taps.iter().map(|tap| tap.index))
        .max()
        .unwrap_or(first);
    first..last + 1
}

fn write_horizontal_scratch_pixel<const CHANNELS: usize, const RAW_SUMS: bool>(
    output_pixel: &mut [f64],
    source_row: &[u8],
    x_taps: &[AxisTap],
) {
    if RAW_SUMS {
        if let Some(x_taps) = as_fixed_taps::<6>(x_taps) {
            write_horizontal_raw_six_tap_scratch_pixel::<CHANNELS>(
                output_pixel,
                source_row,
                x_taps,
            );
            return;
        }
    }

    let mut accumulated = [0.0; CHANNELS];
    let mut total_weight = 0.0;

    for x_tap in x_taps {
        let source_start = x_tap.index * rgba8::RGBA8_CHANNELS;
        let source_pixel = &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];

        if !RAW_SUMS {
            total_weight += x_tap.weight;
        }
        for channel in 0..CHANNELS {
            accumulated[channel] += f64::from(source_pixel[channel]) * x_tap.weight;
        }
    }

    for channel in 0..CHANNELS {
        output_pixel[channel] = if RAW_SUMS {
            accumulated[channel]
        } else {
            accumulated[channel] / total_weight
        };
    }
}

#[inline(always)]
fn accumulate_horizontal_raw_tap<const CHANNELS: usize>(
    accumulated: &mut [f64; CHANNELS],
    source_row: &[u8],
    x_tap: &AxisTap,
) {
    let source_start = x_tap.index * rgba8::RGBA8_CHANNELS;
    let source_pixel = &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];
    for channel in 0..CHANNELS {
        accumulated[channel] += f64::from(source_pixel[channel]) * x_tap.weight;
    }
}

fn write_horizontal_raw_six_tap_scratch_pixel<const CHANNELS: usize>(
    output_pixel: &mut [f64],
    source_row: &[u8],
    x_taps: &[AxisTap; 6],
) {
    let mut accumulated = [0.0; CHANNELS];
    accumulate_horizontal_raw_tap(&mut accumulated, source_row, &x_taps[0]);
    accumulate_horizontal_raw_tap(&mut accumulated, source_row, &x_taps[1]);
    accumulate_horizontal_raw_tap(&mut accumulated, source_row, &x_taps[2]);
    accumulate_horizontal_raw_tap(&mut accumulated, source_row, &x_taps[3]);
    accumulate_horizontal_raw_tap(&mut accumulated, source_row, &x_taps[4]);
    accumulate_horizontal_raw_tap(&mut accumulated, source_row, &x_taps[5]);

    output_pixel[..CHANNELS].copy_from_slice(&accumulated);
}

fn write_vertical_scratch_pixel<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    scratch: &[f64],
    scratch_row_len: usize,
    output_start: usize,
    y_taps: &[AxisTap],
) {
    write_vertical_scratch_pixel_with_base::<CHANNELS, false>(
        output_pixel,
        scratch,
        scratch_row_len,
        output_start,
        y_taps,
        0,
        1.0,
        0.0,
    );
}

fn write_vertical_scratch_pixel_with_base<const CHANNELS: usize, const CACHED_WEIGHT: bool>(
    output_pixel: &mut [u8],
    scratch: &[f64],
    scratch_row_len: usize,
    output_start: usize,
    y_taps: &[AxisTap],
    first_source_y: usize,
    x_weight: f64,
    y_weight: f64,
) {
    let mut accumulated = [0.0; CHANNELS];
    let mut total_weight = y_weight;

    for y_tap in y_taps {
        let scratch_start = (y_tap.index - first_source_y) * scratch_row_len + output_start;
        let scratch_pixel = &scratch[scratch_start..scratch_start + CHANNELS];

        if !CACHED_WEIGHT {
            total_weight += y_tap.weight;
        }
        for channel in 0..CHANNELS {
            accumulated[channel] += scratch_pixel[channel] * y_tap.weight;
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, x_weight * total_weight);
}

fn resize_horizontal_only_into<const CHANNELS: usize>(
    source: &[u8],
    output: &mut [u8],
    source_row_byte_len: usize,
    output_row_byte_len: usize,
    x_taps_by_output: &[Vec<AxisTap>],
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    for (y, (source_row, output_row)) in source
        .chunks_exact(source_row_byte_len)
        .zip(output.chunks_exact_mut(output_row_byte_len))
        .enumerate()
    {
        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(x_taps_by_output)
        {
            write_horizontal_convolution_pixel::<CHANNELS>(output_pixel, source_row, x_taps);
        }
        progress(y as u32 + 1)?;
    }
    Ok(())
}

fn resize_vertical_only_into<const CHANNELS: usize>(
    source: &[u8],
    output: &mut [u8],
    source_row_byte_len: usize,
    output_row_byte_len: usize,
    y_taps_by_output: &[Vec<AxisTap>],
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    for (y, (output_row, y_taps)) in output
        .chunks_exact_mut(output_row_byte_len)
        .zip(y_taps_by_output)
        .enumerate()
    {
        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            write_vertical_convolution_pixel::<CHANNELS>(
                output_pixel,
                source,
                source_row_byte_len,
                output_x,
                y_taps,
            );
        }
        progress(y as u32 + 1)?;
    }
    Ok(())
}

fn write_horizontal_convolution_pixel<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    source_row: &[u8],
    x_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; CHANNELS];
    let mut total_weight = 0.0;

    for x_tap in x_taps {
        let source_start = x_tap.index * rgba8::RGBA8_CHANNELS;
        let source_pixel = &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];

        total_weight += x_tap.weight;
        for channel in 0..CHANNELS {
            accumulated[channel] += f64::from(source_pixel[channel]) * x_tap.weight;
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_vertical_convolution_pixel<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    output_x: usize,
    y_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; CHANNELS];
    let mut total_weight = 0.0;
    let source_x_start = output_x * rgba8::RGBA8_CHANNELS;

    for y_tap in y_taps {
        let source_start = y_tap.index * source_row_byte_len + source_x_start;
        let source_pixel = &source[source_start..source_start + rgba8::RGBA8_CHANNELS];

        total_weight += y_tap.weight;
        for channel in 0..CHANNELS {
            accumulated[channel] += f64::from(source_pixel[channel]) * y_tap.weight;
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_convolution_pixel<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    x_taps: &[AxisTap],
    y_taps: &[AxisTap],
) {
    if let (Some(x_taps), Some(y_taps)) = (as_fixed_taps::<4>(x_taps), as_fixed_taps::<4>(y_taps)) {
        write_fixed_convolution_pixel::<CHANNELS, 4>(
            output_pixel,
            source,
            source_row_byte_len,
            x_taps,
            y_taps,
        );
        return;
    }

    if let (Some(x_taps), Some(y_taps)) = (as_fixed_taps::<6>(x_taps), as_fixed_taps::<6>(y_taps)) {
        write_fixed_convolution_pixel::<CHANNELS, 6>(
            output_pixel,
            source,
            source_row_byte_len,
            x_taps,
            y_taps,
        );
        return;
    }

    write_variable_convolution_pixel::<CHANNELS>(
        output_pixel,
        source,
        source_row_byte_len,
        x_taps,
        y_taps,
    );
}

fn as_fixed_taps<const TAP_COUNT: usize>(taps: &[AxisTap]) -> Option<&[AxisTap; TAP_COUNT]> {
    taps.try_into().ok()
}

fn write_fixed_convolution_pixel<const CHANNELS: usize, const TAP_COUNT: usize>(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    x_taps: &[AxisTap; TAP_COUNT],
    y_taps: &[AxisTap; TAP_COUNT],
) {
    let mut accumulated = [0.0; CHANNELS];
    let mut total_weight = 0.0;

    for y_tap in y_taps {
        let source_row_start = y_tap.index * source_row_byte_len;

        for x_tap in x_taps {
            let weight = x_tap.weight * y_tap.weight;
            let source_start = source_row_start + x_tap.index * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source[source_start..source_start + rgba8::RGBA8_CHANNELS];

            total_weight += weight;
            for channel in 0..CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_variable_convolution_pixel<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    x_taps: &[AxisTap],
    y_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; CHANNELS];
    let mut total_weight = 0.0;

    for y_tap in y_taps {
        let source_row_start = y_tap.index * source_row_byte_len;

        for x_tap in x_taps {
            let weight = x_tap.weight * y_tap.weight;
            let source_start = source_row_start + x_tap.index * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source[source_start..source_start + rgba8::RGBA8_CHANNELS];

            total_weight += weight;
            for channel in 0..CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_accumulated_pixel<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    accumulated: [f64; CHANNELS],
    total_weight: f64,
) {
    for channel in 0..CHANNELS {
        output_pixel[channel] = round_byte(accumulated[channel] / total_weight);
    }
    if CHANNELS == 3 {
        output_pixel[3] = u8::MAX;
    }
}

#[inline]
fn round_byte(value: f64) -> u8 {
    let truncated = value as u8;
    truncated.saturating_add(u8::from(value - f64::from(truncated) >= 0.5))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::image::ImageDimensions;
    use crate::prod::resize::scalar::convolution::{ReconstructionKernel, ResizeAnchor};

    fn assert_six_tap_raw_sum_matches_loop<const CHANNELS: usize>() {
        let source = (0..8 * rgba8::RGBA8_CHANNELS)
            .map(|i| ((i * 47 + 19) % 256) as u8)
            .collect::<Vec<_>>();
        let taps = [
            AxisTap {
                index: 0,
                weight: -0.03125,
            },
            AxisTap {
                index: 2,
                weight: 0.15625,
            },
            AxisTap {
                index: 3,
                weight: 0.875,
            },
            AxisTap {
                index: 4,
                weight: 0.078125,
            },
            AxisTap {
                index: 6,
                weight: -0.09375,
            },
            AxisTap {
                index: 7,
                weight: 0.015625,
            },
        ];
        let mut expected = [0.0; CHANNELS];
        for tap in &taps {
            let start = tap.index * rgba8::RGBA8_CHANNELS;
            for channel in 0..CHANNELS {
                expected[channel] += f64::from(source[start + channel]) * tap.weight;
            }
        }
        let mut actual = [f64::NAN; CHANNELS];

        write_horizontal_raw_six_tap_scratch_pixel::<CHANNELS>(&mut actual, &source, &taps);

        assert_eq!(actual.map(f64::to_bits), expected.map(f64::to_bits));
    }

    #[test]
    fn six_tap_raw_sum_preserves_accumulation_order() {
        assert_six_tap_raw_sum_matches_loop::<3>();
        assert_six_tap_raw_sum_matches_loop::<4>();
    }

    struct RadiusThree;
    impl ReconstructionKernel for RadiusThree {
        fn radius(&self) -> f64 {
            3.0
        }
        fn weight(&self, distance: f64) -> f64 {
            if distance == 0.0 {
                return 1.0;
            }
            if distance.abs() >= 3.0 {
                return 0.0;
            }
            let x = distance * std::f64::consts::PI;
            (x.sin() / x) * ((x / 3.0).sin() / (x / 3.0))
        }
        fn allows_fixed_separable_shrink(&self) -> bool {
            true
        }
    }

    #[test]
    fn cached_raw_block_totals_match_per_pixel_totals_exactly() {
        let source_dimensions = ImageDimensions::new(128, 200).unwrap();
        for anchor in [
            ResizeAnchor::TopLeft,
            ResizeAnchor::Center,
            ResizeAnchor::BottomRight,
        ] {
            for (width, height) in [(32, 50), (43, 71), (64, 100), (77, 120), (96, 150)] {
                let output_dimensions = ImageDimensions::new(width, height).unwrap();
                let plan = ConvolutionResizePlan::new(
                    source_dimensions,
                    output_dimensions,
                    anchor,
                    &RadiusThree,
                    SupportPolicy::Fixed,
                );
                let mut source = (0..128 * 200 * 4)
                    .map(|i| ((i * 47 + i / 13) % 256) as u8)
                    .collect::<Vec<_>>();
                let row_len = width as usize * 4;
                let mut expected = vec![0; row_len * height as usize];
                let mut actual = expected.clone();
                let mut reference_scratch = vec![0.0; row_len * 200];
                let mut scratch = vec![f64::NAN; plan.scratch_elements().unwrap()];
                for opaque in [false, true] {
                    if opaque {
                        for pixel in source.chunks_exact_mut(4) {
                            pixel[3] = 255;
                        }
                    }
                    // Prior raw-block arithmetic, with totals summed independently per pixel.
                    for (source_row, scratch_row) in source
                        .chunks_exact(128 * 4)
                        .zip(reference_scratch.chunks_exact_mut(row_len))
                    {
                        for (pixel, taps) in scratch_row.chunks_exact_mut(4).zip(&plan.x_taps) {
                            write_horizontal_scratch_pixel::<4, true>(pixel, source_row, taps);
                        }
                    }
                    for (output_row, y_taps) in expected.chunks_exact_mut(row_len).zip(&plan.y_taps)
                    {
                        for (x, (pixel, x_taps)) in
                            output_row.chunks_exact_mut(4).zip(&plan.x_taps).enumerate()
                        {
                            write_vertical_scratch_pixel_with_base::<4, false>(
                                pixel,
                                &reference_scratch,
                                row_len,
                                x * 4,
                                y_taps,
                                0,
                                x_taps.iter().map(|tap| tap.weight).sum(),
                                0.0,
                            );
                        }
                    }
                    resize_with_progress(
                        ImageView::packed(&source, source_dimensions).unwrap(),
                        ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
                        &plan,
                        Some(&mut scratch),
                        &mut |_| Ok(()),
                    )
                    .unwrap();
                    assert_eq!(
                        actual, expected,
                        "{width}x{height}, {anchor:?}, opaque={opaque}"
                    );
                }
            }
        }
    }

    #[test]
    fn raw_fixed_blocks_preserve_patterned_fixture_across_blocks() {
        for anchor in [
            ResizeAnchor::TopLeft,
            ResizeAnchor::Center,
            ResizeAnchor::BottomRight,
        ] {
            for (width, height) in [(64, 100), (77, 120), (96, 150), (64, 183)] {
                let source_dimensions = ImageDimensions::new(128, 200).unwrap();
                let output_dimensions = ImageDimensions::new(width, height).unwrap();
                let plan = ConvolutionResizePlan::new(
                    source_dimensions,
                    output_dimensions,
                    anchor,
                    &RadiusThree,
                    SupportPolicy::Fixed,
                );
                let mut source = (0..128 * 200 * 4)
                    .map(|i| ((i * 47 + i / 13) % 256) as u8)
                    .collect::<Vec<_>>();
                let mut expected = vec![0; width as usize * height as usize * 4];
                let mut actual = expected.clone();
                let mut scratch = vec![f64::NAN; plan.scratch_elements().unwrap()];
                for taps in plan.y_taps.chunks(block_height(&plan)) {
                    assert!(source_rows(taps).len() <= block_source_rows(&plan));
                    assert!(source_rows(taps).len() * width as usize * 4 <= scratch.len());
                }
                for opaque in [false, true] {
                    if opaque {
                        for pixel in source.chunks_exact_mut(4) {
                            pixel[3] = 255;
                        }
                    }
                    resize_x_then_y_into::<4>(
                        &source,
                        &mut expected,
                        128 * 4,
                        width as usize * 4,
                        &plan.x_taps,
                        &plan.y_taps,
                        None,
                        &mut |_| Ok(()),
                    )
                    .unwrap();
                    resize_with_progress(
                        ImageView::packed(&source, source_dimensions).unwrap(),
                        ImageViewMut::packed(&mut actual, output_dimensions).unwrap(),
                        &plan,
                        Some(&mut scratch),
                        &mut |_| Ok(()),
                    )
                    .unwrap();
                    assert_eq!(actual, expected, "{width}x{height}, opaque={opaque}");
                }
            }
        }
    }

    #[test]
    fn adjacent_blocks_reuse_their_exact_horizontal_support_overlap() {
        let source_dimensions = ImageDimensions::new(128, 200).unwrap();
        let output_dimensions = ImageDimensions::new(64, 100).unwrap();
        let plan = ConvolutionResizePlan::new(
            source_dimensions,
            output_dimensions,
            ResizeAnchor::Center,
            &RadiusThree,
            SupportPolicy::Fixed,
        );
        let source = (0..128 * 200 * 4)
            .map(|i| ((i * 47 + i / 13) % 256) as u8)
            .collect::<Vec<_>>();
        let mut scratch = vec![f64::NAN; plan.scratch_elements().unwrap()];
        let mut blocks = plan.y_taps.chunks(block_height(&plan));
        let first = source_rows(blocks.next().unwrap());
        let second = source_rows(blocks.next().unwrap());
        let overlap = first.start.max(second.start)..first.end.min(second.end);

        let first_calculated = prepare_horizontal_scratch_rows::<4, true>(
            &source,
            128 * 4,
            &plan.x_taps,
            &mut scratch,
            first.clone(),
            None,
        );
        let second_calculated = prepare_horizontal_scratch_rows::<4, true>(
            &source,
            128 * 4,
            &plan.x_taps,
            &mut scratch,
            second.clone(),
            Some(first.clone()),
        );

        assert!(!overlap.is_empty());
        assert_eq!(first_calculated, first.len());
        assert_eq!(second_calculated, second.len() - overlap.len());

        let row_len = 64 * 4;
        let mut expected = vec![f64::NAN; second.len() * row_len];
        prepare_horizontal_scratch_rows::<4, true>(
            &source,
            128 * 4,
            &plan.x_taps,
            &mut expected,
            second.clone(),
            None,
        );
        assert_eq!(&scratch[..second.len() * row_len], expected);
    }

    #[test]
    fn byte_rounding_matches_clamp_and_round_at_every_boundary() {
        for integer in -1..=256 {
            for value in [f64::from(integer), f64::from(integer) + 0.5] {
                for neighbor in [value.next_down(), value, value.next_up()] {
                    assert_eq!(
                        round_byte(neighbor),
                        neighbor.clamp(0.0, 255.0).round() as u8,
                        "{neighbor:?}"
                    );
                }
            }
        }
        for value in [
            f64::NEG_INFINITY,
            f64::INFINITY,
            f64::NAN,
            -0.0,
            f64::MIN,
            f64::MAX,
        ] {
            assert_eq!(round_byte(value), value.clamp(0.0, 255.0).round() as u8);
        }
    }
}
