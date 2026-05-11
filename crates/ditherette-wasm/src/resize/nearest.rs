use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
};

/// Resizes a tightly packed RGBA image with nearest-neighbor sampling.
///
/// The mapping is integer-proportional and intentionally simple:
/// `source_x = output_x * source_width / output_width`, with the same rule for
/// y. This avoids floating-point rounding differences at the Wasm boundary and
/// gives deterministic behavior that is straightforward to test.
pub fn resize_rgba_nearest(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;

    let output_byte_len = rgba::checked_rgba_byte_len(output_dimensions)?;
    let mut output_rgba = vec![0; output_byte_len];

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;

    // TODO(perf): Benchmark before optimizing. Possible future paths include
    // precomputed source-x/source-y offsets, identity and same-width row copies,
    // exact integer-scale fast paths, u32 word-copy pixels, unsafe unchecked
    // indexing after validation, tiled/chunked rows, SIMD/vectorized copies, and
    // reusable output buffers.
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

            let source_offset = rgba::pixel_byte_offset(source_width, source_x, source_y);
            let output_offset = rgba::pixel_byte_offset(output_width, output_x, output_y);

            output_rgba[output_offset..output_offset + rgba::RGBA_CHANNEL_COUNT].copy_from_slice(
                &source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT],
            );
        }
    }

    Ok(output_rgba)
}

fn map_output_coordinate(output_coordinate: usize, source_size: u32, output_size: u32) -> usize {
    ((output_coordinate as u64 * u64::from(source_size)) / u64::from(output_size)) as usize
}

#[cfg(test)]
mod tests {
    use super::map_output_coordinate;

    #[test]
    fn maps_coordinates_with_integer_proportions() {
        assert_eq!(map_output_coordinate(0, 4, 2), 0);
        assert_eq!(map_output_coordinate(1, 4, 2), 2);
        assert_eq!(map_output_coordinate(3, 2, 4), 1);
    }
}
