use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
};

/// Straightforward reference implementation for the 3x3 box antialiasing pass.
#[doc(hidden)]
pub fn antialias_rgba_box3_reference(
    source_rgba: &[u8],
    dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, dimensions)?;
    let mut output_rgba = vec![0; rgba::checked_rgba_byte_len(dimensions)?];
    antialias_rgba_box3_reference_into(source_rgba, dimensions, &mut output_rgba)?;
    Ok(output_rgba)
}

/// Allocation-free form of the 3x3 box antialiasing reference implementation.
#[doc(hidden)]
pub fn antialias_rgba_box3_reference_into(
    source_rgba: &[u8],
    dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, dimensions)?;

    let expected = rgba::checked_rgba_byte_len(dimensions)?;
    let actual = output_rgba.len();
    if actual != expected {
        return Err(ProcessingError::InvalidBufferLength { expected, actual });
    }

    let width = dimensions.width_usize()?;
    let height = dimensions.height_usize()?;

    for y in 0..height {
        for x in 0..width {
            let output_offset = rgba::pixel_byte_offset(width, x, y);

            for channel in 0..rgba::RGBA_CHANNEL_COUNT {
                output_rgba[output_offset + channel] =
                    blurred_channel(source_rgba, width, height, x, y, channel);
            }
        }
    }

    Ok(())
}

fn blurred_channel(
    source_rgba: &[u8],
    width: usize,
    height: usize,
    x: usize,
    y: usize,
    channel: usize,
) -> u8 {
    let y_start = y.saturating_sub(1);
    let y_end = (y + 1).min(height - 1);
    let x_start = x.saturating_sub(1);
    let x_end = (x + 1).min(width - 1);
    let mut sum = 0_u32;
    let mut count = 0_u32;

    for sample_y in y_start..=y_end {
        for sample_x in x_start..=x_end {
            let offset = rgba::pixel_byte_offset(width, sample_x, sample_y);
            sum += u32::from(source_rgba[offset + channel]);
            count += 1;
        }
    }

    ((sum + count / 2) / count) as u8
}
