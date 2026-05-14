use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

/// Straightforward reference implementation used by tests and benchmarks.
#[doc(hidden)]
pub fn resize_rgba_nearest_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_nearest_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the straightforward reference implementation.
#[doc(hidden)]
pub fn resize_rgba_nearest_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    write_reference_resize(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

pub(crate) fn write_reference_resize(
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

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;

    for output_y in 0..output_height {
        let source_y = map_output_coordinate_reference(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
        );

        for output_x in 0..output_width {
            let source_x = map_output_coordinate_reference(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
            );
            let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);

            output_rgba[output_offset..output_offset + rgba::RGBA_CHANNEL_COUNT].copy_from_slice(
                &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT],
            );
        }
    }

    Ok(())
}

fn map_output_coordinate_reference(
    output_coordinate: usize,
    source_size: u32,
    output_size: u32,
) -> usize {
    // Map the center of the output pixel into source-pixel coordinates, then
    // choose the source pixel containing that mapped center. Algebraically this
    // is floor(((output + 0.5) * source_size) / output_size).
    let mapped = (((2 * output_coordinate as u64 + 1) * u64::from(source_size))
        / (2 * u64::from(output_size))) as usize;

    mapped.min(source_size as usize - 1)
}
