//! Packed RGBA8 scalar kernel for production area resize.
//!
//! This kernel assumes the shared production resize boundary has already
//! normalized source and output rows to packed RGBA8. It mirrors the spec area
//! coverage formula while keeping RGBA8-specific row traversal in one place.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::AreaResizePlan;

// NOTE(perf): Fractional minification now uses compact precomputed x coverage
// plus planned y coverage. Exact integer downscales bypass this path because
// 0.1x/0.125x are primary Ditherette cases and need local block kernels.
// DEFER(perf): Row-band tiling needs an explicit production/tiled benchmark
// subject and a native-only execution boundary. Keep scalar area focused on the
// Wasm-compatible path until tiled subjects can compare four-band vs near-source
// choices without hiding scheduling overhead inside this scalar subject.
// DEFER(perf): Separable horizontal-then-vertical area is no longer the next
// useful path for the focused Ditherette scale group: exact down/up scales now
// bypass coverage, and near-source fractional cases have tiny spans. Revisit
// only with a fixture/scale group that makes non-integer large minification hot
// enough to justify a scratch-buffer path and f64 order audit.

pub(super) fn resize_exact_integer_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) -> bool {
    let Some((x_step, y_step)) = exact_integer_downscale_steps(source, output) else {
        return false;
    };

    if x_step == 2 && y_step == 2 {
        resize_exact_2x_downscale_into(source, output);
        return true;
    }
    if x_step == 4 && y_step == 4 {
        resize_exact_4x_downscale_into(source, output);
        return true;
    }
    if x_step == 8 && y_step == 8 {
        resize_exact_8x_downscale_into(source, output);
        return true;
    }
    if x_step == 10 && y_step == 10 {
        resize_exact_10x_downscale_into(source, output);
        return true;
    }

    resize_exact_block_downscale_into(source, output, x_step, y_step);
    true
}

pub(super) fn resize_exact_integer_upscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) -> bool {
    let Some((x_step, y_step)) = exact_integer_upscale_steps(source, output) else {
        return false;
    };

    resize_exact_block_upscale_into(source, output, x_step, y_step);
    true
}

pub(super) fn resize_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    let source_row_byte_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = plan.output_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let y_spans = &plan.y_spans[output_y];

        for (output_pixel, x_spans) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_spans)
        {
            let accumulated = accumulate_pixel(
                source_data,
                source_row_byte_len,
                x_spans,
                y_spans,
                plan.area,
            );

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

fn exact_integer_downscale_steps(
    source: ImageView<'_, Rgba8>,
    output: &ImageViewMut<'_, Rgba8>,
) -> Option<(usize, usize)> {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let source_width = source_dimensions.width();
    let source_height = source_dimensions.height();
    let output_width = output_dimensions.width();
    let output_height = output_dimensions.height();

    if source_width < output_width || source_height < output_height {
        return None;
    }
    if source_width == output_width && source_height == output_height {
        return None;
    }
    if !source_width.is_multiple_of(output_width) || !source_height.is_multiple_of(output_height) {
        return None;
    }

    Some((
        (source_width / output_width) as usize,
        (source_height / output_height) as usize,
    ))
}

fn exact_integer_upscale_steps(
    source: ImageView<'_, Rgba8>,
    output: &ImageViewMut<'_, Rgba8>,
) -> Option<(usize, usize)> {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let source_width = source_dimensions.width();
    let source_height = source_dimensions.height();
    let output_width = output_dimensions.width();
    let output_height = output_dimensions.height();

    if output_width < source_width || output_height < source_height {
        return None;
    }
    if output_width == source_width && output_height == source_height {
        return None;
    }
    if !output_width.is_multiple_of(source_width) || !output_height.is_multiple_of(source_height) {
        return None;
    }

    Some((
        (output_width / source_width) as usize,
        (output_height / source_height) as usize,
    ))
}

fn resize_exact_block_upscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    x_step: usize,
    y_step: usize,
) {
    let source_width = source.dimensions().width_usize();
    let source_height = source.dimensions().height_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for source_y in 0..source_height {
        let source_row_start = source_y * source_row_byte_len;
        let source_row = &source_data[source_row_start..source_row_start + source_row_byte_len];
        let first_output_y = source_y * y_step;
        let first_output_row_start = first_output_y * output_row_byte_len;
        let (first_output_row, repeated_output_rows) = output.data_mut()
            [first_output_row_start..first_output_row_start + output_row_byte_len * y_step]
            .split_at_mut(output_row_byte_len);

        for (source_pixel, output_pixels) in source_row
            .chunks_exact(rgba8::RGBA8_CHANNELS)
            .zip(first_output_row.chunks_exact_mut(rgba8::RGBA8_CHANNELS * x_step))
        {
            for output_pixel in output_pixels.chunks_exact_mut(rgba8::RGBA8_CHANNELS) {
                output_pixel.copy_from_slice(source_pixel);
            }
        }

        for repeated_output_row in repeated_output_rows.chunks_exact_mut(output_row_byte_len) {
            repeated_output_row.copy_from_slice(first_output_row);
        }
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

fn resize_exact_4x_downscale_into(
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
        let first_source_row_start = output_y * 4 * source_row_byte_len;

        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            let source_x_start = output_x * 4 * rgba8::RGBA8_CHANNELS;
            let mut red_sum = 0u16;
            let mut green_sum = 0u16;
            let mut blue_sum = 0u16;
            let mut alpha_sum = 0u16;

            for source_row_offset in 0..4 {
                let row_start = first_source_row_start + source_row_offset * source_row_byte_len;
                let source_start = row_start + source_x_start;
                let source_end = source_start + 4 * rgba8::RGBA8_CHANNELS;
                for source_pixel in
                    source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
                {
                    red_sum += u16::from(source_pixel[0]);
                    green_sum += u16::from(source_pixel[1]);
                    blue_sum += u16::from(source_pixel[2]);
                    alpha_sum += u16::from(source_pixel[3]);
                }
            }

            output_pixel[0] = ((red_sum + 8) / 16) as u8;
            output_pixel[1] = ((green_sum + 8) / 16) as u8;
            output_pixel[2] = ((blue_sum + 8) / 16) as u8;
            output_pixel[3] = ((alpha_sum + 8) / 16) as u8;
        }
    }
}

fn resize_exact_8x_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) {
    const STEP: usize = 8;
    const DIVISOR: u32 = (STEP * STEP) as u32;
    const ROUNDING_HALF: u32 = DIVISOR / 2;

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
            let mut red_sum = 0u32;
            let mut green_sum = 0u32;
            let mut blue_sum = 0u32;
            let mut alpha_sum = 0u32;

            for source_row_offset in 0..STEP {
                let row_start = first_source_row_start + source_row_offset * source_row_byte_len;
                let source_start = row_start + source_x_start;
                let source_end = source_start + STEP * rgba8::RGBA8_CHANNELS;
                for source_pixel in
                    source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
                {
                    red_sum += u32::from(source_pixel[0]);
                    green_sum += u32::from(source_pixel[1]);
                    blue_sum += u32::from(source_pixel[2]);
                    alpha_sum += u32::from(source_pixel[3]);
                }
            }

            output_pixel[0] = ((red_sum + ROUNDING_HALF) / DIVISOR) as u8;
            output_pixel[1] = ((green_sum + ROUNDING_HALF) / DIVISOR) as u8;
            output_pixel[2] = ((blue_sum + ROUNDING_HALF) / DIVISOR) as u8;
            output_pixel[3] = ((alpha_sum + ROUNDING_HALF) / DIVISOR) as u8;
        }
    }
}

fn resize_exact_10x_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) {
    const STEP: usize = 10;
    const WEIGHT: f64 = 1.0 / (STEP * STEP) as f64;

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
            let mut red_sum = 0.0;
            let mut green_sum = 0.0;
            let mut blue_sum = 0.0;
            let mut alpha_sum = 0.0;

            for source_row_offset in 0..STEP {
                let row_start = first_source_row_start + source_row_offset * source_row_byte_len;
                let source_start = row_start + source_x_start;
                let source_end = source_start + STEP * rgba8::RGBA8_CHANNELS;
                for source_pixel in
                    source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
                {
                    red_sum += f64::from(source_pixel[0]) * WEIGHT;
                    green_sum += f64::from(source_pixel[1]) * WEIGHT;
                    blue_sum += f64::from(source_pixel[2]) * WEIGHT;
                    alpha_sum += f64::from(source_pixel[3]) * WEIGHT;
                }
            }

            output_pixel[0] = red_sum.clamp(0.0, 255.0).round() as u8;
            output_pixel[1] = green_sum.clamp(0.0, 255.0).round() as u8;
            output_pixel[2] = blue_sum.clamp(0.0, 255.0).round() as u8;
            output_pixel[3] = alpha_sum.clamp(0.0, 255.0).round() as u8;
        }
    }
}

fn resize_exact_block_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
    x_step: usize,
    y_step: usize,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let weight = 1.0 / (x_step * y_step) as f64;
    let source_data = source.data();

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let source_y_start = output_y * y_step;
        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            let source_x_start = output_x * x_step;
            let mut red_sum = 0.0;
            let mut green_sum = 0.0;
            let mut blue_sum = 0.0;
            let mut alpha_sum = 0.0;

            for source_y in source_y_start..source_y_start + y_step {
                let row_start = source_y * source_row_byte_len;
                let source_start = row_start + source_x_start * rgba8::RGBA8_CHANNELS;
                let source_end = source_start + x_step * rgba8::RGBA8_CHANNELS;
                for source_pixel in
                    source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
                {
                    red_sum += f64::from(source_pixel[0]) * weight;
                    green_sum += f64::from(source_pixel[1]) * weight;
                    blue_sum += f64::from(source_pixel[2]) * weight;
                    alpha_sum += f64::from(source_pixel[3]) * weight;
                }
            }

            output_pixel[0] = red_sum.clamp(0.0, 255.0).round() as u8;
            output_pixel[1] = green_sum.clamp(0.0, 255.0).round() as u8;
            output_pixel[2] = blue_sum.clamp(0.0, 255.0).round() as u8;
            output_pixel[3] = alpha_sum.clamp(0.0, 255.0).round() as u8;
        }
    }
}

// REJECT(perf): Flattening y spans into `{ first_source_index, overlaps }`
// preserved correctness but regressed fractional and upscale cases by roughly
// 2-9% in `ditherette-bench run area --baseline accepted`; keep the copied
// `AxisOverlap` y layout until the whole row traversal changes.
fn accumulate_pixel(
    source_data: &[u8],
    source_row_byte_len: usize,
    x_spans: &super::plan::XAxisOverlapSpan,
    y_spans: &[super::plan::AxisOverlap],
    area: f64,
) -> [f64; rgba8::RGBA8_CHANNELS] {
    if y_spans.len() == 1 && x_spans.overlaps.len() == 1 {
        let source_start =
            y_spans[0].source_index * source_row_byte_len + x_spans.first_byte_offset;
        let source_pixel = &source_data[source_start..source_start + rgba8::RGBA8_CHANNELS];
        return [
            f64::from(source_pixel[0]),
            f64::from(source_pixel[1]),
            f64::from(source_pixel[2]),
            f64::from(source_pixel[3]),
        ];
    }

    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];

    if y_spans.len() == 1 {
        let y_span = y_spans[0];
        let source_row_start = y_span.source_index * source_row_byte_len;
        let source_start = source_row_start + x_spans.first_byte_offset;
        let source_end = source_row_start + x_spans.last_exclusive_byte_offset;
        let source_pixels =
            source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS);

        for (x_overlap, source_pixel) in x_spans.overlaps.iter().copied().zip(source_pixels) {
            let weight = x_overlap * y_span.overlap / area;
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }

        return accumulated;
    }

    if x_spans.overlaps.len() == 1 {
        let x_overlap = x_spans.overlaps[0];
        for y_span in y_spans {
            let source_start =
                y_span.source_index * source_row_byte_len + x_spans.first_byte_offset;
            let source_pixel = &source_data[source_start..source_start + rgba8::RGBA8_CHANNELS];
            let weight = x_overlap * y_span.overlap / area;
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }

        return accumulated;
    }

    for y_span in y_spans {
        let source_row_start = y_span.source_index * source_row_byte_len;
        let source_start = source_row_start + x_spans.first_byte_offset;
        let source_end = source_row_start + x_spans.last_exclusive_byte_offset;
        let source_pixels =
            source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS);

        for (x_overlap, source_pixel) in x_spans.overlaps.iter().copied().zip(source_pixels) {
            // REJECT(perf): Hoisting `y_span.overlap / area` changed f64
            // rounding order and failed exact verification on 0.75x box art.
            let weight = x_overlap * y_span.overlap / area;

            // REJECT(perf): f32 accumulators are not byte-identical to the f64
            // spec oracle; `cargo test --test prod_resize_area` fails at a
            // representative 3x7 resize. Keep f64 on the generic coverage path.
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    accumulated
}
