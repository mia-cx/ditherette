//! Packed RGBA8 scalar kernel for production area resize.
//!
//! This kernel assumes the shared production resize boundary has already
//! normalized source and output rows to packed RGBA8. It mirrors the spec area
//! coverage formula while keeping RGBA8-specific row traversal in one place.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::AreaResizePlan;

// TODO(perf:path, rank=8, after perf:layout area-resize-plan): Split exact
// integer upscales into pixel replication plus row repeat, matching the old area
// scalar strategy. Benchmark 2x and 4x upscale cases with `ditherette-bench run
// area --baseline accepted`.
// TODO(perf:path, rank=11, after perf:layout area-resize-plan): Route fractional
// minification through compact precomputed x/y coverage, using the old
// fractional-minify area-style path as the first candidate. Benchmark 0.95x,
// 0.9x, 0.75x, and 0.5x with `ditherette-bench run area --baseline
// accepted`.
// TODO(perf:kernel, rank=12, after perf:path area-fractional-minify): Recreate
// old dynamic row-band tiling for large area outputs once scalar path classes
// are settled; use four-band vs near-source tiling choices as hypotheses, not
// constants. Benchmark with `ditherette-bench run area --baseline
// accepted` and representative large fractional cases.
// TODO(perf:path, rank=13, after perf:layout area-resize-plan): Test a
// separable horizontal-then-vertical area path with reusable scratch rows only
// under the new packed-RGBA8/custom-harness conditions; older notes warned that
// separable/prefix/integral variants can alter rounding or lose to direct
// coverage. Verify with `--oracle spec:resize:area:scalar` and benchmark
// `ditherette-bench run area --baseline accepted`.

pub(super) fn resize_exact_integer_downscale_into(
    source: ImageView<'_, Rgba8>,
    output: &mut ImageViewMut<'_, Rgba8>,
) -> bool {
    let Some((x_step, y_step)) = exact_integer_downscale_steps(source, output) else {
        return false;
    };

    resize_exact_block_downscale_into(source, output, x_step, y_step);
    true
}

pub(super) fn resize_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    for output_y in 0..plan.output_dimensions.height() {
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");
        let y_spans = &plan.y_spans[output_y as usize];

        for output_x in 0..plan.output_dimensions.width() {
            let x_spans = &plan.x_spans[output_x as usize];
            let output_start = output_x as usize * rgba8::RGBA8_CHANNELS;
            let output_pixel = &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS];
            let accumulated = accumulate_pixel(source, x_spans, y_spans, plan.area);

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
            let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];

            for source_y in source_y_start..source_y_start + y_step {
                let row_start = source_y * source_row_byte_len;
                let source_start = row_start + source_x_start * rgba8::RGBA8_CHANNELS;
                let source_end = source_start + x_step * rgba8::RGBA8_CHANNELS;
                for source_pixel in
                    source_data[source_start..source_end].chunks_exact(rgba8::RGBA8_CHANNELS)
                {
                    for channel in 0..rgba8::RGBA8_CHANNELS {
                        accumulated[channel] += f64::from(source_pixel[channel]) * weight;
                    }
                }
            }

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

// TODO(perf:kernel, rank=14, after perf:layout area-x-byte-spans): Flatten the
// remaining y spans or specialize this span-driven kernel once path classes are
// settled. Benchmark with `ditherette-bench run area --baseline accepted`.
fn accumulate_pixel(
    source: ImageView<'_, Rgba8>,
    x_spans: &super::plan::XAxisOverlapSpan,
    y_spans: &[super::plan::AxisOverlap],
    area: f64,
) -> [f64; rgba8::RGBA8_CHANNELS] {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];

    for y_span in y_spans {
        let source_row = source
            .row(y_span.source_index as u32)
            .expect("planned source y should stay in bounds");
        let source_pixels = source_row
            [x_spans.first_byte_offset..x_spans.last_exclusive_byte_offset]
            .chunks_exact(rgba8::RGBA8_CHANNELS);

        for (x_overlap, source_pixel) in x_spans.overlaps.iter().copied().zip(source_pixels) {
            let weight = x_overlap * y_span.overlap / area;

            // TODO(perf:micro, rank=15, after perf:kernel area-span-kernel):
            // Compare f32 accumulators only after the new custom harness and
            // span kernel are in place; old attempts changed rounding or lost,
            // so accept only if byte-identical to `spec:resize:area:scalar`
            // across `ditherette-bench run area --oracle spec:resize:area:scalar
            // --baseline accepted`.
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    accumulated
}
