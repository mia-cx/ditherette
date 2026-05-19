use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        shared::convolution::Kernel,
    },
};

/// Allocates and resizes with the straightforward image-compatible separable convolution path.
#[allow(dead_code)]
pub(crate) fn resize_with_convolution_2<K: Kernel>(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    kernel: K,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_with_convolution_2_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
        kernel,
        scale_aware,
    )?;
    Ok(output_rgba)
}

/// Straightforward reference implementation for image-compatible separable convolution filters.
#[allow(dead_code)]
pub(crate) fn resize_with_convolution_2_into<K: Kernel>(
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

    let source_width = source_dimensions.width_usize()?;
    let source_height = source_dimensions.height_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    // REJECT(perf): A flat contribution plan shared by bicubic_2 and lanczos3_2
    // reduced allocation count but regressed most `pnpm crit:resize:convolution_2`
    // cases by ~1-3%, likely from extra span indexing in hot loops.
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

    // TODO(perf:api, rank=2): Add caller-owned scratch/plan plumbing for convolution_2
    // so repeated fixture/scale benchmark passes can reuse vertical buffers and prepared
    // weights. Benchmark scalar_2 vs scalar with `pnpm crit:cmp --compare resize:bicubic:scalar --to resize:bicubic:scalar_2`.
    let mut vertical_rgba = vec![0.0; output_height * source_width * rgba::RGBA_CHANNEL_COUNT];
    vertical_sample(
        source_rgba,
        source_width,
        source_height,
        output_height,
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

#[derive(Debug)]
struct AxisContributions {
    first: usize,
    weights: Vec<f32>,
}

fn vertical_sample(
    source_rgba: &[u8],
    source_width: usize,
    source_height: usize,
    output_height: usize,
    y_contributions: &[AxisContributions],
    vertical_rgba: &mut [f32],
) {
    const LARGE_SOURCE_PIXELS: usize = 2_000_000;
    let source_pixels = source_width * source_height;

    // NOTE(perf): Source-row vertical accumulation regressed small upscales by ~6-8%
    // but improved both bicubic_2 and lanczos3_2 downscales by ~27-59% in
    // `pnpm crit:resize:convolution_2 --baseline conv2_accepted`. Keep small upscales
    // on the original output-pixel loop until a separate upscale path is justified.
    if output_height <= source_height || source_pixels >= LARGE_SOURCE_PIXELS {
        vertical_sample_by_source_row(
            source_rgba,
            source_width,
            source_height,
            y_contributions,
            vertical_rgba,
        );
    } else {
        vertical_sample_by_output_pixel(
            source_rgba,
            source_width,
            source_height,
            y_contributions,
            vertical_rgba,
        );
    }
}

fn vertical_sample_by_source_row(
    source_rgba: &[u8],
    source_width: usize,
    source_height: usize,
    y_contributions: &[AxisContributions],
    vertical_rgba: &mut [f32],
) {
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let vertical_row_byte_len = source_row_byte_len;

    for (output_y, y_contribution) in y_contributions.iter().enumerate() {
        let vertical_row = &mut vertical_rgba
            [output_y * vertical_row_byte_len..(output_y + 1) * vertical_row_byte_len];

        vertical_row.fill(0.0);

        // TODO(perf:kernel, rank=5, after perf:layout flat-contribution-plan):
        // Specialize fixed-tap bicubic and lanczos3 vertical loops once the plan exposes
        // tap counts without nested Vec iteration. Benchmark bicubic_2 and lanczos3_2 separately.
        for (weight_offset, weight) in y_contribution.weights.iter().enumerate() {
            let source_y = y_contribution.first + weight_offset;
            debug_assert!(source_y < source_height);
            let source_row_start = source_y * source_row_byte_len;
            let source_row = &source_rgba[source_row_start..source_row_start + source_row_byte_len];

            for (source_pixel, vertical_pixel) in source_row
                .chunks_exact(rgba::RGBA_CHANNEL_COUNT)
                .zip(vertical_row.chunks_exact_mut(rgba::RGBA_CHANNEL_COUNT))
            {
                vertical_pixel[0] += f32::from(source_pixel[0]) * weight;
                vertical_pixel[1] += f32::from(source_pixel[1]) * weight;
                vertical_pixel[2] += f32::from(source_pixel[2]) * weight;
                vertical_pixel[3] += f32::from(source_pixel[3]) * weight;
            }
        }
    }
}

fn vertical_sample_by_output_pixel(
    source_rgba: &[u8],
    source_width: usize,
    source_height: usize,
    y_contributions: &[AxisContributions],
    vertical_rgba: &mut [f32],
) {
    let vertical_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;

    for (output_y, y_contribution) in y_contributions.iter().enumerate() {
        let vertical_row = &mut vertical_rgba
            [output_y * vertical_row_byte_len..(output_y + 1) * vertical_row_byte_len];

        for source_x in 0..source_width {
            let mut red = 0.0;
            let mut green = 0.0;
            let mut blue = 0.0;
            let mut alpha = 0.0;

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

            // TODO(perf:kernel, rank=6, after perf:layout flat-contribution-plan):
            // Evaluate row-pair/channel-unrolled horizontal kernels after contribution layout
            // is fixed; this may reduce bounds checks and weight loads in the hottest pass.
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

    // TODO(perf:path, rank=4, after perf:layout flat-contribution-plan): Detect stable
    // exact-ratio/fractional-minify tap patterns and reuse contribution spans instead of
    // recomputing nearly identical weight vectors for every output coordinate.
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
        let mut weights = Vec::with_capacity(right - left);
        let mut total_weight = 0.0;

        for source_coordinate in left..right {
            let weight = kernel.weight((source_coordinate as f32 - center) / scale);
            weights.push(weight);
            total_weight += weight;
        }

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
    // TODO(perf:micro, rank=7, after perf:kernel horizontal-unroll): Benchmark a
    // branch-light saturating round helper only after the horizontal kernel shape settles;
    // preserve image-compatible rounding exactly.
    value.clamp(0.0, 255.0).round() as u8
}
