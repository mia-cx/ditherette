use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

/// Alternate scalar implementation seeded from the independent area reference.
// TODO(perf): Treat area_2 as a clean-room optimizer and keep changes isolated
// from scalar/area.rs so benchmarks can compare different architectures with
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

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    let x_weights_by_output =
        AxisWeights::for_output_axis(output_width, x_scale, source_dimensions.width());
    let y_weights_by_output =
        AxisWeights::for_output_axis(output_height, y_scale, source_dimensions.height());

    // TODO(perf): Split area_2 into architecture-specific paths at the top:
    // enlargement, mild fractional minification, exact integer minification, and
    // large minification. Benchmark each split with `pnpm bench:resize:area_2`
    // or bench:cmp before accepting.
    // TODO(perf): Add a transposed traversal variant for tall/narrow output bands
    // so the hotter loop walks contiguous source bytes when y coverage is wider
    // than x coverage. Benchmark with area_2 fractional downscales before
    // accepting.
    for output_y in 0..output_height {
        let y_weights = y_weights_by_output.weights_for(output_y);

        // TODO(perf): For rows whose y_range covers one source row, dispatch to a
        // horizontal-only area_2 kernel that avoids y_weight multiplication and
        // total_weight accumulation. Benchmark upscale and 0.95x before
        // accepting.
        for output_x in 0..output_width {
            let x_weights = x_weights_by_output.weights_for(output_x);
            let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];
            let mut total_weight = 0.0;

            // TODO(perf): Incrementally advance x_range across output_x instead
            // of rebuilding it with floor/ceil; preserve exact f64 endpoints by
            // deriving start/end from integer numerators. Benchmark with area_2
            // before accepting.
            // TODO(perf): Detect x_range/y_range pairs that fully cover a source
            // pixel grid rectangle and route interior pixels through unweighted
            // byte sums plus weighted edge strips. Benchmark moderate downscales
            // before accepting.
            // TODO(perf): Use a row-major tile accumulator for small output tiles
            // so one source pixel contributes to several neighboring output
            // pixels, reversing the current output-pixel gathers. Benchmark
            // enlargement and near-identity downscale before accepting.
            for &(source_y, y_weight) in y_weights {
                // REJECT(perf): Precomputing source row bounds and advancing a
                // byte cursor preserved correctness but regressed 2x, 0.75x,
                // and 0.125x in `pnpm bench:resize:area_2`; keep the direct
                // pixel_byte_offset expression.
                // TODO(perf): Specialize the common two-column/four-column x
                // coverage cases with straight-line edge/interior formulas
                // instead of generic nested overlap calls. Benchmark 0.75x and
                // 0.625x before accepting.
                for &(source_x, x_weight) in x_weights {
                    let sample_weight = x_weight * y_weight;
                    let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
                    let source_pixel =
                        &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

                    // TODO(perf): Try source-driven weighted scatter for area_2:
                    // precompute which output x/y intervals each source pixel
                    // overlaps, then add one loaded RGBA sample to multiple
                    // outputs. Benchmark near-identity and enlargement before
                    // accepting.
                    // TODO(perf): Accumulate color*weight and alpha*weight as two
                    // f64x2-style pairs or explicit lane structs to reduce array
                    // indexing in the inner loop. Benchmark with area_2 before
                    // accepting.
                    weighted_sums[0] += f64::from(source_pixel[0]) * sample_weight;
                    weighted_sums[1] += f64::from(source_pixel[1]) * sample_weight;
                    weighted_sums[2] += f64::from(source_pixel[2]) * sample_weight;
                    weighted_sums[3] += f64::from(source_pixel[3]) * sample_weight;
                    total_weight += sample_weight;
                }
            }

            // TODO(perf): For exact arithmetic paths, compute total_weight from
            // x/y interval area once (`x_scale * y_scale`) instead of summing per
            // tap; benchmark carefully because f64 rounding must stay byte-exact.
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);
            output_rgba[output_offset] = round_channel(weighted_sums[0] / total_weight);
            output_rgba[output_offset + 1] = round_channel(weighted_sums[1] / total_weight);
            output_rgba[output_offset + 2] = round_channel(weighted_sums[2] / total_weight);
            output_rgba[output_offset + 3] = round_channel(weighted_sums[3] / total_weight);
        }
    }

    Ok(())
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
        // TODO(perf): Represent area_2 ranges as rational numerators over
        // output_size instead of f64 endpoints so exact-integer and repeating
        // fractional patterns can be detected without rounding drift. Benchmark
        // with `pnpm bench:cmp --compare resize:area:scalar --to resize:area_2:scalar`
        // before accepting.
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
        // TODO(perf): Replace min/max overlap math with preclassified left-edge,
        // interior, and right-edge weights when constructing range metadata.
        // Benchmark with area_2 before accepting.
        let source_start = source_coordinate as f64;
        let source_end = source_start + 1.0;

        (self.end.min(source_end) - self.start.max(source_start)).max(0.0)
    }
}

fn round_channel(value: f64) -> u8 {
    // TODO(perf): Batch four channel divisions before rounding, or multiply by a
    // precomputed reciprocal total_weight where correctness allows it. Benchmark
    // with area_2 before accepting.
    value.round().clamp(0.0, 255.0) as u8
}
