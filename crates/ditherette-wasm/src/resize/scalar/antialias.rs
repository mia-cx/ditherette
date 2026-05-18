use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
};

pub(crate) fn antialias_rgba_box3_into(
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
            let channels = blurred_pixel(source_rgba, width, height, x, y);
            output_rgba[output_offset..output_offset + rgba::RGBA_CHANNEL_COUNT]
                .copy_from_slice(&channels);
        }
    }

    Ok(())
}

fn blurred_pixel(source_rgba: &[u8], width: usize, height: usize, x: usize, y: usize) -> [u8; 4] {
    let y_start = y.saturating_sub(1);
    let y_end = (y + 1).min(height - 1);
    let x_start = x.saturating_sub(1);
    let x_end = (x + 1).min(width - 1);
    let mut sums = [0_u32; rgba::RGBA_CHANNEL_COUNT];
    let mut count = 0_u32;

    for sample_y in y_start..=y_end {
        for sample_x in x_start..=x_end {
            let offset = rgba::pixel_byte_offset(width, sample_x, sample_y);
            sums[0] += u32::from(source_rgba[offset]);
            sums[1] += u32::from(source_rgba[offset + 1]);
            sums[2] += u32::from(source_rgba[offset + 2]);
            sums[3] += u32::from(source_rgba[offset + 3]);
            count += 1;
        }
    }

    [
        rounded_average(sums[0], count),
        rounded_average(sums[1], count),
        rounded_average(sums[2], count),
        rounded_average(sums[3], count),
    ]
}

fn rounded_average(sum: u32, count: u32) -> u8 {
    ((sum + count / 2) / count) as u8
}
