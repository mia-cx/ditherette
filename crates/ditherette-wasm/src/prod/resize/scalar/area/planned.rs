//! Fractional planned-coverage area kernel for packed RGBA8.
//!
//! Exact integer down/up scales bypass this module. This path applies the area
//! coverage plan with f32 accumulation for accepted bounded visual drift.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::AreaResizePlan;

// Current-code area separability map:
// fractional one-shot plan shape -> shape gate -> scratch reuse -> common-span
// kernels -> weight normalization.
// ACCEPT(perf): Fractional area now filters y coverage into a scratch row, then
// applies x coverage. `ditherette-bench run area` preserved bounded correctness
// and improved represented fractional cases by ~50-72% while exact integer paths
// stayed on their existing bypasses.
// REJECT(perf): Gating tiny fractional outputs (<=10k output pixels) back to a
// direct 2D area kernel preserved bounded correctness but did not improve tiny
// cases and regressed representative large fractional cases by roughly 2-6% in
// `ditherette-bench run area`; keep the separable area path for aspect-preserving
// one-shot resizes.
// CLOSE(perf): Separable area already reuses one vertical scratch row across
// output rows inside a cold resize call. Thread-local or caller-owned scratch
// reuse targets repeated same-shape calls, which is not the current product
// workload.
// ACCEPT(perf): `area-anisotropic` now covers width-only and height-only area
// profiles with bounded correctness before tuning one-axis separable order.
// ACCEPT(perf): One-axis area now skips the unused separable pass; bounded
// `area-anisotropic` stayed correct and improved fractional one-axis cases by
// roughly 15-44%, while exact one-axis cases remain on exact fast paths.
// REJECT(perf): Specializing one-overlap vertical/horizontal area spans
// preserved bounded correctness but regressed near-identity and upscale cases by
// roughly 40-45% in `ditherette-bench run area`; keep the compact generic
// accumulation loops.
// REJECT(perf): Specializing two-overlap horizontal area spans preserved bounded
// correctness but regressed fractional downscales and near-identity cases by
// roughly 30-48% in `ditherette-bench run area`; keep the compact generic
// horizontal loop.
// REJECT(perf): Hoisting area division to one f32 reciprocal preserved bounded
// correctness but regressed representative large fractional cases by roughly
// 2-6% in `ditherette-bench run area`; keep the existing per-span f64 divide/cast
// order.

pub(super) fn resize_with_plan_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    let mut scratch = vec![0.0; plan.scratch_elements()];
    resize_with_scratch_into(source, output, plan, &mut scratch);
}

pub(super) fn resize_with_scratch_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
    vertical_row: &mut [f32],
) {
    if plan.same_width() {
        resize_vertical_only_into(source, output, plan);
        return;
    }

    if plan.same_height() {
        resize_horizontal_only_into(source, output, plan);
        return;
    }

    let source_row_byte_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = plan.output_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        vertical_row.fill(0.0);
        accumulate_vertical_row(
            source_data,
            source_row_byte_len,
            vertical_row,
            &plan.y_spans[output_y],
        );

        for (output_pixel, x_spans) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_spans)
        {
            write_horizontal_pixel(output_pixel, vertical_row, x_spans, plan.area);
        }
    }
}

pub(super) fn resize_rows_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
    y_start: u32,
) {
    let source_row_byte_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = plan.output_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();
    let mut vertical_row = vec![0.0; source_row_byte_len];

    for (local_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let output_y = y_start as usize + local_y;
        if plan.same_width() {
            write_vertical_row(
                output_row,
                source_data,
                source_row_byte_len,
                &plan.y_spans[output_y],
                plan.area,
            );
            continue;
        }

        if plan.same_height() {
            let source_row_start = output_y * source_row_byte_len;
            let source_row = &source_data[source_row_start..source_row_start + source_row_byte_len];
            for (output_pixel, x_spans) in output_row
                .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
                .zip(&plan.x_spans)
            {
                write_horizontal_source_pixel(output_pixel, source_row, x_spans, plan.area);
            }
            continue;
        }

        vertical_row.fill(0.0);
        accumulate_vertical_row(
            source_data,
            source_row_byte_len,
            &mut vertical_row,
            &plan.y_spans[output_y],
        );

        for (output_pixel, x_spans) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_spans)
        {
            write_horizontal_pixel(output_pixel, &vertical_row, x_spans, plan.area);
        }
    }
}

fn resize_vertical_only_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    let row_byte_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (output_y, output_row) in output.data_mut().chunks_exact_mut(row_byte_len).enumerate() {
        write_vertical_row(
            output_row,
            source_data,
            row_byte_len,
            &plan.y_spans[output_y],
            plan.area,
        );
    }
}

fn resize_horizontal_only_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    let source_row_byte_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = plan.output_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;

    for (source_row, output_row) in source
        .data()
        .chunks_exact(source_row_byte_len)
        .zip(output.data_mut().chunks_exact_mut(output_row_byte_len))
    {
        for (output_pixel, x_spans) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_spans)
        {
            write_horizontal_source_pixel(output_pixel, source_row, x_spans, plan.area);
        }
    }
}

// REJECT(perf): Flattening y spans into `{ first_source_index, overlaps }`
// preserved correctness but regressed fractional and upscale cases by roughly
// 2-9% in `ditherette-bench run area`; the separable path still reuses the
// copied `AxisOverlap` y layout.
// NOTE(perf): This planned coverage path handles fractional resizes. Exact
// integer down/up scales bypass it above and remain byte-exact.
fn accumulate_vertical_row(
    source_data: &[u8],
    source_row_byte_len: usize,
    vertical_row: &mut [f32],
    y_spans: &[super::plan::AxisOverlap],
) {
    for y_span in y_spans {
        let y_weight = y_span.overlap as f32;
        let source_start = y_span.source_index * source_row_byte_len;
        let source_row = &source_data[source_start..source_start + source_row_byte_len];

        for (vertical_pixel, source_pixel) in vertical_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(source_row.chunks_exact(rgba8::RGBA8_CHANNELS))
        {
            accumulate_weighted_pixel(vertical_pixel, source_pixel, y_weight);
        }
    }
}

fn write_vertical_row(
    output_row: &mut [u8],
    source_data: &[u8],
    source_row_byte_len: usize,
    y_spans: &[super::plan::AxisOverlap],
    area: f64,
) {
    for (output_pixel, output_accumulated) in output_row
        .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
        .zip((0..source_row_byte_len).step_by(rgba8::RGBA8_CHANNELS))
    {
        let mut accumulated = [0.0f32; rgba8::RGBA8_CHANNELS];

        for y_span in y_spans {
            let weight = (y_span.overlap / area) as f32;
            let source_start = y_span.source_index * source_row_byte_len + output_accumulated;
            accumulate_weighted_pixel(
                &mut accumulated,
                &source_data[source_start..source_start + rgba8::RGBA8_CHANNELS],
                weight,
            );
        }

        for channel in 0..rgba8::RGBA8_CHANNELS {
            output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
        }
    }
}

fn write_horizontal_source_pixel(
    output_pixel: &mut [u8],
    source_row: &[u8],
    x_spans: &super::plan::XAxisOverlapSpan,
    area: f64,
) {
    let mut accumulated = [0.0f32; rgba8::RGBA8_CHANNELS];
    let source_pixels = source_row[x_spans.first_byte_offset..x_spans.last_exclusive_byte_offset]
        .chunks_exact(rgba8::RGBA8_CHANNELS);

    for (x_overlap, source_pixel) in x_spans.overlaps.iter().copied().zip(source_pixels) {
        let weight = (x_overlap / area) as f32;
        accumulate_weighted_pixel(&mut accumulated, source_pixel, weight);
    }

    for channel in 0..rgba8::RGBA8_CHANNELS {
        output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
    }
}

fn write_horizontal_pixel(
    output_pixel: &mut [u8],
    vertical_row: &[f32],
    x_spans: &super::plan::XAxisOverlapSpan,
    area: f64,
) {
    let mut accumulated = [0.0f32; rgba8::RGBA8_CHANNELS];
    let vertical_pixels = vertical_row
        [x_spans.first_byte_offset..x_spans.last_exclusive_byte_offset]
        .chunks_exact(rgba8::RGBA8_CHANNELS);

    for (x_overlap, vertical_pixel) in x_spans.overlaps.iter().copied().zip(vertical_pixels) {
        let weight = (x_overlap / area) as f32;
        for channel in 0..rgba8::RGBA8_CHANNELS {
            accumulated[channel] += vertical_pixel[channel] * weight;
        }
    }

    for channel in 0..rgba8::RGBA8_CHANNELS {
        output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
    }
}

fn accumulate_weighted_pixel(accumulated: &mut [f32], source_pixel: &[u8], weight: f32) {
    for channel in 0..rgba8::RGBA8_CHANNELS {
        accumulated[channel] += f32::from(source_pixel[channel]) * weight;
    }
}
