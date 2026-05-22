//! Fractional planned-coverage area kernel for packed RGBA8.
//!
//! Exact integer down/up scales bypass this module. This path applies the area
//! coverage plan with f32 accumulation for accepted bounded visual drift.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::AreaResizePlan;

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

// REJECT(perf): Flattening y spans into `{ first_source_index, overlaps }`
// preserved correctness but regressed fractional and upscale cases by roughly
// 2-9% in `ditherette-bench run area`; keep the copied `AxisOverlap` y layout
// until the whole row traversal changes.
// NOTE(perf): This planned coverage path handles fractional resizes. Exact
// integer down/up scales bypass it above and remain byte-exact.
fn accumulate_pixel(
    source_data: &[u8],
    source_row_byte_len: usize,
    x_spans: &super::plan::XAxisOverlapSpan,
    y_spans: &[super::plan::AxisOverlap],
    area: f64,
) -> [f32; rgba8::RGBA8_CHANNELS] {
    if y_spans.len() == 1 && x_spans.overlaps.len() == 1 {
        let source_start =
            y_spans[0].source_index * source_row_byte_len + x_spans.first_byte_offset;
        let source_pixel = &source_data[source_start..source_start + rgba8::RGBA8_CHANNELS];
        return [
            f32::from(source_pixel[0]),
            f32::from(source_pixel[1]),
            f32::from(source_pixel[2]),
            f32::from(source_pixel[3]),
        ];
    }

    let mut accumulated = [0.0f32; rgba8::RGBA8_CHANNELS];

    if y_spans.len() == 1 {
        accumulate_single_y(
            source_data,
            source_row_byte_len,
            x_spans,
            y_spans[0],
            area,
            &mut accumulated,
        );
        return accumulated;
    }

    if x_spans.overlaps.len() == 1 {
        accumulate_single_x(
            source_data,
            source_row_byte_len,
            x_spans,
            y_spans,
            area,
            &mut accumulated,
        );
        return accumulated;
    }

    accumulate_grid(
        source_data,
        source_row_byte_len,
        x_spans,
        y_spans,
        area,
        &mut accumulated,
    );
    accumulated
}

fn accumulate_single_y(
    source_data: &[u8],
    source_row_byte_len: usize,
    x_spans: &super::plan::XAxisOverlapSpan,
    y_span: super::plan::AxisOverlap,
    area: f64,
    accumulated: &mut [f32; rgba8::RGBA8_CHANNELS],
) {
    let source_row_start = y_span.source_index * source_row_byte_len;
    let source_start = source_row_start + x_spans.first_byte_offset;
    let source_end = source_row_start + x_spans.last_exclusive_byte_offset;
    let source_pixels = source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS);

    for (x_overlap, source_pixel) in x_spans.overlaps.iter().copied().zip(source_pixels) {
        let weight = (x_overlap * y_span.overlap / area) as f32;
        accumulate_weighted_pixel(accumulated, source_pixel, weight);
    }
}

fn accumulate_single_x(
    source_data: &[u8],
    source_row_byte_len: usize,
    x_spans: &super::plan::XAxisOverlapSpan,
    y_spans: &[super::plan::AxisOverlap],
    area: f64,
    accumulated: &mut [f32; rgba8::RGBA8_CHANNELS],
) {
    let x_overlap = x_spans.overlaps[0];
    for y_span in y_spans {
        let source_start = y_span.source_index * source_row_byte_len + x_spans.first_byte_offset;
        let source_pixel = &source_data[source_start..source_start + rgba8::RGBA8_CHANNELS];
        let weight = (x_overlap * y_span.overlap / area) as f32;
        accumulate_weighted_pixel(accumulated, source_pixel, weight);
    }
}

fn accumulate_grid(
    source_data: &[u8],
    source_row_byte_len: usize,
    x_spans: &super::plan::XAxisOverlapSpan,
    y_spans: &[super::plan::AxisOverlap],
    area: f64,
    accumulated: &mut [f32; rgba8::RGBA8_CHANNELS],
) {
    for y_span in y_spans {
        let source_row_start = y_span.source_index * source_row_byte_len;
        let source_start = source_row_start + x_spans.first_byte_offset;
        let source_end = source_row_start + x_spans.last_exclusive_byte_offset;
        let source_pixels =
            source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS);

        for (x_overlap, source_pixel) in x_spans.overlaps.iter().copied().zip(source_pixels) {
            // REJECT(perf): Hoisting `y_span.overlap / area` changed f64
            // rounding order and failed exact verification on 0.75x box art.
            let weight = (x_overlap * y_span.overlap / area) as f32;
            accumulate_weighted_pixel(accumulated, source_pixel, weight);
        }
    }
}

fn accumulate_weighted_pixel(
    accumulated: &mut [f32; rgba8::RGBA8_CHANNELS],
    source_pixel: &[u8],
    weight: f32,
) {
    for channel in 0..rgba8::RGBA8_CHANNELS {
        accumulated[channel] += f32::from(source_pixel[channel]) * weight;
    }
}
