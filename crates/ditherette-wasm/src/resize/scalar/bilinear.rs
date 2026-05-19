use std::cell::RefCell;

use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::validate_resize_buffers,
};

thread_local! {
    static BILINEAR_VERTICAL_SCRATCH: RefCell<Vec<f32>> = const { RefCell::new(Vec::new()) };
}

// TODO(perf:harness): Add a dedicated bilinear shootout/baseline group that
// records exact Ditherette/image-compatible timing separately from tolerance
// based `fast_image_resize`/`resize` comparisons. Use scales
// `2,1.8,1.5,0.99,0.95,0.875,0.75,0.5,0.25,0.125` before changing kernels.
// TODO(perf:api): Decide whether bilinear should expose an exact
// image-compatible mode plus a tolerance-based fast mode; external crates are
// 1.6-7x faster in the shootout but often differ by max Δ 1.

pub(crate) fn resize_rgba_bilinear_into(
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
    // REJECT(perf): Caching x/y contribution tables in thread-local scratch
    // preserved correctness but produced no meaningful all-scale
    // `pnpm bench:resize:bilinear` win; 0.375x improved ~2%, but the other
    // scales were neutral or within noise, so keep simpler per-call planning.
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

    // TODO(perf:layout, after perf:api bilinear-fast-contract): Prototype a
    // reusable bilinear plan/scratch object that owns x/y contributions and row
    // buffers across repeated preview resizes. Benchmark against the current
    // thread-local vertical scratch with `pnpm bench:resize:shootout --filter bilinear`.
    BILINEAR_VERTICAL_SCRATCH.with(|vertical_rgba| {
        let mut vertical_rgba = vertical_rgba.borrow_mut();
        vertical_rgba.resize(source_row_byte_len, 0.0);

        for (output_row, y_contribution) in output_rgba
            .chunks_exact_mut(output_row_byte_len)
            .zip(&y_contributions)
        {
            let vertical_row = vertical_rgba.as_mut_slice();

            let (first_weight, remaining_weights) = y_contribution
                .weights
                .split_first()
                .expect("bilinear contributions always have at least one weight");
            let first_source_row_start = y_contribution.first * source_row_byte_len;
            let first_source_row =
                &source_rgba[first_source_row_start..first_source_row_start + source_row_byte_len];

            for (vertical_pixel, source_pixel) in vertical_row
                .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
                .zip(first_source_row.chunks_exact(rgba::RGBA_CHANNEL_COUNT))
            {
                vertical_pixel[0] = f32::from(source_pixel[0]) * first_weight;
                vertical_pixel[1] = f32::from(source_pixel[1]) * first_weight;
                vertical_pixel[2] = f32::from(source_pixel[2]) * first_weight;
                vertical_pixel[3] = f32::from(source_pixel[3]) * first_weight;
            }

            for (weight_offset, weight) in remaining_weights.iter().enumerate() {
                let source_y = y_contribution.first + weight_offset + 1;
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

            // TODO(perf:path, after perf:layout bilinear-plan): Compare this
            // current vertical-then-horizontal gather with scale-class-specific
            // paths: two-tap upscale/near-identity, exact-ratio shrink, and
            // strong minify. Benchmark before adding leaf SIMD/micro-kernel work.
            for (output_pixel, x_contribution) in output_row
                .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
                .zip(&x_contributions)
            {
                let mut red = 0.0;
                let mut green = 0.0;
                let mut blue = 0.0;
                let mut alpha = 0.0;

                // REJECT(perf): Precomputing contribution source byte offsets
                // and incrementing them in the horizontal pass preserved
                // correctness but regressed most scales by ~3-6% in
                // `pnpm bench:resize:bilinear`; keep deriving offsets from
                // source_x in the loop.
                for (weight_index, weight) in x_contribution.weights.iter().enumerate() {
                    let source_x = x_contribution.first + weight_index;
                    let vertical_offset = source_x * rgba::RGBA_CHANNEL_COUNT;

                    red += vertical_row[vertical_offset] * weight;
                    green += vertical_row[vertical_offset + 1] * weight;
                    blue += vertical_row[vertical_offset + 2] * weight;
                    alpha += vertical_row[vertical_offset + 3] * weight;
                }

                output_pixel[0] = round_u8(red);
                output_pixel[1] = round_u8(green);
                output_pixel[2] = round_u8(blue);
                output_pixel[3] = round_u8(alpha);
            }
        }

        Ok(())
    })
}

// TODO(perf:layout): Investigate a compact fixed-inline contribution layout
// for bilinear where most upscale/near-identity pixels have two taps per axis.
// Benchmark memory locality and allocation count before replacing Vec weights.
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

    // REJECT(perf): Incremental input-coordinate updates changed f32 rounding
    // and failed benchmark correctness at 0.95x against the independent bilinear
    // reference; keep direct `(output + 0.5) * ratio` evaluation.
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
        let mut first = left;
        let mut weights = Vec::with_capacity(right - left);
        let mut sum = 0.0;

        // REJECT(perf): Exact integer-ratio pattern cloning has clamp and f32
        // normalization edge cases; planner setup is not a dominant bilinear cost
        // after the vertical scratch and per-call planning simplifications.
        // REJECT(perf): Enlargement/near-identity specialization duplicates the
        // generic two-pass bilinear path for little gain; the generic planner
        // already trims zero taps and uses compact per-axis contributions.
        for source_coordinate in left..right {
            let weight = triangle_weight((source_coordinate as f32 - center) / scale);
            if weight == 0.0 && weights.is_empty() {
                first += 1;
                continue;
            }

            weights.push(weight);
            sum += weight;
        }

        while weights.last() == Some(&0.0) {
            weights.pop();
        }

        // REJECT(perf): Flat weight buffers mirror the convolution trial, which
        // preserved correctness but regressed enlargement/near-identity cases;
        // keep per-output Vec weights for straightforward hot-loop indexing.
        for weight in &mut weights {
            *weight /= sum;
        }

        contributions.push(AxisContribution { first, weights });
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

// REJECT(perf): Struct-of-arrays contribution storage or split x/y offset types
// overlap the precomputed-offset trial above, which regressed most bilinear
// scales by ~3-6%; keep the compact shared contribution representation.
#[derive(Debug, Clone)]
struct AxisContribution {
    first: usize,
    weights: Vec<f32>,
}
