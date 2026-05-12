use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        scalar::nearest::map_output_coordinate,
        shared::pixels::copy_pixel_bytes,
    },
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
        let source_y = map_output_coordinate(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
        );

        for output_x in 0..output_width {
            let source_x = map_output_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
            );

            copy_pixel_bytes(
                source_rgba,
                rgba::pixel_byte_offset(source_width, source_x, source_y),
                output_rgba,
                rgba::pixel_byte_offset(output_width, output_x, output_y),
            );
        }
    }

    Ok(())
}
