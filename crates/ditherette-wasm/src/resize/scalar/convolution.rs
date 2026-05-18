use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        shared::convolution::Kernel,
    },
};

// TODO(perf): Store contributions in a flat buffer plus per-output ranges. One
// Vec per output coordinate creates allocation overhead and scattered reads in
// the hot loop.
// TODO(perf): Use fixed-size stack arrays or SmallVec-style storage for compact
// kernels whose tap count is bounded (bicubic/Lanczos2/Lanczos3 upscales) to
// avoid one heap allocation per output coordinate. Benchmark with
// `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before accepting.
// TODO(perf): Split x and y contribution types so x can store byte offsets and y
// can store row offsets without carrying unused generic fields. Benchmark with
// `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before accepting.
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
    // so pure vertical or horizontal convolution skips building and walking the
    // unchanged axis. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Add a reusable convolution plan keyed by dimensions, kernel,
    // and scale-aware mode so repeated previews reuse contribution tables.
    // TODO(perf): Reuse contribution Vec storage through thread-local scratch so
    // repeated resizes with changing dimensions avoid allocator churn even when a
    // cached plan misses. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Reuse a horizontal/vertical scratch buffer for the separable
    // path instead of allocating an intermediate image per resize. Benchmark with
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

    // TODO(perf): Split edge and interior rows. Interior convolution can skip
    // clamping/duplicate source taps and use fixed tap counts for bicubic,
    // Lanczos2, and Lanczos3. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    for (output_y, y_contribution) in y_contributions.iter().enumerate() {
        let vertical_row = &mut vertical_rgba
            [output_y * vertical_row_byte_len..(output_y + 1) * vertical_row_byte_len];

        for source_x in 0..source_width {
            let mut red = 0.0;
            let mut green = 0.0;
            let mut blue = 0.0;
            let mut alpha = 0.0;

            // TODO(perf): Precompute y source row byte offsets once per y
            // contribution so every output pixel in the row reuses them.
            // Benchmark with `pnpm bench:resize:bicubic` and
            // `pnpm bench:resize:lanczos3` before accepting.
            for (weight_offset, weight) in y_contribution.weights.iter().enumerate() {
                let source_y = y_contribution.first + weight_offset;
                debug_assert!(source_y < source_height);
                let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
                red += f32::from(source_rgba[source_offset]) * weight;
                green += f32::from(source_rgba[source_offset + 1]) * weight;
                blue += f32::from(source_rgba[source_offset + 2]) * weight;
                alpha += f32::from(source_rgba[source_offset + 3]) * weight;
            }

            let vertical_offset = source_x * rgba::RGBA_CHANNEL_COUNT;
            vertical_row[vertical_offset] = red;
            vertical_row[vertical_offset + 1] = green;
            vertical_row[vertical_offset + 2] = blue;
            vertical_row[vertical_offset + 3] = alpha;
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

    // TODO(perf): Process output rows with `chunks_exact_mut` and carry a row
    // base offset to avoid recomputing row offsets for every pixel. Benchmark
    // with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before
    // accepting.
    // TODO(perf): Add row-band tiling for convolution filters once the scalar hot
    // loop is stable; output rows are independent and should parallelize like
    // nearest/area tiling. Benchmark with `pnpm bench:resize:bicubic:tiling`
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

            // TODO(perf): Precompute x*y weight products per output pixel or per
            // recurring contribution pair so the four RGBA channels do not all
            // repeat the same floating-point multiply chain. Benchmark with
            // `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
            // before accepting.
            for (weight_offset, weight) in x_contribution.weights.iter().enumerate() {
                let source_x = x_contribution.first + weight_offset;
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

    // TODO(perf): Detect identity-axis contribution plans and return a compact
    // copy/alias representation rather than one single-tap Vec per coordinate.
    // Benchmark with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
    // before accepting.
    // TODO(perf): Detect exact integer ratios where contribution patterns repeat
    // periodically and clone a short pattern table instead of evaluating every
    // output coordinate. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Specialize non-scale-aware upscales separately from
    // scale-aware downscales so compact fixed-radius kernels avoid minification
    // planning overhead. Benchmark with `pnpm bench:resize:bicubic` and
    // `pnpm bench:resize:lanczos3` before accepting.
    // TODO(perf): Use incremental center updates instead of recomputing from a
    // division for every output coordinate.
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
        // TODO(perf): Split contribution planning into edge and interior ranges;
        // interior coordinates can skip source-coordinate clamp calls entirely.
        // Benchmark with `pnpm bench:resize:bicubic` and
        // `pnpm bench:resize:lanczos3` before accepting.
        // TODO(perf): Reserve exact tap capacity from radius/support so each
        // contribution Vec avoids growth checks.
        let mut weights = Vec::with_capacity(right - left);
        let mut total_weight = 0.0;

        // TODO(perf): Drop zero-weight edge taps while preserving contiguous
        // source indices, so fixed-support kernels avoid useless samples without
        // changing image-compatible contribution order. Benchmark with
        // `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3` before
        // accepting.
        for source_coordinate in left..right {
            let weight = kernel.weight((source_coordinate as f32 - center) / scale);
            weights.push(weight);
            total_weight += weight;
        }

        // TODO(perf): Pre-normalize with reciprocal multiply and consider
        // dropping zero/near-zero taps after normalization for Lanczos tails.
        // TODO(perf): Store the reciprocal total weight once and multiply samples
        // by it instead of dividing each sample during normalization. Benchmark
        // with `pnpm bench:resize:bicubic` and `pnpm bench:resize:lanczos3`
        // before accepting.
        if total_weight.abs() > f32::EPSILON {
            for weight in &mut weights {
                *weight /= total_weight;
            }
        }

        contributions.push(AxisContributions {
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

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.max(min).min(max)
}

fn round_u8(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}
