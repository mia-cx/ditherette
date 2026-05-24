//! Packed RGBA8 convolution kernel.
//!
//! The kernel consumes preplanned x/y support taps and preserves the direct
//! y-major, x-minor contribution grouping used by the scalar convolution oracle.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::plan::{AxisTap, ConvolutionResizePlan};

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
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    if plan.same_height() {
        resize_horizontal_only_into(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.x_taps,
        );
        return;
    }

    if plan.same_width() {
        resize_vertical_only_into(
            source_data,
            output.data_mut(),
            source_row_byte_len,
            output_row_byte_len,
            &plan.y_taps,
        );
        return;
    }

    for (output_row, y_taps) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .zip(&plan.y_taps)
    {
        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_taps)
        {
            write_convolution_pixel(
                output_pixel,
                source_data,
                source_row_byte_len,
                x_taps,
                y_taps,
            );
        }
    }
}

fn resize_horizontal_only_into(
    source: &[u8],
    output: &mut [u8],
    source_row_byte_len: usize,
    output_row_byte_len: usize,
    x_taps_by_output: &[Vec<AxisTap>],
) {
    for (source_row, output_row) in source
        .chunks_exact(source_row_byte_len)
        .zip(output.chunks_exact_mut(output_row_byte_len))
    {
        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(x_taps_by_output)
        {
            write_horizontal_convolution_pixel(output_pixel, source_row, x_taps);
        }
    }
}

fn resize_vertical_only_into(
    source: &[u8],
    output: &mut [u8],
    source_row_byte_len: usize,
    output_row_byte_len: usize,
    y_taps_by_output: &[Vec<AxisTap>],
) {
    for (output_row, y_taps) in output
        .chunks_exact_mut(output_row_byte_len)
        .zip(y_taps_by_output)
    {
        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            write_vertical_convolution_pixel(
                output_pixel,
                source,
                source_row_byte_len,
                output_x,
                y_taps,
            );
        }
    }
}

fn write_horizontal_convolution_pixel(
    output_pixel: &mut [u8],
    source_row: &[u8],
    x_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];
    let mut total_weight = 0.0;

    for x_tap in x_taps {
        let source_start = x_tap.index * rgba8::RGBA8_CHANNELS;
        let source_pixel = &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];

        total_weight += x_tap.weight;
        for channel in 0..rgba8::RGBA8_CHANNELS {
            accumulated[channel] += f64::from(source_pixel[channel]) * x_tap.weight;
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_vertical_convolution_pixel(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    output_x: usize,
    y_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];
    let mut total_weight = 0.0;
    let source_x_start = output_x * rgba8::RGBA8_CHANNELS;

    for y_tap in y_taps {
        let source_start = y_tap.index * source_row_byte_len + source_x_start;
        let source_pixel = &source[source_start..source_start + rgba8::RGBA8_CHANNELS];

        total_weight += y_tap.weight;
        for channel in 0..rgba8::RGBA8_CHANNELS {
            accumulated[channel] += f64::from(source_pixel[channel]) * y_tap.weight;
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_convolution_pixel(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    x_taps: &[AxisTap],
    y_taps: &[AxisTap],
) {
    if let (Some(x_taps), Some(y_taps)) = (as_fixed_taps::<4>(x_taps), as_fixed_taps::<4>(y_taps)) {
        write_fixed_convolution_pixel(output_pixel, source, source_row_byte_len, x_taps, y_taps);
        return;
    }

    if let (Some(x_taps), Some(y_taps)) = (as_fixed_taps::<6>(x_taps), as_fixed_taps::<6>(y_taps)) {
        write_fixed_convolution_pixel(output_pixel, source, source_row_byte_len, x_taps, y_taps);
        return;
    }

    write_variable_convolution_pixel(output_pixel, source, source_row_byte_len, x_taps, y_taps);
}

fn as_fixed_taps<const TAP_COUNT: usize>(taps: &[AxisTap]) -> Option<&[AxisTap; TAP_COUNT]> {
    taps.try_into().ok()
}

fn write_fixed_convolution_pixel<const TAP_COUNT: usize>(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    x_taps: &[AxisTap; TAP_COUNT],
    y_taps: &[AxisTap; TAP_COUNT],
) {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];
    let mut total_weight = 0.0;

    for y_tap in y_taps {
        let source_row_start = y_tap.index * source_row_byte_len;

        for x_tap in x_taps {
            let weight = x_tap.weight * y_tap.weight;
            let source_start = source_row_start + x_tap.index * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source[source_start..source_start + rgba8::RGBA8_CHANNELS];

            total_weight += weight;
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_variable_convolution_pixel(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    x_taps: &[AxisTap],
    y_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];
    let mut total_weight = 0.0;

    for y_tap in y_taps {
        let source_row_start = y_tap.index * source_row_byte_len;

        for x_tap in x_taps {
            let weight = x_tap.weight * y_tap.weight;
            let source_start = source_row_start + x_tap.index * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source[source_start..source_start + rgba8::RGBA8_CHANNELS];

            total_weight += weight;
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    write_accumulated_pixel(output_pixel, accumulated, total_weight);
}

fn write_accumulated_pixel(output_pixel: &mut [u8], accumulated: [f64; 4], total_weight: f64) {
    for channel in 0..rgba8::RGBA8_CHANNELS {
        output_pixel[channel] = (accumulated[channel] / total_weight)
            .clamp(0.0, 255.0)
            .round() as u8;
    }
}
