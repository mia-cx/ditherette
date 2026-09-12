use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
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
    let output_height = output_dimensions.height_usize()?;
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    for output_y in 0..output_height {
        let y_range = SourceRange::for_output_pixel(output_y, y_scale, source_dimensions.height());

        for output_x in 0..output_width {
            let x_range =
                SourceRange::for_output_pixel(output_x, x_scale, source_dimensions.width());
            let mut weighted_sums = [0.0; rgba::RGBA_CHANNEL_COUNT];
            let mut total_weight = 0.0;

            for source_y in y_range.first..y_range.last_exclusive {
                let y_weight = y_range.overlap_with(source_y);

                for source_x in x_range.first..x_range.last_exclusive {
                    let x_weight = x_range.overlap_with(source_x);
                    let sample_weight = x_weight * y_weight;
                    let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
                    let source_pixel =
                        &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT];

                    weighted_sums[0] += f64::from(source_pixel[0]) * sample_weight;
                    weighted_sums[1] += f64::from(source_pixel[1]) * sample_weight;
                    weighted_sums[2] += f64::from(source_pixel[2]) * sample_weight;
                    weighted_sums[3] += f64::from(source_pixel[3]) * sample_weight;
                    total_weight += sample_weight;
                }
            }

            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);
            output_rgba[output_offset] = round_channel(weighted_sums[0] / total_weight);
            output_rgba[output_offset + 1] = round_channel(weighted_sums[1] / total_weight);
            output_rgba[output_offset + 2] = round_channel(weighted_sums[2] / total_weight);
            output_rgba[output_offset + 3] = round_channel(weighted_sums[3] / total_weight);
        }
    }

    Ok(())
}

#[derive(Debug, Clone, Copy)]
struct SourceRange {
    start: f64,
    end: f64,
    first: usize,
    last_exclusive: usize,
}

impl SourceRange {
    fn for_output_pixel(output_coordinate: usize, scale: f64, source_size: u32) -> Self {
        // Area resize maps each output pixel to a continuous source interval.
        // Each source pixel contributes in proportion to interval overlap.
        let start = output_coordinate as f64 * scale;
        let end = (output_coordinate + 1) as f64 * scale;
        let first = start.floor() as usize;
        let last_exclusive = (end.ceil() as usize).min(source_size as usize);

        Self {
            start,
            end,
            first,
            last_exclusive,
        }
    }

    fn overlap_with(self, source_coordinate: usize) -> f64 {
        let source_start = source_coordinate as f64;
        let source_end = source_start + 1.0;

        (self.end.min(source_end) - self.start.max(source_start)).max(0.0)
    }
}

fn round_channel(value: f64) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}
