//! Packed RGBA8 bilinear kernel.
//!
//! Production bilinear uses a separable triangle-filter kernel: each output row
//! first accumulates one vertical source-width scratch row, then gathers that row
//! horizontally. This is the same algorithmic shape as the old scalar path while
//! keeping planning and validation in the rewritten production stack.

use std::cell::RefCell;

use crate::image::{ImageView, ImageViewMut, Rgba8};
use crate::prod::contract::failure::Failure;

use super::plan::{AxisTap, BilinearResizePlan};

const RGBA8_CHANNELS: usize = 4;

thread_local! {
    static VERTICAL_SCRATCH: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

pub(super) fn resize_packed_rgba8_with_triangle_filter_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
) {
    if plan.scratch_elements() == 0 {
        resize_with_scratch_into(source, output, plan, &mut []);
        return;
    }
    VERTICAL_SCRATCH.with_borrow_mut(|vertical_row| {
        vertical_row.resize(plan.scratch_elements(), 0.0);
        resize_with_scratch_into(source, output, plan, vertical_row);
    });
}

pub(super) fn resize_with_scratch_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    vertical_row: &mut [f32],
) {
    resize_with_progress(source, output, plan, vertical_row, &mut |_| Ok(()))
        .expect("disabled progress cannot fail");
}

pub(super) fn resize_with_progress(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    vertical_row: &mut [f32],
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    let source_width = source.dimensions().width_usize();
    let source_height = source.dimensions().height_usize();
    let output_width = output.dimensions().width_usize();
    let output_height = output.dimensions().height_usize();
    let source_row_len = source_width * RGBA8_CHANNELS;
    let output_row_len = output_width * RGBA8_CHANNELS;

    if source_width == output_width {
        resize_height_only(
            source.data(),
            output.data_mut(),
            source_row_len,
            output_row_len,
            plan,
            vertical_row,
            progress,
        )?;
        return Ok(());
    }

    if source_height == output_height {
        resize_width_only(
            source.data(),
            output.data_mut(),
            source_row_len,
            output_row_len,
            plan,
            progress,
        )?;
        return Ok(());
    }

    let source_data = source.data();
    let output_data = output.data_mut();

    for (output_y, y_taps) in plan.y_taps.iter().enumerate() {
        vertical_row.fill(0.0);
        let y_weight_sum = accumulate_vertical(source_data, source_row_len, vertical_row, y_taps);

        let output_row_start = output_y * output_row_len;
        let output_row = &mut output_data[output_row_start..output_row_start + output_row_len];

        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(RGBA8_CHANNELS)
            .zip(&plan.x_taps)
        {
            write_horizontal_pixel(output_pixel, vertical_row, x_taps, y_weight_sum);
        }
        progress(output_y as u32 + 1)?;
    }
    Ok(())
}

pub(super) fn resize_packed_rgba8_rows_with_triangle_filter_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    y_start: u32,
) {
    if plan.source_dimensions().width() != plan.output_dimensions().width()
        && plan.source_dimensions().height() == plan.output_dimensions().height()
    {
        resize_rows_with_scratch_into(source, output, plan, y_start, &mut []);
        return;
    }
    VERTICAL_SCRATCH.with_borrow_mut(|vertical_row| {
        vertical_row.resize(source.dimensions().width_usize() * RGBA8_CHANNELS, 0.0);
        resize_rows_with_scratch_into(source, output, plan, y_start, vertical_row);
    });
}

pub(super) fn resize_rows_with_scratch_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
    y_start: u32,
    vertical_row: &mut [f32],
) {
    let source_width = source.dimensions().width_usize();
    let output_width = plan.output_dimensions().width_usize();
    let source_row_len = source_width * RGBA8_CHANNELS;
    let output_row_len = output_width * RGBA8_CHANNELS;
    let y_start = y_start as usize;
    let source_data = source.data();
    let output_data = output.data_mut();

    if source_width == output_width {
        for (local_y, output_row) in output_data.chunks_exact_mut(output_row_len).enumerate() {
            let output_y = y_start + local_y;
            vertical_row.fill(0.0);
            let y_weight_sum = accumulate_vertical(
                source_data,
                source_row_len,
                vertical_row,
                &plan.y_taps[output_y],
            );
            for (output_pixel, vertical_pixel) in output_row
                .chunks_exact_mut(RGBA8_CHANNELS)
                .zip(vertical_row.chunks_exact(RGBA8_CHANNELS))
            {
                output_pixel[0] = round_u8(vertical_pixel[0] / y_weight_sum);
                output_pixel[1] = round_u8(vertical_pixel[1] / y_weight_sum);
                output_pixel[2] = round_u8(vertical_pixel[2] / y_weight_sum);
                output_pixel[3] = round_u8(vertical_pixel[3] / y_weight_sum);
            }
        }
        return;
    }

    if source.dimensions().height_usize() == plan.output_dimensions().height_usize() {
        for (local_y, output_row) in output_data.chunks_exact_mut(output_row_len).enumerate() {
            let output_y = y_start + local_y;
            let source_row_start = output_y * source_row_len;
            let source_row = &source_data[source_row_start..source_row_start + source_row_len];
            for (output_pixel, x_taps) in output_row
                .chunks_exact_mut(RGBA8_CHANNELS)
                .zip(&plan.x_taps)
            {
                write_horizontal_source_pixel(output_pixel, source_row, x_taps);
            }
        }
        return;
    }

    for (local_y, output_row) in output_data.chunks_exact_mut(output_row_len).enumerate() {
        let output_y = y_start + local_y;
        vertical_row.fill(0.0);
        let y_weight_sum = accumulate_vertical(
            source_data,
            source_row_len,
            vertical_row,
            &plan.y_taps[output_y],
        );
        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(RGBA8_CHANNELS)
            .zip(&plan.x_taps)
        {
            write_horizontal_pixel(output_pixel, vertical_row, x_taps, y_weight_sum);
        }
    }
}

// ACCEPT(perf): Specializing identity-axis resizes keeps the single production
// bilinear path but skips the unnecessary separable scratch/gather pass.
// `bilinear` improved identity-axis cases by ~40-240% with representative 2D
// cases neutral to slightly faster.
// REJECT(perf): X-then-y separable bilinear order kept bounded correctness but
// repeatedly re-filtered source rows horizontally for each y tap; `bilinear`
// regressed uniform downscales by roughly 18-80% despite neutral x-only cases.
fn resize_height_only(
    source: &[u8],
    output: &mut [u8],
    source_row_len: usize,
    output_row_len: usize,
    plan: &BilinearResizePlan,
    vertical_row: &mut [f32],
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    for (output_y, y_taps) in plan.y_taps.iter().enumerate() {
        vertical_row.fill(0.0);
        let y_weight_sum = accumulate_vertical(source, source_row_len, vertical_row, y_taps);
        let output_start = output_y * output_row_len;
        let output_row = &mut output[output_start..output_start + output_row_len];

        for (output_pixel, vertical_pixel) in output_row
            .chunks_exact_mut(RGBA8_CHANNELS)
            .zip(vertical_row.chunks_exact(RGBA8_CHANNELS))
        {
            output_pixel[0] = round_u8(vertical_pixel[0] / y_weight_sum);
            output_pixel[1] = round_u8(vertical_pixel[1] / y_weight_sum);
            output_pixel[2] = round_u8(vertical_pixel[2] / y_weight_sum);
            output_pixel[3] = round_u8(vertical_pixel[3] / y_weight_sum);
        }
        progress(output_y as u32 + 1)?;
    }
    Ok(())
}

fn resize_width_only(
    source: &[u8],
    output: &mut [u8],
    source_row_len: usize,
    output_row_len: usize,
    plan: &BilinearResizePlan,
    progress: &mut impl FnMut(u32) -> Result<(), Failure>,
) -> Result<(), Failure> {
    for output_y in 0..plan.y_taps.len() {
        let source_row_start = output_y * source_row_len;
        let source_row = &source[source_row_start..source_row_start + source_row_len];
        let output_row_start = output_y * output_row_len;
        let output_row = &mut output[output_row_start..output_row_start + output_row_len];

        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(RGBA8_CHANNELS)
            .zip(&plan.x_taps)
        {
            write_horizontal_source_pixel(output_pixel, source_row, x_taps);
        }
        progress(output_y as u32 + 1)?;
    }
    Ok(())
}

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

fn write_horizontal_source_pixel(output_pixel: &mut [u8], source_row: &[u8], x_taps: &[AxisTap]) {
    let mut accumulated = [0.0; RGBA8_CHANNELS];
    let mut x_weight_sum = 0.0;

    for x_tap in x_taps {
        let x_weight = x_tap.weight as f32;
        x_weight_sum += x_weight;
        let source_start = x_tap.index * RGBA8_CHANNELS;

        accumulated[0] += f32::from(source_row[source_start]) * x_weight;
        accumulated[1] += f32::from(source_row[source_start + 1]) * x_weight;
        accumulated[2] += f32::from(source_row[source_start + 2]) * x_weight;
        accumulated[3] += f32::from(source_row[source_start + 3]) * x_weight;
    }

    output_pixel[0] = round_u8(accumulated[0] / x_weight_sum);
    output_pixel[1] = round_u8(accumulated[1] / x_weight_sum);
    output_pixel[2] = round_u8(accumulated[2] / x_weight_sum);
    output_pixel[3] = round_u8(accumulated[3] / x_weight_sum);
}

// CLOSE(perf): Removing per-output-pixel x/y weight summing depended on the
// rejected normalized f32 plan shape; keep explicit weight sums with indexed
// taps.
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
