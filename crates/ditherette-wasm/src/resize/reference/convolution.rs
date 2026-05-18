use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        shared::convolution::Kernel,
    },
};

/// Allocates and resizes with the straightforward image-compatible separable convolution path.
pub(crate) fn resize_with_convolution_reference<K: Kernel>(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    kernel: K,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_with_convolution_reference_into(
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
pub(crate) fn resize_with_convolution_reference_into<K: Kernel>(
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

#[derive(Debug)]
struct AxisContributions {
    first: usize,
    weights: Vec<f32>,
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
    value.clamp(0.0, 255.0).round() as u8
}
