use std::cell::RefCell;

use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

thread_local! {
    static BILINEAR2_VERTICAL_SCRATCH: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

/// Clean-room scalar bilinear_2 implementation seeded from the independent reference.
// NOTE(perf): Keep bilinear_2 experimental until it beats `baseline` across
// `pnpm bench:resize:bilinear-criterion`; row-scratch reuse made it much faster
// than the reference seed but it still trails baseline on every measured scale.
// TODO(perf:api): Decide whether bilinear_2 should stay an exact alternative or
// become a tolerance-based fast bilinear candidate. Compare against the shootout
// crates before changing rounding or pass order.
#[allow(dead_code)]
pub fn resize_rgba_bilinear_2(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_bilinear_2_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the bilinear_2 scalar implementation.
#[allow(dead_code)]
pub fn resize_rgba_bilinear_2_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    if source_dimensions == output_dimensions {
        output_rgba.copy_from_slice(source_rgba);
        return Ok(());
    }

    resize_rgba_triangle_filter_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

fn resize_rgba_triangle_filter_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    // TODO(perf:layout): Own x/y contribution plans in reusable scratch so
    // repeated preview resizes avoid per-call Vec allocation. Benchmark against
    // `baseline` and `bilinear_2` with `pnpm bench:resize:bilinear-criterion`.
    let y_contributions = prepare_axis_contributions(
        source_dimensions.height(),
        output_dimensions.height(),
        "bilinear output y axis",
    )?;
    let x_contributions = prepare_axis_contributions(
        source_dimensions.width(),
        output_dimensions.width(),
        "bilinear output x axis",
    )?;
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;

    BILINEAR2_VERTICAL_SCRATCH.with(|vertical_rgba| {
        let mut vertical_rgba = vertical_rgba.borrow_mut();
        vertical_rgba.resize(source_row_byte_len, 0.0);

        // TODO(perf:path, after perf:api bilinear2-contract): Try scale-class
        // dispatch for exact path shapes: two-tap upscale/near-identity,
        // vertical-first minify, and exact-ratio downscale. Keep byte equality
        // unless bilinear_2 explicitly becomes tolerance-based.
        for (output_row, y_contribution) in output_rgba
            .chunks_exact_mut(output_row_byte_len)
            .zip(&y_contributions)
        {
            let vertical_row = vertical_rgba.as_mut_slice();
            vertical_row.fill(0.0);

            for (weight_index, weight) in y_contribution.weights.iter().enumerate() {
                let source_y = y_contribution.first + weight_index;
                let source_row_start = source_y * source_row_byte_len;
                let source_row =
                    &source_rgba[source_row_start..source_row_start + source_row_byte_len];

                for (vertical_pixel, source_pixel) in vertical_row
                    .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
                    .zip(source_row.chunks_exact(rgba::RGBA_CHANNEL_COUNT))
                {
                    vertical_pixel[0] += f32::from(source_pixel[0]) * weight;
                    vertical_pixel[1] += f32::from(source_pixel[1]) * weight;
                    vertical_pixel[2] += f32::from(source_pixel[2]) * weight;
                    vertical_pixel[3] += f32::from(source_pixel[3]) * weight;
                }
            }

            for (output_pixel, x_contribution) in output_row
                .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
                .zip(&x_contributions)
            {
                let mut accumulated = [0.0; rgba::RGBA_CHANNEL_COUNT];

                for (weight_index, weight) in x_contribution.weights.iter().enumerate() {
                    let source_x = x_contribution.first + weight_index;
                    let vertical_offset = source_x * rgba::RGBA_CHANNEL_COUNT;

                    for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                        accumulated[channel] += vertical_row[vertical_offset + channel] * weight;
                    }
                }

                for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                    output_pixel[channel] = round_u8(accumulated[channel]);
                }
            }
        }

        Ok(())
    })
}

// TODO(perf:layout): Trim zero weights and flatten contribution weights into
// contiguous buffers with per-output ranges; this should reduce allocation and
// pointer chasing without forcing the rejected inline-weight layout.
fn prepare_axis_contributions(
    source_size: u32,
    output_size: u32,
    context: &'static str,
) -> Result<Vec<AxisContribution>, ProcessingError> {
    let source_len =
        usize::try_from(source_size).map_err(|_| ProcessingError::SizeOverflow { context })?;
    let output_len =
        usize::try_from(output_size).map_err(|_| ProcessingError::SizeOverflow { context })?;
    let ratio = source_size as f32 / output_size as f32;
    let scale = ratio.max(1.0);
    let support = scale;
    let mut contributions = Vec::with_capacity(output_len);

    for output_coordinate in 0..output_len {
        let input = (output_coordinate as f32 + 0.5) * ratio;
        let left = clamp_i64(
            (input - support).floor() as i64,
            0,
            i64::from(source_size) - 1,
        ) as usize;
        let right = clamp_i64(
            (input + support).ceil() as i64,
            i64::try_from(left + 1).map_err(|_| ProcessingError::SizeOverflow { context })?,
            i64::from(source_size),
        ) as usize;
        let center = input - 0.5;
        let mut weights = Vec::with_capacity(right - left);
        let mut sum = 0.0;

        for source_coordinate in left..right {
            let weight = triangle_weight((source_coordinate as f32 - center) / scale);
            weights.push(weight);
            sum += weight;
        }

        for weight in &mut weights {
            *weight /= sum;
        }

        contributions.push(AxisContribution {
            first: left,
            weights,
        });
    }

    debug_assert_eq!(contributions.len(), output_len);
    debug_assert!(contributions
        .iter()
        .all(|contribution| contribution.first < source_len));

    Ok(contributions)
}

fn triangle_weight(distance: f32) -> f32 {
    if distance.abs() < 1.0 {
        1.0 - distance.abs()
    } else {
        0.0
    }
}

fn round_u8(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}

#[derive(Debug, Clone)]
struct AxisContribution {
    first: usize,
    weights: Vec<f32>,
}
