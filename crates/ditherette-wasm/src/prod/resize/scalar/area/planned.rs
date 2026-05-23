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
// TODO(perf:path, rank=2): Gate the separable area path by span shape and output
// size so one-shot scratch traffic does not hurt small or one-axis cases.
// Benchmark the manifest `area` profile and accept only if representative
// aspect-preserving cases improve.
// TODO(perf:layout, rank=3, after perf:path fractional-area-separable-gate):
// Reuse separable area scratch storage across rows/calls once the path gate is
// chosen; one-shot benchmarks include allocation cost, so compare thread-local
// reuse against per-call allocation with `ditherette-bench run area`.
// TODO(perf:kernel, rank=4, after perf:path fractional-area-separable-gate):
// Specialize common one- and two-overlap vertical/horizontal separable kernels
// after path selection proves those shapes remain hot. Verify bounded area
// correctness, then benchmark `ditherette-bench run area`.
// TODO(perf:micro, rank=5, after perf:kernel fractional-area-separable-spans):
// If the separable path is accepted, test pre-normalized f32 axis weights or a
// precomputed reciprocal area inside that path only. This retests closed weight
// hoisting under changed one-shot/bounded/separable conditions; benchmark
// `ditherette-bench run area`.

pub(super) fn resize_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    let source_row_byte_len = source.dimensions().width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = plan.output_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();
    let mut vertical_row = vec![0.0; source_row_byte_len];

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
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
