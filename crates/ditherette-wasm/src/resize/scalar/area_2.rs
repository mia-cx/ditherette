use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

/// Alternate scalar implementation seeded from the independent area reference.
// NOTE(perf): area_2 remains isolated from scalar/area.rs so benchmarks can
// compare different architectures with
// `pnpm bench:cmp --compare resize:area:scalar --to resize:area_2:scalar`.
#[allow(dead_code)]
pub fn resize_rgba_area_2(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_area_2_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free alternate scalar area implementation.
pub fn resize_rgba_area_2_into(
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

    if is_exact_2x_downscale(source_dimensions, output_dimensions) {
        resize_exact_2x_downscale_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    if is_exact_integer_downscale(source_dimensions, output_dimensions) {
        resize_exact_integer_downscale_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    let x_weights_by_output =
        AxisWeights::for_output_axis(output_width, x_scale, source_dimensions.width());
    let y_weights_by_output =
        AxisWeights::for_output_axis(output_height, y_scale, source_dimensions.height());

    // REJECT(perf): A top-level exact-integer minification path with integer
    // block sums preserved correctness but regressed all minification scales in
    // `pnpm bench:resize:area_2`; keep the shared weighted path for now.
    // NOTE(perf): The accepted single-row specialization covers the important
    // enlargement path; exact-integer top-level splitting regressed. Leave the
    // remaining fractional/large-minification cases on the shared weighted path.
    // NOTE(perf): The current benchmark set has landscape output shapes; a
    // transposed traversal for tall/narrow bands is not actionable without a
    // representative benchmark case.
    for output_y in 0..output_height {
        let y_weights = y_weights_by_output.weights_for(output_y);

        if let [(source_y, _)] = y_weights {
            let source_y = *source_y;
            for output_x in 0..output_width {
                let x_weights = x_weights_by_output.weights_for(output_x);
                let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);

                if let [(source_x, _)] = x_weights {
                    let source_offset = rgba::pixel_byte_offset(source_width, *source_x, source_y);
                    output_rgba[output_offset..output_offset + rgba::RGBA_CHANNEL_COUNT]
                        .copy_from_slice(
                            &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT],
                        );
                    continue;
                }

                let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];
                let mut total_weight = 0.0;

                for &(source_x, x_weight) in x_weights {
                    let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
                    let source_pixel =
                        &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

                    weighted_sums[0] += f64::from(source_pixel[0]) * x_weight;
                    weighted_sums[1] += f64::from(source_pixel[1]) * x_weight;
                    weighted_sums[2] += f64::from(source_pixel[2]) * x_weight;
                    weighted_sums[3] += f64::from(source_pixel[3]) * x_weight;
                    total_weight += x_weight;
                }

                output_rgba[output_offset] = round_channel(weighted_sums[0] / total_weight);
                output_rgba[output_offset + 1] = round_channel(weighted_sums[1] / total_weight);
                output_rgba[output_offset + 2] = round_channel(weighted_sums[2] / total_weight);
                output_rgba[output_offset + 3] = round_channel(weighted_sums[3] / total_weight);
            }
            continue;
        }

        for output_x in 0..output_width {
            let x_weights = x_weights_by_output.weights_for(output_x);
            let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];
            let mut total_weight = 0.0;

            // NOTE(perf): Axis weights are precomputed once; advancing x ranges
            // in the hot loop is superseded by the flattened AxisWeights plan.
            // REJECT(perf): Integer block/full-rectangle accumulation preserved
            // correctness but regressed the measured minification cases; avoid
            // unweighted interior byte sums on this path.
            // NOTE(perf): Source-driven/tile scatter would require a different
            // output accumulation buffer and did not fit the byte-exact gather
            // architecture that benchmarked well here.
            for &(source_y, y_weight) in y_weights {
                // REJECT(perf): Precomputing source row bounds and advancing a
                // byte cursor preserved correctness but regressed 2x, 0.75x,
                // and 0.125x in `pnpm bench:resize:area_2`; keep the direct
                // pixel_byte_offset expression.
                // NOTE(perf): X coverage is already a flat precomputed slice;
                // keep the generic loop until profiles show dispatch overhead.
                for &(source_x, x_weight) in x_weights {
                    let sample_weight = x_weight * y_weight;
                    let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
                    let source_pixel =
                        &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

                    // NOTE(perf): Source-driven scatter overlaps with the tile
                    // accumulator idea above and remains out of scope for the
                    // accepted gather architecture.
                    // REJECT(perf): Replacing weighted_sums with explicit lane
                    // locals preserved correctness but regressed most area_2
                    // downscales; keep the compact channel array accumulation.
                    weighted_sums[0] += f64::from(source_pixel[0]) * sample_weight;
                    weighted_sums[1] += f64::from(source_pixel[1]) * sample_weight;
                    weighted_sums[2] += f64::from(source_pixel[2]) * sample_weight;
                    weighted_sums[3] += f64::from(source_pixel[3]) * sample_weight;
                    total_weight += sample_weight;
                }
            }

            // REJECT(perf): Precomputing x/y total weights and multiplying them
            // once changed f64 rounding and failed area_2 correctness at 0.95x;
            // keep accumulating total_weight in the same tap order as sums.
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);
            output_rgba[output_offset] = round_channel(weighted_sums[0] / total_weight);
            output_rgba[output_offset + 1] = round_channel(weighted_sums[1] / total_weight);
            output_rgba[output_offset + 2] = round_channel(weighted_sums[2] / total_weight);
            output_rgba[output_offset + 3] = round_channel(weighted_sums[3] / total_weight);
        }
    }

    Ok(())
}

fn is_exact_2x_downscale(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    output_dimensions.width().checked_mul(2) == Some(source_dimensions.width())
        && output_dimensions.height().checked_mul(2) == Some(source_dimensions.height())
}

fn is_exact_integer_downscale(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    let source_width = source_dimensions.width();
    let source_height = source_dimensions.height();
    let output_width = output_dimensions.width();
    let output_height = output_dimensions.height();

    source_width >= output_width
        && source_height >= output_height
        && (source_width > output_width || source_height > output_height)
        && source_width.is_multiple_of(output_width)
        && source_height.is_multiple_of(output_height)
}

fn resize_exact_2x_downscale_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;

    for (output_y, output_row) in output_rgba
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let first_source_row_start = output_y * 2 * source_row_byte_len;
        let second_source_row_start = first_source_row_start + source_row_byte_len;

        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
            .enumerate()
        {
            let source_x_start = output_x * 2 * rgba::RGBA_CHANNEL_COUNT;
            let top_left = first_source_row_start + source_x_start;
            let top_right = top_left + rgba::RGBA_CHANNEL_COUNT;
            let bottom_left = second_source_row_start + source_x_start;
            let bottom_right = bottom_left + rgba::RGBA_CHANNEL_COUNT;

            for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                let sum = u64::from(source_rgba[top_left + channel])
                    + u64::from(source_rgba[top_right + channel])
                    + u64::from(source_rgba[bottom_left + channel])
                    + u64::from(source_rgba[bottom_right + channel]);
                output_pixel[channel] = round_average_channel(sum, 4);
            }
        }
    }

    Ok(())
}

fn resize_exact_integer_downscale_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let x_step = source_width / output_width;
    let y_step = source_dimensions.height_usize()? / output_dimensions.height_usize()?;
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;
    let divisor = (x_step * y_step) as u64;

    for (output_y, output_row) in output_rgba
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let source_y_start = output_y * y_step;
        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
            .enumerate()
        {
            let source_x_start = output_x * x_step;
            let mut red_sum = 0u64;
            let mut green_sum = 0u64;
            let mut blue_sum = 0u64;
            let mut alpha_sum = 0u64;

            for source_y in source_y_start..source_y_start + y_step {
                let row_start = source_y * source_row_byte_len;
                let source_start = row_start + source_x_start * rgba::RGBA_CHANNEL_COUNT;
                let source_end = source_start + x_step * rgba::RGBA_CHANNEL_COUNT;
                for source_pixel in
                    source_rgba[source_start..source_end].chunks_exact(rgba::RGBA_CHANNEL_COUNT)
                {
                    red_sum += u64::from(source_pixel[0]);
                    green_sum += u64::from(source_pixel[1]);
                    blue_sum += u64::from(source_pixel[2]);
                    alpha_sum += u64::from(source_pixel[3]);
                }
            }

            output_pixel[0] = round_average_channel(red_sum, divisor);
            output_pixel[1] = round_average_channel(green_sum, divisor);
            output_pixel[2] = round_average_channel(blue_sum, divisor);
            output_pixel[3] = round_average_channel(alpha_sum, divisor);
        }
    }

    Ok(())
}

fn round_average_channel(sum: u64, divisor: u64) -> u8 {
    ((sum * 2 + divisor) / (divisor * 2)).min(u64::from(u8::MAX)) as u8
}

#[derive(Debug)]
struct AxisWeights {
    ranges: Vec<AxisWeightRange>,
    weights: Vec<(usize, f64)>,
}

#[derive(Debug, Clone, Copy)]
struct AxisWeightRange {
    start: usize,
    len: usize,
}

impl AxisWeights {
    fn for_output_axis(output_len: usize, scale: f64, source_size: u32) -> Self {
        let mut ranges = Vec::with_capacity(output_len);
        let mut weights = Vec::new();

        for output_coordinate in 0..output_len {
            let range = SourceRange::for_output_pixel(output_coordinate, scale, source_size);
            let start = weights.len();

            weights.extend(
                (range.first..range.last_exclusive).map(|source_coordinate| {
                    (source_coordinate, range.overlap_with(source_coordinate))
                }),
            );

            ranges.push(AxisWeightRange {
                start,
                len: weights.len() - start,
            });
        }

        Self { ranges, weights }
    }

    fn weights_for(&self, output_coordinate: usize) -> &[(usize, f64)] {
        let range = self.ranges[output_coordinate];
        &self.weights[range.start..range.start + range.len]
    }
}

#[derive(Debug, Clone, Copy)]
struct SourceRange {
    start: f64,
    end: f64,
    first: usize,
    last_exclusive: usize,
}

impl SourceRange {
    fn for_output_pixel(output_coordinate: usize, scale: f64, source_size: u32) -> Self {
        // Area resize maps each output pixel to a continuous source interval.
        // Each source pixel contributes in proportion to interval overlap.
        // NOTE(perf): Keep f64 endpoints: the precomputed-total-weight attempt
        // showed this code is sensitive to rounding order, and rational metadata
        // is only useful with a separate exact-arithmetic path.
        let start = output_coordinate as f64 * scale;
        let end = (output_coordinate + 1) as f64 * scale;
        let first = start.floor() as usize;
        let last_exclusive = (end.ceil() as usize).min(source_size as usize);

        Self {
            start,
            end,
            first,
            last_exclusive,
        }
    }

    fn overlap_with(self, source_coordinate: usize) -> f64 {
        // REJECT(perf): Preclassifying first/interior/last weights while
        // constructing range metadata preserved correctness but regressed
        // 0.95x, 0.5x, 0.375x, 0.25x, and 0.125x in area_2.
        let source_start = source_coordinate as f64;
        let source_end = source_start + 1.0;

        (self.end.min(source_end) - self.start.max(source_start)).max(0.0)
    }
}

fn round_channel(value: f64) -> u8 {
    // REJECT(perf): Precomputed reciprocal/total-weight variants changed f64
    // rounding and failed area_2 correctness; keep per-channel division.
    value.round().clamp(0.0, 255.0) as u8
}
