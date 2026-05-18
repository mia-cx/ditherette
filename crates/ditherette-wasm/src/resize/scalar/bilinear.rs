use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::validate_resize_buffers,
};

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
    let output_height = output_dimensions.height_usize()?;
    // TODO(perf): Reuse cached contribution tables for repeated preview resizes
    // with the same dimensions to avoid rebuilding x/y weights every frame.
    // Benchmark with `pnpm bench:resize:bilinear` before accepting.
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
    // TODO(perf): Reuse the vertical scratch buffer across resize calls to avoid
    // allocating and zero-initializing `output_height * source_width * 4` f32s
    // per frame. Benchmark with `pnpm bench:resize:bilinear` before accepting.
    // TODO(perf): Stream one output row or a small row band through the vertical
    // and horizontal passes to reduce scratch footprint and cache pressure versus
    // materializing the full `output_height * source_width * 4` intermediate.
    // Benchmark with `pnpm bench:resize:bilinear` before accepting.
    let mut vertical_rgba = vec![0.0; output_height * source_width * rgba::RGBA_CHANNEL_COUNT];

    // TODO(perf): Add a same-height horizontal-only path to skip the vertical
    // scratch pass when only width changes. Benchmark with
    // `pnpm bench:resize:bilinear` before accepting.
    // TODO(perf): Add a same-width vertical-only path to skip horizontal
    // contribution planning and the horizontal pass when only height changes.
    // Benchmark with `pnpm bench:resize:bilinear` before accepting.
    // TODO(perf): Fill the vertical scratch row from the first y tap and add
    // remaining taps instead of zero-initializing the whole scratch and using
    // `+=` for every tap; this may reduce writes across all scales. Benchmark
    // with `pnpm bench:resize:bilinear` before accepting.
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    for (output_y, contribution) in y_contributions.iter().enumerate() {
        let vertical_row_start = output_y * source_row_byte_len;
        let vertical_row =
            &mut vertical_rgba[vertical_row_start..vertical_row_start + source_row_byte_len];

        for (weight_index, weight) in contribution.weights.iter().enumerate() {
            let source_y = contribution.first + weight_index;
            let source_row_start = source_y * source_row_byte_len;
            let source_row = &source_rgba[source_row_start..source_row_start + source_row_byte_len];

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
    }

    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;
    for (output_y, output_row) in output_rgba
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let vertical_row_start = output_y * source_row_byte_len;
        let vertical_row =
            &vertical_rgba[vertical_row_start..vertical_row_start + source_row_byte_len];

        for (output_pixel, contribution) in output_row
            .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
            .zip(&x_contributions)
        {
            let mut red = 0.0;
            let mut green = 0.0;
            let mut blue = 0.0;
            let mut alpha = 0.0;

            // TODO(perf): Store contribution source byte offsets during planning
            // so the horizontal pass can add precomputed offsets to the current
            // vertical row base instead of recomputing `source_x * 4` per tap.
            // Benchmark with `pnpm bench:resize:bilinear` before accepting.
            for (weight_index, weight) in contribution.weights.iter().enumerate() {
                let source_x = contribution.first + weight_index;
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
}

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

    // TODO(perf): Use incremental input-coordinate updates instead of computing
    // `(output + 0.5) * ratio` for every output coordinate. Benchmark with
    // `pnpm bench:resize:bilinear` before accepting.
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

        // TODO(perf): Specialize exact integer downscale ratios whose triangle
        // contribution patterns repeat periodically, so planning can clone a
        // short pattern instead of evaluating every output coordinate. Benchmark
        // with `pnpm bench:resize:bilinear` before accepting.
        // TODO(perf): Specialize enlargement and near-identity paths where the
        // triangle support contains at most two non-zero taps. Benchmark with
        // `pnpm bench:resize:bilinear` before accepting.
        // TODO(perf): Drop zero-weight edge taps during contribution planning to
        // avoid sampling pixels whose normalized weight remains zero. Benchmark
        // with `pnpm bench:resize:bilinear` before accepting.
        for source_coordinate in left..right {
            let weight = triangle_weight((source_coordinate as f32 - center) / scale);
            weights.push(weight);
            sum += weight;
        }

        // TODO(perf): Precompute normalized weights into a flat buffer plus
        // per-output ranges to avoid one Vec allocation per output coordinate
        // and improve cache locality. Benchmark with `pnpm bench:resize:bilinear`
        // before accepting.
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

// TODO(perf): Store contribution weights and source offsets in a struct-of-arrays
// layout, or split x/y contribution types, to improve sequential access in the
// vertical and horizontal passes. Benchmark with `pnpm bench:resize:bilinear`
// before accepting.
#[derive(Debug, Clone)]
struct AxisContribution {
    first: usize,
    weights: Vec<f32>,
}
