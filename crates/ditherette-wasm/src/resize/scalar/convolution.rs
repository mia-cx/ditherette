use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        shared::convolution::Kernel,
    },
};

// TODO(perf): Store convolution weights in a flat buffer plus per-output ranges
// to avoid one Vec allocation per output coordinate and improve sequential
// access during sampling. Benchmark with `pnpm bench:resize:bicubic` and
// `pnpm bench:resize:lanczos3` before accepting.
// TODO(perf): Use fixed-size inline storage for compact kernels whose tap count
// is bounded in upscales, so bicubic/Lanczos2/Lanczos3 avoid heap allocations
// per output coordinate. Benchmark with `pnpm bench:resize:bicubic` and
// `pnpm bench:resize:lanczos3` before accepting.
// TODO(perf): Split x/y contribution types so x stores source byte offsets and y
// stores row byte offsets, removing repeated offset multiplication in the hot
// loops. Benchmark with `pnpm bench:resize:bicubic` and
// `pnpm bench:resize:lanczos3` before accepting.
#[derive(Debug)]
struct AxisContributions {
    first: usize,
    weights: Vec<f32>,
}

/// Allocates and resizes with an image-compatible separable convolution filter.
pub(crate) fn resize_with_convolution<K: Kernel>(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    kernel: K,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
        kernel,
        scale_aware,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-owned output buffer with an image-compatible separable convolution filter.
pub(crate) fn resize_with_convolution_into<K: Kernel>(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    kernel: K,
    scale_aware: bool,
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

    // TODO(perf): Add one-axis fast paths for same-width or same-height resizes
    // so pure vertical or horizontal convolution skips the unchanged axis.
    // Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Cache contribution plans keyed by source/output dimensions,
    // kernel, and scale-aware mode for repeated preview renders. Benchmark with
    // `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before
    // accepting.
    let source_width = source_dimensions.width_usize()?;
    let source_height = source_dimensions.height_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let y_contributions = prepare_axis_contributions(
        source_dimensions.height(),
        output_dimensions.height(),
        kernel,
        scale_aware,
    )?;
    let x_contributions = prepare_axis_contributions(
        source_dimensions.width(),
        output_dimensions.width(),
        kernel,
        scale_aware,
    )?;

    // TODO(perf): Reuse the vertical scratch buffer across resize calls to avoid
    // allocating a large f32 intermediate every frame. Benchmark with
    // `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before
    // accepting.
    // REJECT(perf): Streaming one vertical row into the horizontal pass instead
    // of materializing the full f32 intermediate preserved correctness but
    // regressed most `pnpm bench:resize:bicubic --baseline convolution_accepted`
    // scales by ~2-10%; keeping the full intermediate preserves horizontal row
    // locality for downscales.
    // TODO(perf): Process output rows in reusable row bands so future tiling can
    // share a bounded scratch buffer rather than one full intermediate image.
    // Benchmark with `pnpm bench:resize:bicubic:tiling` before accepting.
    let mut vertical_rgba = vec![0.0; output_height * source_width * rgba::RGBA_CHANNEL_COUNT];
    vertical_sample(
        source_rgba,
        source_width,
        source_height,
        &y_contributions,
        &mut vertical_rgba,
    );
    horizontal_sample(
        &vertical_rgba,
        source_width,
        output_width,
        &x_contributions,
        output_rgba,
    );

    Ok(())
}

fn vertical_sample(
    source_rgba: &[u8],
    source_width: usize,
    source_height: usize,
    y_contributions: &[AxisContributions],
    vertical_rgba: &mut [f32],
) {
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let vertical_row_byte_len = source_row_byte_len;

    // REJECT(perf): Filling the vertical row from the first y tap instead of
    // zeroing then adding all taps preserved correctness but had no meaningful
    // `pnpm bench:resize:bicubic --baseline convolution_accepted` win after row
    // slice traversal; keep the simpler zero-fill loop.
    for (output_y, y_contribution) in y_contributions.iter().enumerate() {
        let vertical_row = &mut vertical_rgba
            [output_y * vertical_row_byte_len..(output_y + 1) * vertical_row_byte_len];
        vertical_row.fill(0.0);

        // REJECT(perf): Precomputing y source row offsets in the contribution
        // plan preserved correctness but regressed 0.8x and 0.375x in
        // `pnpm bench:resize:bicubic --baseline convolution_accepted`; keep the
        // multiply in the vertical loop.
        for (weight_offset, weight) in y_contribution.weights.iter().enumerate() {
            let source_y = y_contribution.first + weight_offset;
            debug_assert!(source_y < source_height);
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
}

fn horizontal_sample(
    vertical_rgba: &[f32],
    source_width: usize,
    output_width: usize,
    x_contributions: &[AxisContributions],
    output_rgba: &mut [u8],
) {
    let vertical_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;

    // TODO(perf): Add row-band parallelism for convolution filters after the
    // scalar row path stabilizes; output rows are independent and should tile
    // similarly to nearest/area. Benchmark with `pnpm bench:resize:bicubic:tiling`
    // before accepting.
    for (output_y, output_row) in output_rgba
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let vertical_row = &vertical_rgba
            [output_y * vertical_row_byte_len..(output_y + 1) * vertical_row_byte_len];

        for (output_pixel, x_contribution) in output_row
            .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
            .zip(x_contributions)
        {
            let mut red = 0.0;
            let mut green = 0.0;
            let mut blue = 0.0;
            let mut alpha = 0.0;

            // REJECT(perf): Precomputing x source byte offsets in each
            // contribution improved enlargement and near-identity bicubic, but
            // regressed 0.25x/0.125x and threshold variants still regressed small
            // downscales in `pnpm bench:resize:bicubic --baseline convolution_accepted`.
            // TODO(perf): Unroll fixed-tap horizontal loops for compact kernels
            // so bicubic/Lanczos upscales avoid iterator overhead. Benchmark
            // with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
            // before accepting.
            for (weight_offset, weight) in x_contribution.weights.iter().enumerate() {
                let source_x = x_contribution.first + weight_offset;
                let vertical_offset = source_x * rgba::RGBA_CHANNEL_COUNT;
                red += vertical_row[vertical_offset] * weight;
                green += vertical_row[vertical_offset + 1] * weight;
                blue += vertical_row[vertical_offset + 2] * weight;
                alpha += vertical_row[vertical_offset + 3] * weight;
            }

            write_rounded_rgba(output_pixel, red, green, blue, alpha);
        }
    }
}

fn prepare_axis_contributions<K: Kernel>(
    source_size: u32,
    output_size: u32,
    kernel: K,
    scale_aware: bool,
) -> Result<Vec<AxisContributions>, ProcessingError> {
    let source_len = usize::try_from(source_size).map_err(|_| ProcessingError::SizeOverflow {
        context: "convolution source axis",
    })?;
    let output_len = usize::try_from(output_size).map_err(|_| ProcessingError::SizeOverflow {
        context: "convolution output axis",
    })?;
    let ratio = source_size as f32 / output_size as f32;
    let scale = if scale_aware { ratio.max(1.0) } else { 1.0 };
    let support = kernel.support() * scale;
    let mut contributions = Vec::with_capacity(output_len);

    // TODO(perf): Detect identity-axis contribution plans and represent them as
    // direct copies instead of building one single-tap Vec per coordinate.
    // Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Detect exact integer ratios where contribution patterns repeat
    // periodically and clone a short pattern table instead of evaluating every
    // output coordinate. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Use incremental input-coordinate updates instead of computing
    // `(output + 0.5) * ratio` for every output coordinate. Benchmark with
    // `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before
    // accepting.
    // TODO(perf): Split scale-aware minification planning from fixed-support
    // upscale planning so the common fixed-radius case avoids scale branches and
    // wider dynamic capacities. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    for output_coordinate in 0..output_len {
        let input = (output_coordinate as f32 + 0.5) * ratio;
        let left = clamp_i64(
            (input - support).floor() as i64,
            0,
            i64::from(source_size) - 1,
        ) as usize;
        let right = clamp_i64(
            (input + support).ceil() as i64,
            i64::try_from(left + 1).map_err(|_| ProcessingError::SizeOverflow {
                context: "convolution output axis",
            })?,
            i64::from(source_size),
        ) as usize;
        let center = input - 0.5;
        let mut first = left;
        // TODO(perf): Split contribution planning into edge and interior ranges;
        // interior coordinates can skip clamp calls and use known tap bounds.
        // Benchmark with `pnpm bench:resize:bicubic` and
        // `pnpm bench:resize:lanczos3` before accepting.
        let mut weights = Vec::with_capacity(right - left);
        let mut total_weight = 0.0;

        // TODO(perf): Add fixed-support specialized planners for bicubic and
        // fixed-window Lanczos so kernel weights are written into pre-sized
        // arrays without Vec growth checks. Benchmark with
        // `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before
        // accepting.
        for source_coordinate in left..right {
            let weight = kernel.weight((source_coordinate as f32 - center) / scale);
            if weight == 0.0 && weights.is_empty() {
                first += 1;
                continue;
            }

            weights.push(weight);
            total_weight += weight;
        }

        while weights.last() == Some(&0.0) {
            weights.pop();
        }

        // REJECT(perf): Normalizing weights with one reciprocal multiply instead
        // of dividing each tap changed f32 rounding and failed
        // `pnpm bench:resize:bicubic --baseline convolution_accepted`
        // correctness at 0.95x; keep per-tap division to match the oracle.
        if total_weight.abs() > f32::EPSILON {
            for weight in &mut weights {
                *weight /= total_weight;
            }
        }

        contributions.push(AxisContributions { first, weights });
    }

    debug_assert_eq!(contributions.len(), output_len);
    debug_assert!(contributions
        .iter()
        .all(|contribution| contribution.first < source_len));

    Ok(contributions)
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

fn write_rounded_rgba(output_pixel: &mut [u8], red: f32, green: f32, blue: f32, alpha: f32) {
    output_pixel[0] = round_u8(red);
    output_pixel[1] = round_u8(green);
    output_pixel[2] = round_u8(blue);
    output_pixel[3] = round_u8(alpha);
}

fn round_u8(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}
