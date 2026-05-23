//! Packed RGBA8 bilinear kernel.
//!
//! Production bilinear uses a separable triangle-filter kernel: each output row
//! first accumulates one vertical source-width scratch row, then gathers that row
//! horizontally. This is the same algorithmic shape as the old scalar path while
//! keeping planning and validation in the rewritten production stack.

use std::cell::RefCell;

use crate::image::{ImageView, ImageViewMut, Rgba8};

use super::plan::{AxisTap, BilinearResizePlan};

const RGBA8_CHANNELS: usize = 4;

thread_local! {
    static VERTICAL_SCRATCH: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn resize_packed_rgba8_with_triangle_filter_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
) {
    VERTICAL_SCRATCH.with_borrow_mut(|vertical_row| {
        let source_width = source.dimensions().width_usize();
        let output_width = output.dimensions().width_usize();
        let source_row_len = source_width * RGBA8_CHANNELS;
        let output_row_len = output_width * RGBA8_CHANNELS;

        vertical_row.resize(source_row_len, 0.0);
        let source_data = source.data();
        let output_data = output.data_mut();

        for (output_y, y_taps) in plan.y_taps.iter().enumerate() {
            vertical_row.fill(0.0);
            let y_weight_sum = accumulate_vertical(
                source_data,
                source_row_len,
                vertical_row.as_mut_slice(),
                y_taps,
            );

            let output_row_start = output_y * output_row_len;
            let output_row = &mut output_data[output_row_start..output_row_start + output_row_len];

            for (output_pixel, x_taps) in output_row
                .chunks_exact_mut(RGBA8_CHANNELS)
                .zip(&plan.x_taps)
            {
                write_horizontal_pixel(output_pixel, vertical_row, x_taps, y_weight_sum);
            }
        }
    });
}

// CLOSE(perf): Removing per-output-pixel x/y weight summing depended on the
// rejected normalized f32 plan shape; keep explicit weight sums with indexed
// taps.
// TODO(perf:path, rank=2): Specialize width-only and height-only resizes so
// anisotropic identity-axis cases skip the unnecessary separable scratch/gather
// pass. Verify bounded correctness, then benchmark `ditherette-bench run
// bilinear`.
// NOTE(perf): This path intentionally uses f32 scratch accumulation. A f64
// separable scratch row is closer to the direct oracle but materially slower;
// the benchmark profile enforces bounded color-distance correctness for the
// single production bilinear path instead of byte-for-byte f64 grouping.
fn accumulate_vertical(
    source: &[u8],
    source_row_len: usize,
    vertical_row: &mut [f32],
    y_taps: &[AxisTap],
) -> f32 {
    let mut y_weight_sum = 0.0;

    for y_tap in y_taps {
        let y_weight = y_tap.weight as f32;
        y_weight_sum += y_weight;
        let source_start = y_tap.index * source_row_len;
        let source_row = &source[source_start..source_start + source_row_len];

        for (vertical_pixel, source_pixel) in vertical_row
            .chunks_exact_mut(RGBA8_CHANNELS)
            .zip(source_row.chunks_exact(RGBA8_CHANNELS))
        {
            vertical_pixel[0] += f32::from(source_pixel[0]) * y_weight;
            vertical_pixel[1] += f32::from(source_pixel[1]) * y_weight;
            vertical_pixel[2] += f32::from(source_pixel[2]) * y_weight;
            vertical_pixel[3] += f32::from(source_pixel[3]) * y_weight;
        }
    }

    y_weight_sum
}

fn write_horizontal_pixel(
    output_pixel: &mut [u8],
    vertical_row: &[f32],
    x_taps: &[AxisTap],
    y_weight_sum: f32,
) {
    let mut accumulated = [0.0; RGBA8_CHANNELS];
    let mut x_weight_sum = 0.0;

    for x_tap in x_taps {
        let x_weight = x_tap.weight as f32;
        x_weight_sum += x_weight;
        let source_start = x_tap.index * RGBA8_CHANNELS;

        accumulated[0] += vertical_row[source_start] * x_weight;
        accumulated[1] += vertical_row[source_start + 1] * x_weight;
        accumulated[2] += vertical_row[source_start + 2] * x_weight;
        accumulated[3] += vertical_row[source_start + 3] * x_weight;
    }

    let total_weight = x_weight_sum * y_weight_sum;
    output_pixel[0] = round_u8(accumulated[0] / total_weight);
    output_pixel[1] = round_u8(accumulated[1] / total_weight);
    output_pixel[2] = round_u8(accumulated[2] / total_weight);
    output_pixel[3] = round_u8(accumulated[3] / total_weight);
}

fn round_u8(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}
