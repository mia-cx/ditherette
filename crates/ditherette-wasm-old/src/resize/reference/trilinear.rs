use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        reference::{
            area::resize_rgba_area_reference, bilinear::resize_rgba_bilinear_reference_into,
        },
    },
};

/// Straightforward reference implementation for mipmapped trilinear filtering.
#[doc(hidden)]
pub fn resize_rgba_trilinear_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_trilinear_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the trilinear reference implementation.
#[doc(hidden)]
pub fn resize_rgba_trilinear_reference_into(
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

    let minification = minification_factor(source_dimensions, output_dimensions);
    if minification <= 1.0 {
        return resize_rgba_bilinear_reference_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        );
    }

    let lod = minification.log2();
    let lower_lod = lod.floor() as usize;
    let upper_lod = lod.ceil() as usize;
    let blend = lod - lower_lod as f64;
    let lower_level = mip_level(source_rgba, source_dimensions, lower_lod)?;

    if lower_lod == upper_lod
        || lower_level.dimensions.width() == 1 && lower_level.dimensions.height() == 1
    {
        return resize_rgba_bilinear_reference_into(
            &lower_level.rgba,
            lower_level.dimensions,
            output_dimensions,
            output_rgba,
        );
    }

    let upper_level = mip_level(source_rgba, source_dimensions, upper_lod)?;
    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions)?;
    let mut lower_rgba = vec![0; output_byte_len];
    let mut upper_rgba = vec![0; output_byte_len];

    resize_rgba_bilinear_reference_into(
        &lower_level.rgba,
        lower_level.dimensions,
        output_dimensions,
        &mut lower_rgba,
    )?;
    resize_rgba_bilinear_reference_into(
        &upper_level.rgba,
        upper_level.dimensions,
        output_dimensions,
        &mut upper_rgba,
    )?;

    for (output, (lower, upper)) in output_rgba
        .iter_mut()
        .zip(lower_rgba.iter().zip(&upper_rgba))
    {
        *output = (f64::from(*lower) * (1.0 - blend) + f64::from(*upper) * blend)
            .round()
            .clamp(0.0, 255.0) as u8;
    }

    Ok(())
}

#[derive(Debug)]
struct RgbaLevel {
    dimensions: ImageDimensions,
    rgba: Vec<u8>,
}

fn minification_factor(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> f64 {
    let x_factor = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_factor = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    x_factor.max(y_factor)
}

fn mip_level(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    target_level: usize,
) -> Result<RgbaLevel, ProcessingError> {
    let mut current_rgba = source_rgba.to_vec();
    let mut current_dimensions = source_dimensions;

    for _ in 0..target_level {
        if current_dimensions.width() == 1 && current_dimensions.height() == 1 {
            break;
        }

        let next_dimensions = ImageDimensions::new(
            half_rounded_up(current_dimensions.width()),
            half_rounded_up(current_dimensions.height()),
        )?;
        let next_rgba =
            resize_rgba_area_reference(&current_rgba, current_dimensions, next_dimensions)?;

        current_rgba = next_rgba;
        current_dimensions = next_dimensions;
    }

    Ok(RgbaLevel {
        dimensions: current_dimensions,
        rgba: current_rgba,
    })
}

fn half_rounded_up(value: u32) -> u32 {
    value.div_ceil(2).max(1)
}

#[cfg(test)]
mod tests {
    use super::minification_factor;
    use crate::image::ImageDimensions;

    #[test]
    fn computes_largest_axis_minification() {
        assert_eq!(minification_factor(dimensions(8, 4), dimensions(2, 4)), 4.0);
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
