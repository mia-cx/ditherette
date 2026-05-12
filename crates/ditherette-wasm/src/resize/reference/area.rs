use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::{
        buffers::{allocate_output_rgba, validate_resize_buffers},
        scalar::area::{prepare_axis_coverages, write_area_rows, AreaRowRange},
    },
};

/// Straightforward reference implementation for exact pixel-area averaging.
#[doc(hidden)]
pub fn resize_rgba_area_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_area_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the area reference implementation.
#[doc(hidden)]
pub fn resize_rgba_area_reference_into(
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
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;
    let x_coverages = prepare_axis_coverages(source_dimensions.width(), output_dimensions.width())?;
    let y_coverages =
        prepare_axis_coverages(source_dimensions.height(), output_dimensions.height())?;

    write_area_rows(
        source_rgba,
        source_width,
        output_row_byte_len,
        &x_coverages,
        &y_coverages,
        AreaRowRange {
            output_y_start: 0,
            output_y_end: output_dimensions.height_usize()?,
        },
        output_rgba,
    );

    Ok(())
}
