use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

/// Straightforward reference implementation for bilinear resize.
#[doc(hidden)]
pub fn resize_rgba_bilinear_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_bilinear_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the bilinear reference implementation.
#[doc(hidden)]
pub fn resize_rgba_bilinear_reference_into(
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
    let mut vertical_rgba = vec![0.0; output_height * source_width * rgba::RGBA_CHANNEL_COUNT];

    for (output_y, contribution) in y_contributions.iter().enumerate() {
        for source_x in 0..source_width {
            let vertical_offset = rgba::pixel_byte_offset(source_width, source_x, output_y);

            for (weight_index, weight) in contribution.weights.iter().enumerate() {
                let source_y = contribution.first + weight_index;
                let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);

                for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                    vertical_rgba[vertical_offset + channel] +=
                        f32::from(source_rgba[source_offset + channel]) * weight;
                }
            }
        }
    }

    for output_y in 0..output_height {
        for (output_x, contribution) in x_contributions.iter().enumerate() {
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);
            let mut accumulated = [0.0; rgba::RGBA_CHANNEL_COUNT];

            for (weight_index, weight) in contribution.weights.iter().enumerate() {
                let source_x = contribution.first + weight_index;
                let vertical_offset = rgba::pixel_byte_offset(source_width, source_x, output_y);

                for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                    accumulated[channel] += vertical_rgba[vertical_offset + channel] * weight;
                }
            }

            for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                output_rgba[output_offset + channel] = round_u8(accumulated[channel]);
            }
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

        for source_coordinate in left..right {
            let weight = triangle_weight((source_coordinate as f32 - center) / scale);
            weights.push(weight);
            sum += weight;
        }

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

#[derive(Debug, Clone)]
struct AxisContribution {
    first: usize,
    weights: Vec<f32>,
}
