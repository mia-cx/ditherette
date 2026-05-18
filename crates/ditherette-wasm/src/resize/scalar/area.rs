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

    // REJECT(perf): Caching precomputed x/y coverage plans with a single-entry
    // global Mutex+Arc cache preserved correctness but produced no meaningful
    // `pnpm bench:resize:area` win and regressed 0.25x.
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
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    // REJECT(perf): Packing all x/y weights into shared Vec arenas instead of
    // one small Vec per coverage preserved correctness but regressed 0.95x,
    // 0.875x, 0.8x, and 0.75x in `pnpm bench:resize:area`.
    let x_coverages: Vec<_> = (0..output_width)
        .map(|output_x| XCoverage::for_output_pixel(output_x, x_scale, source_dimensions.width()))
        .collect();
    // REJECT(perf): Precomputing source row byte offsets inside y coverage
    // preserved correctness but produced no meaningful downscale win in
    // `pnpm bench:resize:area`.
    let y_coverages: Vec<_> = (0..output_height)
        .map(|output_y| {
            AxisCoverage::for_output_pixel(output_y, y_scale, source_dimensions.height())
        })
        .collect();

    // REJECT(perf): Iterating output rows with `chunks_exact_mut` and writing
    // output pixels directly preserved correctness but regressed 0.8x, 0.75x,
    // 0.625x, 0.5x, 0.375x, 0.25x, and 0.125x in `pnpm bench:resize:area`.
    for (output_y, y_coverage) in y_coverages.iter().enumerate() {
        for (output_x, x_coverage) in x_coverages.iter().enumerate() {
            // REJECT(perf): Replacing weighted_sums with named channel
            // accumulators preserved correctness but regressed 2x and produced no
            // meaningful downscale win in `pnpm bench:resize:area`.
            let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];
            let mut total_weight = 0.0;

            // REJECT(perf): Precomputing x/y total weights and reusing their
            // product instead of accumulating total_weight in sample order failed
            // `pnpm bench:resize:area` correctness preflight at 0.95x
            // (one-byte mismatch), so preserve the exact accumulation order.
            // REJECT(perf): Reordering fractional area accumulation to process
            // each covered source row across all output columns preserved
            // correctness but regressed 2x, 0.95x, 0.875x, 0.8x, 0.75x,
            // 0.625x, and 0.375x in `pnpm bench:resize:area`.
            // TODO(perf): Use a separable horizontal scratch pass followed by
            // vertical accumulation to reuse horizontal source-row work across
            // covered output rows. Benchmark with `pnpm bench:resize:area`
            // before accepting.
            // REJECT(perf): Branching on full-span x/y weights to skip
            // multiplying by 1.0 preserved correctness but regressed 2x, 0.95x,
            // 0.875x, 0.8x, 0.75x, 0.625x, and 0.375x in
            // `pnpm bench:resize:area`.
            // REJECT(perf): Splitting each x coverage into weighted edges plus
            // a full-weight interior loop preserved correctness but regressed
            // 0.95x, 0.875x, 0.8x, 0.75x, 0.625x, and 0.375x in
            // `pnpm bench:resize:area`.
            // TODO(perf): Use row or integral prefix sums for full interior spans
            // so large downscales do O(1) full-span accumulation plus fractional
            // edge samples. Benchmark with `pnpm bench:resize:area` before
            // accepting.
            for (source_y_offset, y_weight) in y_coverage.weights.iter().copied().enumerate() {
                let source_y = y_coverage.first + source_y_offset;
                let source_row_start = source_y * source_row_byte_len;
                let source_start = source_row_start + x_coverage.first_byte_offset;
                let source_end = source_row_start + x_coverage.last_exclusive_byte_offset;
                let source_pixels =
                    source_rgba[source_start..source_end].chunks_exact(rgba::RGBA_CHANNEL_COUNT);

                for (x_weight, source_pixel) in
                    x_coverage.weights.iter().copied().zip(source_pixels)
                {
                    let sample_weight = x_weight * y_weight;

                    // REJECT(perf): Switching generic area weights/sums to f32
                    // failed correctness preflight at 0.95x in
                    // `pnpm bench:resize:area` (one-byte mismatch), so keep f64
                    // for fractional area accumulation.
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

#[derive(Debug)]
struct XCoverage {
    first_byte_offset: usize,
    last_exclusive_byte_offset: usize,
    weights: Vec<f64>,
}

impl XCoverage {
    fn for_output_pixel(output_coordinate: usize, scale: f64, source_size: u32) -> Self {
        let range = SourceRange::for_output_pixel(output_coordinate, scale, source_size);
        let weights = (range.first..range.last_exclusive)
            .map(|source_x| range.overlap_with(source_x))
            .collect();

        Self {
            first_byte_offset: range.first * rgba::RGBA_CHANNEL_COUNT,
            last_exclusive_byte_offset: range.last_exclusive * rgba::RGBA_CHANNEL_COUNT,
            weights,
        }
    }
}

#[derive(Debug)]
struct AxisCoverage {
    first: usize,
    weights: Vec<f64>,
}

impl AxisCoverage {
    fn for_output_pixel(output_coordinate: usize, scale: f64, source_size: u32) -> Self {
        let range = SourceRange::for_output_pixel(output_coordinate, scale, source_size);
        let weights = (range.first..range.last_exclusive)
            .map(|source_coordinate| range.overlap_with(source_coordinate))
            .collect();

        Self {
            first: range.first,
            weights,
        }
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
