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
pub(super) const FIXED_BLOCK_HEIGHT: usize = 64;

/// Bound the source interval of 64 output rows, including radius-3 support.
pub(super) fn fixed_block_source_rows(plan: &ConvolutionResizePlan) -> usize {
    let source_height = u64::from(plan.source_dimensions().height());
    let output_height = u64::from(plan.output_dimensions().height());
    ((source_height * FIXED_BLOCK_HEIGHT as u64).div_ceil(output_height) + 6).min(source_height)
        as usize
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
    if should_use_fixed_blocks(plan) {
        return plan.output_dimensions().height()
            + plan
                .y_taps
                .chunks(FIXED_BLOCK_HEIGHT)
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
    if source
        .data()
        .chunks_exact(rgba8::RGBA8_CHANNELS)
        .all(|pixel| pixel[3] == u8::MAX)
    {
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
        resize_x_then_y_blocks_into::<CHANNELS>(
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
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
    y_start: u32,
    scratch: Option<&mut [f64]>,
) {
    // Row-band callers reuse the full source for every band. Keep this path at four channels so
    // threaded execution never rescans the complete image once per worker assignment.
    let source_width = source.dimensions().width_usize();
    let output_width = plan.output_dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();
    let y_start = y_start as usize;

    if plan.same_height() {
        let start = y_start * source_row_byte_len;
        let source_slice = &source_data[start..];
        resize_horizontal_only_into::<{ rgba8::RGBA8_CHANNELS }>(
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
        resize_vertical_only_into::<{ rgba8::RGBA8_CHANNELS }>(
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
        resize_x_then_y_rows_into::<{ rgba8::RGBA8_CHANNELS }, false>(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.x_taps,
            y_taps,
            scratch,
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
            write_convolution_pixel::<{ rgba8::RGBA8_CHANNELS }>(
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

fn resize_x_then_y_blocks_into<const CHANNELS: usize>(
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
    for (output_block, y_taps) in output
        .chunks_mut(output_row_byte_len * FIXED_BLOCK_HEIGHT)
        .zip(plan.y_taps.chunks(FIXED_BLOCK_HEIGHT))
    {
        let support_rows = source_rows(y_taps).len();
        debug_assert!(support_rows <= fixed_block_source_rows(plan));
        // Fixed Lanczos3 already accepts bounded regrouping. Store raw horizontal
        // sums and normalize once after the vertical pass within this block.
        resize_x_then_y_rows_into::<CHANNELS, true>(
            source,
            output_block,
            source_row_byte_len,
            output_row_byte_len,
            &plan.x_taps,
            y_taps,
            Some(scratch),
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
) {
    let output_width = output_row_byte_len / rgba8::RGBA8_CHANNELS;
    let scratch_row_len = output_width * CHANNELS;
    let support = source_rows(y_taps_by_output);
    let first_source_y = support.start;
    let scratch_height = support.len();
    let mut owned_scratch;
    let scratch = match scratch {
        Some(scratch) => scratch,
        None => {
            owned_scratch = vec![0.0; scratch_height * scratch_row_len];
            &mut owned_scratch
        }
    };

    for (source_row, scratch_row) in source
        .chunks_exact(source_row_byte_len)
        .skip(first_source_y)
        .take(scratch_height)
        .zip(scratch.chunks_exact_mut(scratch_row_len))
    {
        for (scratch_pixel, x_taps) in scratch_row.chunks_exact_mut(CHANNELS).zip(x_taps_by_output)
        {
            write_horizontal_scratch_pixel::<CHANNELS, RAW_SUMS>(scratch_pixel, source_row, x_taps);
        }
    }

    for (output_row, y_taps) in output
        .chunks_exact_mut(output_row_byte_len)
        .zip(y_taps_by_output)
    {
        for output_x in 0..output_width {
            let output_start = output_x * rgba8::RGBA8_CHANNELS;
            let x_weight = if RAW_SUMS {
                x_taps_by_output[output_x]
                    .iter()
                    .map(|tap| tap.weight)
                    .sum()
            } else {
                1.0
            };
            write_vertical_scratch_pixel_with_base::<CHANNELS>(
                &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS],
                &scratch,
                scratch_row_len,
                output_x * CHANNELS,
                y_taps,
                first_source_y,
                x_weight,
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

fn write_vertical_scratch_pixel<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    scratch: &[f64],
    scratch_row_len: usize,
    output_start: usize,
    y_taps: &[AxisTap],
) {
    write_vertical_scratch_pixel_with_base::<CHANNELS>(
        output_pixel,
        scratch,
        scratch_row_len,
        output_start,
        y_taps,
        0,
        1.0,
    );
}

fn write_vertical_scratch_pixel_with_base<const CHANNELS: usize>(
    output_pixel: &mut [u8],
    scratch: &[f64],
    scratch_row_len: usize,
    output_start: usize,
    y_taps: &[AxisTap],
    first_source_y: usize,
    x_weight: f64,
) {
    let mut accumulated = [0.0; CHANNELS];
    let mut total_weight = 0.0;

    for y_tap in y_taps {
        let scratch_start = (y_tap.index - first_source_y) * scratch_row_len + output_start;
        let scratch_pixel = &scratch[scratch_start..scratch_start + CHANNELS];

        total_weight += y_tap.weight;
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
                for taps in plan.y_taps.chunks(FIXED_BLOCK_HEIGHT) {
                    assert!(source_rows(taps).len() <= fixed_block_source_rows(&plan));
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
