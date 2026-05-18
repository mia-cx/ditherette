use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        shared::convolution::Kernel,
    },
};

// REJECT(perf): Storing convolution weights in one flat buffer plus per-output
// ranges preserved correctness and helped some downscales, but regressed 2x and
// 0.95x bicubic in `pnpm bench:resize:bicubic`; keep per-output Vec weights.
// REJECT(perf): Fixed-size inline contribution storage depends on fixed kernel
// metadata/planners; the metadata trial preserved correctness but caused broad
// bicubic regressions, so keep Vec-backed contribution weights for now.
// REJECT(perf): Splitting x/y contribution types for byte/row offsets was tested
// directly as precomputed x/y offsets and regressed bicubic downscales.
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

    // REJECT(perf): A same-height horizontal-only convolution fast path preserved
    // correctness, but adding it required planning x before y and regressed normal
    // bicubic scale benchmarks; keep the vertical-only fast path only.
    // REJECT(perf): Caching contribution plans is a cross-call ownership policy,
    // not a local scalar kernel optimization; Criterion single-call resize
    // benchmarks would measure cache plumbing more than convolution throughput.
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

    if source_width == output_width {
        vertical_sample_into_output(
            source_rgba,
            source_width,
            source_height,
            &y_contributions,
            output_rgba,
        );
        return Ok(());
    }

    let x_contributions = prepare_axis_contributions(
        source_dimensions.width(),
        output_dimensions.width(),
        kernel,
        scale_aware,
    )?;

    // REJECT(perf): Reusing the vertical scratch buffer across calls requires an
    // external workspace/cache API; local thread-local scratch would bake hidden
    // memory retention into scalar resize and is not represented by the current
    // single-call benchmark contract.
    // REJECT(perf): Streaming one vertical row into the horizontal pass instead
    // of materializing the full f32 intermediate preserved correctness but
    // regressed most `pnpm bench:resize:bicubic --baseline convolution_accepted`
    // scales by ~2-10%; keeping the full intermediate preserves horizontal row
    // locality for downscales.
    // REJECT(perf): Reusable row bands overlap the row-streaming trial, which
    // preserved correctness but regressed most bicubic scales by losing full-row
    // horizontal locality.
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
    // REJECT(perf): Unrolling four-tap vertical sampling preserved correctness
    // but regressed every bicubic benchmark scale by ~6-43%, likely from worse
    // row-stream locality; keep the compact weighted-row accumulation loop.
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

fn vertical_sample_into_output(
    source_rgba: &[u8],
    source_width: usize,
    source_height: usize,
    y_contributions: &[AxisContributions],
    output_rgba: &mut [u8],
) {
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;

    for (output_row, y_contribution) in output_rgba
        .chunks_exact_mut(source_row_byte_len)
        .zip(y_contributions)
    {
        for (output_pixel, source_x) in output_row
            .chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT)
            .zip(0..source_width)
        {
            let mut red = 0.0;
            let mut green = 0.0;
            let mut blue = 0.0;
            let mut alpha = 0.0;

            for (weight_offset, weight) in y_contribution.weights.iter().enumerate() {
                let source_y = y_contribution.first + weight_offset;
                debug_assert!(source_y < source_height);
                let source_offset =
                    source_y * source_row_byte_len + source_x * rgba::RGBA_CHANNEL_COUNT;
                red += f32::from(source_rgba[source_offset]) * weight;
                green += f32::from(source_rgba[source_offset + 1]) * weight;
                blue += f32::from(source_rgba[source_offset + 2]) * weight;
                alpha += f32::from(source_rgba[source_offset + 3]) * weight;
            }

            output_pixel[0] = round_u8(red);
            output_pixel[1] = round_u8(green);
            output_pixel[2] = round_u8(blue);
            output_pixel[3] = round_u8(alpha);
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

    // REJECT(perf): Row-band parallelism should live in resize/tiling adapters,
    // not this scalar base; keeping scalar single-threaded preserves the module
    // boundary while tiling evolves independently.
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
            // REJECT(perf): Splitting the four-tap horizontal fast path into
            // a separate all-four-tap row function preserved correctness but
            // regressed enlargement and near-identity bicubic; keep the inline
            // branch that was accepted with the four-tap unroll.
            if x_contribution.weights.len() == 4 {
                let mut vertical_offset = x_contribution.first * rgba::RGBA_CHANNEL_COUNT;
                let weight = x_contribution.weights[0];
                red += vertical_row[vertical_offset] * weight;
                green += vertical_row[vertical_offset + 1] * weight;
                blue += vertical_row[vertical_offset + 2] * weight;
                alpha += vertical_row[vertical_offset + 3] * weight;

                vertical_offset += rgba::RGBA_CHANNEL_COUNT;
                let weight = x_contribution.weights[1];
                red += vertical_row[vertical_offset] * weight;
                green += vertical_row[vertical_offset + 1] * weight;
                blue += vertical_row[vertical_offset + 2] * weight;
                alpha += vertical_row[vertical_offset + 3] * weight;

                vertical_offset += rgba::RGBA_CHANNEL_COUNT;
                let weight = x_contribution.weights[2];
                red += vertical_row[vertical_offset] * weight;
                green += vertical_row[vertical_offset + 1] * weight;
                blue += vertical_row[vertical_offset + 2] * weight;
                alpha += vertical_row[vertical_offset + 3] * weight;

                vertical_offset += rgba::RGBA_CHANNEL_COUNT;
                let weight = x_contribution.weights[3];
                red += vertical_row[vertical_offset] * weight;
                green += vertical_row[vertical_offset + 1] * weight;
                blue += vertical_row[vertical_offset + 2] * weight;
                alpha += vertical_row[vertical_offset + 3] * weight;
            } else {
                for (weight_offset, weight) in x_contribution.weights.iter().enumerate() {
                    let source_x = x_contribution.first + weight_offset;
                    let vertical_offset = source_x * rgba::RGBA_CHANNEL_COUNT;
                    red += vertical_row[vertical_offset] * weight;
                    green += vertical_row[vertical_offset + 1] * weight;
                    blue += vertical_row[vertical_offset + 2] * weight;
                    alpha += vertical_row[vertical_offset + 3] * weight;
                }
            }

            // REJECT(perf): A fused clamp/round/write helper looked neutral in
            // candidate comparison, but the baseline refresh showed broad small
            // downscale regressions in `pnpm bench:resize:bicubic`; keep the
            // explicit channel writes.
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

    // REJECT(perf): Identity-axis plans are handled at the resize boundary by
    // exact identity copy and the accepted same-width vertical-only fast path;
    // adding a second plan representation would duplicate that logic.
    // REJECT(perf): Exact integer-ratio pattern cloning has edge-specific clamps
    // and normalization differences; the planner cost is small next to sampling
    // after the accepted four-tap horizontal and zero-tap improvements.
    // REJECT(perf): Incremental input-coordinate updates changed f32 rounding
    // and failed `pnpm bench:resize:bicubic --baseline convolution_accepted`
    // correctness at 0.95x; keep the per-coordinate multiply.
    // REJECT(perf): Replacing ratio.max(1.0) with a dimension branch preserved
    // correctness but baseline refresh regressed several bicubic scales; keep
    // the compact scale-aware expression.
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
        // REJECT(perf): Splitting edge/interior planning adds plan complexity on
        // a setup path that is no longer the dominant bicubic cost; prior planner
        // representation changes regressed hot scales despite preserving output.
        let mut weights = Vec::with_capacity(right - left);
        let mut total_weight = 0.0;

        // REJECT(perf): Fixed-support specialized planners need the rejected
        // fixed kernel metadata/inline-storage direction; keep the generic
        // planner until a dedicated bicubic-only implementation replaces it.
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

fn round_u8(value: f32) -> u8 {
    value.clamp(0.0, 255.0).round() as u8
}
