use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

/// Resizes a tightly packed RGBA image with bilinear interpolation.
///
/// Bilinear sampling uses center-aligned pixel coordinates:
/// `source = (output + 0.5) * source_size / output_size - 0.5`, clamped to the
/// image edge. The implementation keeps the interpolation weights as integer
/// ratios so results are deterministic across native Rust and Wasm builds.
pub fn resize_rgba_bilinear(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_bilinear_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with bilinear interpolation.
pub fn resize_rgba_bilinear_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // TODO(perf): Add same-width and exact-2x fast paths. Same-width resize can
    // interpolate vertically with two row pointers; exact 2x can reuse the same
    // x weights for every pair of output columns.
    // TODO(perf): Precompute paired source byte offsets and fixed-point weights
    // once we have baseline bilinear timings for the Celeste fixture.
    // TODO(perf): Split edge pixels from interior pixels. Interior pixels can
    // skip clamped-sample branches/data shapes and use a tighter hot loop.
    // TODO(perf): Interpolate RGB and alpha as packed lanes or unrolled scalar
    // channels to remove the tiny per-pixel channel loop.
    // TODO(perf): Replace per-channel u128 math with a fixed-point two-pass
    // blend: horizontal interpolation into u16/u32 intermediates, then vertical.
    resize_rgba_bilinear_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

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

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let x_samples = prepare_axis_samples(source_dimensions.width(), output_dimensions.width())?;
    let y_samples = prepare_axis_samples(source_dimensions.height(), output_dimensions.height())?;

    for (output_y, y_sample) in y_samples.iter().take(output_height).enumerate() {
        for (output_x, x_sample) in x_samples.iter().take(output_width).enumerate() {
            let top_left_offset =
                rgba::pixel_byte_offset(source_width, x_sample.lower, y_sample.lower);
            let top_right_offset =
                rgba::pixel_byte_offset(source_width, x_sample.upper, y_sample.lower);
            let bottom_left_offset =
                rgba::pixel_byte_offset(source_width, x_sample.lower, y_sample.upper);
            let bottom_right_offset =
                rgba::pixel_byte_offset(source_width, x_sample.upper, y_sample.upper);
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);

            for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                output_rgba[output_offset + channel] = interpolate_channel(
                    source_rgba[top_left_offset + channel],
                    source_rgba[top_right_offset + channel],
                    source_rgba[bottom_left_offset + channel],
                    source_rgba[bottom_right_offset + channel],
                    x_sample,
                    y_sample,
                );
            }
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct AxisSample {
    lower: usize,
    upper: usize,
    weight_numerator: u64,
    denominator: u64,
}

fn prepare_axis_samples(
    source_size: u32,
    output_size: u32,
) -> Result<Vec<AxisSample>, ProcessingError> {
    let output_len = usize::try_from(output_size).map_err(|_| ProcessingError::SizeOverflow {
        context: "bilinear output axis",
    })?;
    let denominator = 2_u64 * u64::from(output_size);
    let denominator_i128 = i128::from(denominator);
    let max_source_numerator = i128::from(source_size - 1) * denominator_i128;
    let last_source_index =
        usize::try_from(source_size - 1).map_err(|_| ProcessingError::SizeOverflow {
            context: "bilinear source axis",
        })?;

    let mut samples = Vec::with_capacity(output_len);

    for output_coordinate in 0..output_len {
        let numerator = ((2_i128 * output_coordinate as i128 + 1) * i128::from(source_size))
            - i128::from(output_size);

        if numerator <= 0 {
            samples.push(AxisSample {
                lower: 0,
                upper: 0,
                weight_numerator: 0,
                denominator,
            });
            continue;
        }

        if numerator >= max_source_numerator {
            samples.push(AxisSample {
                lower: last_source_index,
                upper: last_source_index,
                weight_numerator: 0,
                denominator,
            });
            continue;
        }

        let lower = usize::try_from(numerator / denominator_i128).map_err(|_| {
            ProcessingError::SizeOverflow {
                context: "bilinear source coordinate",
            }
        })?;
        let weight_numerator = u64::try_from(numerator % denominator_i128).map_err(|_| {
            ProcessingError::SizeOverflow {
                context: "bilinear interpolation weight",
            }
        })?;

        samples.push(AxisSample {
            lower,
            upper: lower + 1,
            weight_numerator,
            denominator,
        });
    }

    Ok(samples)
}

fn interpolate_channel(
    top_left: u8,
    top_right: u8,
    bottom_left: u8,
    bottom_right: u8,
    x_sample: &AxisSample,
    y_sample: &AxisSample,
) -> u8 {
    let x_weight = u128::from(x_sample.weight_numerator);
    let y_weight = u128::from(y_sample.weight_numerator);
    let x_inverse = u128::from(x_sample.denominator) - x_weight;
    let y_inverse = u128::from(y_sample.denominator) - y_weight;

    let weighted_sum = u128::from(top_left) * x_inverse * y_inverse
        + u128::from(top_right) * x_weight * y_inverse
        + u128::from(bottom_left) * x_inverse * y_weight
        + u128::from(bottom_right) * x_weight * y_weight;
    let denominator = u128::from(x_sample.denominator) * u128::from(y_sample.denominator);

    ((weighted_sum + denominator / 2) / denominator) as u8
}

#[cfg(test)]
mod tests {
    use super::{prepare_axis_samples, resize_rgba_bilinear, resize_rgba_bilinear_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn prepares_center_aligned_samples() {
        let samples = prepare_axis_samples(2, 3).unwrap();

        assert_eq!(samples[0].lower, 0);
        assert_eq!(samples[0].upper, 0);
        assert_eq!(samples[1].lower, 0);
        assert_eq!(samples[1].upper, 1);
        assert_eq!(samples[1].weight_numerator, samples[1].denominator / 2);
        assert_eq!(samples[2].lower, 1);
        assert_eq!(samples[2].upper, 1);
    }

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 255, 255, // white
        ];

        let output_rgba =
            resize_rgba_bilinear(&source_rgba, dimensions(2, 2), dimensions(2, 2)).unwrap();
        let reference_rgba =
            resize_rgba_bilinear_reference(&source_rgba, dimensions(2, 2), dimensions(2, 2))
                .unwrap();

        assert_eq!(output_rgba, source_rgba);
        assert_eq!(reference_rgba, source_rgba);
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
