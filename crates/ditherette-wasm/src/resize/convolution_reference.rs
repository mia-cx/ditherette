use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        convolution::Kernel,
    },
};

/// Allocates and resizes with the straightforward separable convolution path.
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

/// Straightforward reference implementation for separable convolution filters.
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
    let output_width = output_dimensions.width_usize()?;
    let x_contributions = prepare_axis_contributions(
        source_dimensions.width(),
        output_dimensions.width(),
        kernel,
        scale_aware,
    )?;
    let y_contributions = prepare_axis_contributions(
        source_dimensions.height(),
        output_dimensions.height(),
        kernel,
        scale_aware,
    )?;

    for (output_y, y_contribution) in y_contributions.iter().enumerate() {
        for (output_x, x_contribution) in x_contributions.iter().enumerate() {
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);

            for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                output_rgba[output_offset + channel] = convolution_channel(
                    source_rgba,
                    source_width,
                    x_contribution,
                    y_contribution,
                    channel,
                );
            }
        }
    }

    Ok(())
}

#[derive(Debug)]
struct AxisContributions {
    samples: Vec<WeightedSourceIndex>,
}

#[derive(Debug)]
struct WeightedSourceIndex {
    index: usize,
    weight: f64,
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
    let scale = f64::from(output_size) / f64::from(source_size);
    let filter_scale = if scale_aware && scale < 1.0 {
        scale
    } else {
        1.0
    };
    let radius = kernel.radius() / filter_scale;
    let mut contributions = Vec::with_capacity(output_len);

    for output_coordinate in 0..output_len {
        let center = (output_coordinate as f64 + 0.5) / scale - 0.5;
        let first_source = (center - radius).floor() as isize;
        let last_source = (center + radius).ceil() as isize;
        let mut samples = Vec::new();
        let mut total_weight = 0.0;

        for source_coordinate in first_source..=last_source {
            let clamped_source = source_coordinate.clamp(0, source_len as isize - 1) as usize;
            let weight = kernel.weight((center - source_coordinate as f64) * filter_scale);

            if weight.abs() <= f64::EPSILON {
                continue;
            }

            samples.push(WeightedSourceIndex {
                index: clamped_source,
                weight,
            });
            total_weight += weight;
        }

        if total_weight.abs() > f64::EPSILON {
            for sample in &mut samples {
                sample.weight /= total_weight;
            }
        }

        contributions.push(AxisContributions { samples });
    }

    Ok(contributions)
}

fn convolution_channel(
    source_rgba: &[u8],
    source_width: usize,
    x_contribution: &AxisContributions,
    y_contribution: &AxisContributions,
    channel: usize,
) -> u8 {
    let mut value = 0.0;

    for y_sample in &y_contribution.samples {
        for x_sample in &x_contribution.samples {
            let source_offset =
                rgba::pixel_byte_offset(source_width, x_sample.index, y_sample.index);
            value +=
                f64::from(source_rgba[source_offset + channel]) * x_sample.weight * y_sample.weight;
        }
    }

    value.round().clamp(0.0, 255.0) as u8
}
