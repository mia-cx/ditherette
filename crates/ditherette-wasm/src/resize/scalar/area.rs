use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::validate_resize_buffers,
};

#[doc(hidden)]
pub fn resize_rgba_area_scalar_into(
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

    // TODO(perf): Cache x/y range plans for repeated preview resizes with the
    // same dimensions so interactive downscales do not rebuild coverage metadata
    // every frame. Benchmark with `pnpm bench:resize:area` before accepting.
    // TODO(perf): Add single-axis downscale paths for same-width or same-height
    // resizes so exact area work only runs along the changing axis. Benchmark
    // with `pnpm bench:resize:area` before accepting.
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

    let x_ranges: Vec<_> = (0..output_width)
        .map(|output_x| SourceRange::for_output_pixel(output_x, x_scale, source_dimensions.width()))
        .collect();

    // TODO(perf): Iterate output rows with `chunks_exact_mut` instead of
    // recomputing byte offsets per output pixel. Benchmark with
    // `pnpm bench:resize:area` before accepting.
    for output_y in 0..output_height {
        let y_range = SourceRange::for_output_pixel(output_y, y_scale, source_dimensions.height());

        for (output_x, x_range) in x_ranges.iter().copied().enumerate() {
            // TODO(perf): Replace the temporary weighted_sums array with named
            // channel accumulators to reduce indexing and stack traffic in the
            // hottest loop. Benchmark with `pnpm bench:resize:area` before
            // accepting.
            let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];
            let mut total_weight = 0.0;

            // TODO(perf): Use a separable horizontal scratch pass followed by
            // vertical accumulation to avoid redoing x coverage work for every
            // covered source row. Benchmark with `pnpm bench:resize:area` before
            // accepting.
            // TODO(perf): Use row or integral prefix sums for full interior spans
            // so large downscales do O(1) full-span accumulation plus fractional
            // edge samples. Benchmark with `pnpm bench:resize:area` before
            // accepting.
            for source_y in y_range.first..y_range.last_exclusive {
                let y_weight = y_range.overlap_with(source_y);

                for source_x in x_range.first..x_range.last_exclusive {
                    let x_weight = x_range.overlap_with(source_x);
                    let sample_weight = x_weight * y_weight;
                    // TODO(perf): Carry row byte offsets through the source-y
                    // loop and increment source offsets by RGBA stride instead
                    // of multiplying in `pixel_byte_offset` for each sample.
                    // Benchmark with `pnpm bench:resize:area` before accepting.
                    let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
                    let source_pixel =
                        &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

                    // TODO(perf): Test f32 or fixed-point weights/sums against
                    // exact-reference byte output; lower precision may be faster
                    // if it still matches accepted cases. Benchmark with
                    // `pnpm bench:resize:area` before accepting.
                    weighted_sums[0] += f64::from(source_pixel[0]) * sample_weight;
                    weighted_sums[1] += f64::from(source_pixel[1]) * sample_weight;
                    weighted_sums[2] += f64::from(source_pixel[2]) * sample_weight;
                    weighted_sums[3] += f64::from(source_pixel[3]) * sample_weight;
                    total_weight += sample_weight;
                }
            }

            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);
            output_rgba[output_offset] = round_channel(weighted_sums[0] / total_weight);
            output_rgba[output_offset + 1] = round_channel(weighted_sums[1] / total_weight);
            output_rgba[output_offset + 2] = round_channel(weighted_sums[2] / total_weight);
            output_rgba[output_offset + 3] = round_channel(weighted_sums[3] / total_weight);
        }
    }

    Ok(())
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

#[derive(Debug, Clone, Copy)]
struct SourceRange {
    start: f64,
    end: f64,
    first: usize,
    last_exclusive: usize,
}

impl SourceRange {
    fn for_output_pixel(output_coordinate: usize, scale: f64, source_size: u32) -> Self {
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
        let source_start = source_coordinate as f64;
        let source_end = source_start + 1.0;

        (self.end.min(source_end) - self.start.max(source_start)).max(0.0)
    }
}

fn round_channel(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
#[path = "area_tests.rs"]
mod area_tests;
