//! Exact integer downscale area kernels for packed RGBA8.
//!
//! These keep area downscales byte-identical to the spec oracle. Power-of-two
//! hot cases use integer sums; the 10x/default paths keep the spec's weighted
//! f64 accumulation order.
//!
//! NOTE(perf): Exact integer area downscale is intentionally not a separable
//! TODO. The block kernels already read each source pixel once for disjoint
//! output blocks; a two-pass scratch image would add memory traffic without
//! reducing coverage work. Fractional planned coverage is the separable target.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

pub(super) fn resize_exact_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    x_step: usize,
    y_step: usize,
) {
    resize_exact_downscale_rows_into(source, output, x_step, y_step, 0);
}

pub(super) fn resize_exact_downscale_rows_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    x_step: usize,
    y_step: usize,
    y_start: u32,
) {
    if y_start == 0 && x_step == 2 && y_step == 2 {
        resize_exact_2x_downscale_into(source, output);
    } else if y_start == 0 && x_step == 4 && y_step == 4 {
        downscale_fixed_u16::<4, 16, 8>(source, output);
    } else if y_start == 0 && x_step == 8 && y_step == 8 {
        downscale_fixed_u32::<8, 64, 32>(source, output);
    } else if x_step == 10 && y_step == 10 {
        resize_exact_weighted_block_downscale_rows_into(
            source,
            output,
            10,
            10,
            1.0 / 100.0,
            y_start,
        );
    } else {
        let weight = 1.0 / (x_step * y_step) as f64;
        resize_exact_weighted_block_downscale_rows_into(
            source, output, x_step, y_step, weight, y_start,
        );
    }
}

fn resize_exact_2x_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let first_source_row_start = output_y * 2 * source_row_byte_len;
        let second_source_row_start = first_source_row_start + source_row_byte_len;

        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            let source_x_start = output_x * 2 * rgba8::RGBA8_CHANNELS;
            let top_left = first_source_row_start + source_x_start;
            let top_right = top_left + rgba8::RGBA8_CHANNELS;
            let bottom_left = second_source_row_start + source_x_start;
            let bottom_right = bottom_left + rgba8::RGBA8_CHANNELS;

            for channel in 0..rgba8::RGBA8_CHANNELS {
                let sum = u16::from(source_data[top_left + channel])
                    + u16::from(source_data[top_right + channel])
                    + u16::from(source_data[bottom_left + channel])
                    + u16::from(source_data[bottom_right + channel]);
                output_pixel[channel] = ((sum + 2) / 4) as u8;
            }
        }
    }
}

fn downscale_fixed_u16<const STEP: usize, const DIVISOR: u16, const ROUNDING_HALF: u16>(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let first_source_row_start = output_y * STEP * source_row_byte_len;

        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            let source_x_start = output_x * STEP * rgba8::RGBA8_CHANNELS;
            let sums = sum_block_u16::<STEP>(
                source_data,
                source_row_byte_len,
                first_source_row_start,
                source_x_start,
            );

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = ((sums[channel] + ROUNDING_HALF) / DIVISOR) as u8;
            }
        }
    }
}

fn downscale_fixed_u32<const STEP: usize, const DIVISOR: u32, const ROUNDING_HALF: u32>(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let first_source_row_start = output_y * STEP * source_row_byte_len;

        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            let source_x_start = output_x * STEP * rgba8::RGBA8_CHANNELS;
            let sums = sum_block_u32::<STEP>(
                source_data,
                source_row_byte_len,
                first_source_row_start,
                source_x_start,
            );

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = ((sums[channel] + ROUNDING_HALF) / DIVISOR) as u8;
            }
        }
    }
}

fn sum_block_u16<const STEP: usize>(
    source_data: &[u8],
    source_row_byte_len: usize,
    first_source_row_start: usize,
    source_x_start: usize,
) -> [u16; rgba8::RGBA8_CHANNELS] {
    let mut sums = [0u16; rgba8::RGBA8_CHANNELS];
    for source_row_offset in 0..STEP {
        let row_start = first_source_row_start + source_row_offset * source_row_byte_len;
        let source_start = row_start + source_x_start;
        let source_end = source_start + STEP * rgba8::RGBA8_CHANNELS;
        for source_pixel in
            source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
        {
            for channel in 0..rgba8::RGBA8_CHANNELS {
                sums[channel] += u16::from(source_pixel[channel]);
            }
        }
    }
    sums
}

fn sum_block_u32<const STEP: usize>(
    source_data: &[u8],
    source_row_byte_len: usize,
    first_source_row_start: usize,
    source_x_start: usize,
) -> [u32; rgba8::RGBA8_CHANNELS] {
    let mut sums = [0u32; rgba8::RGBA8_CHANNELS];
    for source_row_offset in 0..STEP {
        let row_start = first_source_row_start + source_row_offset * source_row_byte_len;
        let source_start = row_start + source_x_start;
        let source_end = source_start + STEP * rgba8::RGBA8_CHANNELS;
        for source_pixel in
            source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
        {
            for channel in 0..rgba8::RGBA8_CHANNELS {
                sums[channel] += u32::from(source_pixel[channel]);
            }
        }
    }
    sums
}

fn resize_exact_weighted_block_downscale_rows_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    x_step: usize,
    y_step: usize,
    weight: f64,
    y_start: u32,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (local_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let source_y_start = (y_start as usize + local_y) * y_step;
        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            let source_x_start = output_x * x_step;
            let sums = sum_weighted_block(
                source_data,
                source_row_byte_len,
                source_x_start,
                source_y_start,
                x_step,
                y_step,
                weight,
            );

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = sums[channel].clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

fn sum_weighted_block(
    source_data: &[u8],
    source_row_byte_len: usize,
    source_x_start: usize,
    source_y_start: usize,
    x_step: usize,
    y_step: usize,
    weight: f64,
) -> [f64; rgba8::RGBA8_CHANNELS] {
    let mut sums = [0.0; rgba8::RGBA8_CHANNELS];
    for source_y in source_y_start..source_y_start + y_step {
        let row_start = source_y * source_row_byte_len;
        let source_start = row_start + source_x_start * rgba8::RGBA8_CHANNELS;
        let source_end = source_start + x_step * rgba8::RGBA8_CHANNELS;
        for source_pixel in
            source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
        {
            for channel in 0..rgba8::RGBA8_CHANNELS {
                sums[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }
    sums
}
