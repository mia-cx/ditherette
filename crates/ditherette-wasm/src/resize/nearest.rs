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
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_nearest_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with the production nearest path.
///
/// After validating the boundary buffers once, this uses the fastest measured
/// strategy: identity and row-copy fast paths, exact integer downscale, then a
/// precomputed-offset fallback that copies each RGBA pixel as one unaligned
/// 32-bit word. The sampling rule remains the deterministic integer mapping
/// documented on [`resize_rgba_nearest`].
pub fn resize_rgba_nearest_into(
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

    if source_dimensions.width() == output_dimensions.width() {
        copy_same_width_rows(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    if is_exact_integer_downscale(source_dimensions, output_dimensions) {
        resize_exact_integer_downscale_word(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    // TODO(perf): Revisit reusable offset buffers, unchecked offset helpers,
    // row tiling, and SIMD once broader resize/dither kernels are in place.
    resize_precomputed_offsets_word_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

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

/// Straightforward reference implementation into a caller-provided output buffer.
#[doc(hidden)]
pub fn resize_rgba_nearest_reference_into(
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

/// Resizes by precomputing source byte offsets for each output row and column.
///
/// This variant tests whether removing coordinate division from the hot inner
/// loop beats the extra allocation and indirection cost.
pub fn resize_rgba_nearest_precomputed_offsets(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;

    let source_x_byte_offsets: Vec<usize> = (0..output_width)
        .map(|output_x| {
            let source_x = map_output_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
            );
            source_x * rgba::RGBA_CHANNEL_COUNT
        })
        .collect();

    let source_row_byte_offsets: Vec<usize> = (0..output_height)
        .map(|output_y| {
            let source_y = map_output_coordinate(
                output_y,
                source_dimensions.height(),
                output_dimensions.height(),
            );
            source_y * source_width * rgba::RGBA_CHANNEL_COUNT
        })
        .collect();

    for (output_y, source_row_offset) in source_row_byte_offsets.into_iter().enumerate() {
        for (output_x, source_x_offset) in source_x_byte_offsets.iter().copied().enumerate() {
            copy_pixel_bytes(
                source_rgba,
                source_row_offset + source_x_offset,
                &mut output_rgba,
                rgba::pixel_byte_offset(output_width, output_x, output_y),
            );
        }
    }

    Ok(output_rgba)
}

/// Resizes with simple fast paths before falling back to the baseline loop.
///
/// The fast paths cover identity resizes, same-width vertical-only resizes, and
/// exact integer downscales. They are separated for benchmarking because each
/// branch adds code complexity that must earn its place.
pub fn resize_rgba_nearest_fast_paths(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;

    if source_dimensions == output_dimensions {
        return Ok(source_rgba.to_vec());
    }

    let mut output_rgba = vec![0; rgba::checked_rgba_byte_len(output_dimensions)?];

    if source_dimensions.width() == output_dimensions.width() {
        copy_same_width_rows(
            source_rgba,
            source_dimensions,
            output_dimensions,
            &mut output_rgba,
        )?;
        return Ok(output_rgba);
    }

    if is_exact_integer_downscale(source_dimensions, output_dimensions) {
        resize_exact_integer_downscale(
            source_rgba,
            source_dimensions,
            output_dimensions,
            &mut output_rgba,
        )?;
        return Ok(output_rgba);
    }

    resize_rgba_nearest_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes by copying each RGBA pixel as one unaligned `u32` word.
///
/// This variant checks whether replacing four-byte slice copies with one word
/// load/store improves the hot loop. It copies bytes without interpreting
/// channel order, so host endianness does not affect output.
pub fn resize_rgba_nearest_word_copy(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;

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

            copy_pixel_word(
                source_rgba,
                rgba::pixel_byte_offset(source_width, source_x, source_y),
                &mut output_rgba,
                rgba::pixel_byte_offset(output_width, output_x, output_y),
            );
        }
    }

    Ok(output_rgba)
}

fn allocate_output_rgba(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;
    Ok(vec![0; rgba::checked_rgba_byte_len(output_dimensions)?])
}

fn validate_resize_buffers(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &[u8],
) -> Result<(), ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, source_dimensions)?;

    let expected = rgba::checked_rgba_byte_len(output_dimensions)?;
    let actual = output_rgba.len();

    if actual != expected {
        return Err(ProcessingError::InvalidBufferLength { expected, actual });
    }

    Ok(())
}

fn copy_same_width_rows(
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

    let row_byte_len = source_dimensions.width_usize()? * rgba::RGBA_CHANNEL_COUNT;
    let output_height = output_dimensions.height_usize()?;

    for output_y in 0..output_height {
        let source_y = map_output_coordinate(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
        );
        let source_offset = source_y * row_byte_len;
        let output_offset = output_y * row_byte_len;

        output_rgba[output_offset..output_offset + row_byte_len]
            .copy_from_slice(&source_rgba[source_offset..source_offset + row_byte_len]);
    }

    Ok(())
}

fn resize_exact_integer_downscale(
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
    let x_step = source_dimensions.width_usize()? / output_width;
    let y_step = source_dimensions.height_usize()? / output_height;

    for output_y in 0..output_height {
        let source_y = output_y * y_step;

        for output_x in 0..output_width {
            let source_x = output_x * x_step;

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

fn resize_exact_integer_downscale_word(
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
    let x_step = source_dimensions.width_usize()? / output_width;
    let y_step = source_dimensions.height_usize()? / output_height;

    for output_y in 0..output_height {
        let source_y = output_y * y_step;

        for output_x in 0..output_width {
            let source_x = output_x * x_step;

            copy_pixel_word(
                source_rgba,
                rgba::pixel_byte_offset(source_width, source_x, source_y),
                output_rgba,
                rgba::pixel_byte_offset(output_width, output_x, output_y),
            );
        }
    }

    Ok(())
}

fn resize_precomputed_offsets_word_into(
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

    let source_x_byte_offsets: Vec<usize> = (0..output_width)
        .map(|output_x| {
            let source_x = map_output_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
            );
            source_x * rgba::RGBA_CHANNEL_COUNT
        })
        .collect();

    let source_row_byte_offsets: Vec<usize> = (0..output_height)
        .map(|output_y| {
            let source_y = map_output_coordinate(
                output_y,
                source_dimensions.height(),
                output_dimensions.height(),
            );
            source_y * source_width * rgba::RGBA_CHANNEL_COUNT
        })
        .collect();

    for (output_y, source_row_offset) in source_row_byte_offsets.into_iter().enumerate() {
        for (output_x, source_x_offset) in source_x_byte_offsets.iter().copied().enumerate() {
            copy_pixel_word(
                source_rgba,
                source_row_offset + source_x_offset,
                output_rgba,
                rgba::pixel_byte_offset(output_width, output_x, output_y),
            );
        }
    }

    Ok(())
}

fn is_exact_integer_downscale(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    source_dimensions.width() > output_dimensions.width()
        && source_dimensions.height() > output_dimensions.height()
        && source_dimensions
            .width()
            .is_multiple_of(output_dimensions.width())
        && source_dimensions
            .height()
            .is_multiple_of(output_dimensions.height())
}

fn copy_pixel_bytes(
    source_rgba: &[u8],
    source_offset: usize,
    output_rgba: &mut [u8],
    output_offset: usize,
) {
    output_rgba[output_offset..output_offset + rgba::RGBA_CHANNEL_COUNT]
        .copy_from_slice(&source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT]);
}

fn copy_pixel_word(
    source_rgba: &[u8],
    source_offset: usize,
    output_rgba: &mut [u8],
    output_offset: usize,
) {
    // SAFETY: Callers pass offsets produced from validated image dimensions and
    // in-bounds loop coordinates. `read_unaligned` and `write_unaligned` avoid
    // alignment requirements for byte-backed Wasm memory.
    unsafe {
        let source_ptr = source_rgba.as_ptr().add(source_offset).cast::<u32>();
        let output_ptr = output_rgba.as_mut_ptr().add(output_offset).cast::<u32>();
        output_ptr.write_unaligned(source_ptr.read_unaligned());
    }
}

fn map_output_coordinate(output_coordinate: usize, source_size: u32, output_size: u32) -> usize {
    ((output_coordinate as u64 * u64::from(source_size)) / u64::from(output_size)) as usize
}

#[cfg(test)]
mod tests {
    use super::{
        is_exact_integer_downscale, map_output_coordinate, resize_rgba_nearest,
        resize_rgba_nearest_fast_paths, resize_rgba_nearest_into,
        resize_rgba_nearest_precomputed_offsets, resize_rgba_nearest_reference,
        resize_rgba_nearest_reference_into, resize_rgba_nearest_word_copy,
    };
    use crate::image::ImageDimensions;

    #[test]
    fn maps_coordinates_with_integer_proportions() {
        assert_eq!(map_output_coordinate(0, 4, 2), 0);
        assert_eq!(map_output_coordinate(1, 4, 2), 2);
        assert_eq!(map_output_coordinate(3, 2, 4), 1);
    }

    #[test]
    fn detects_exact_integer_downscale() {
        assert!(is_exact_integer_downscale(
            dimensions(4, 4),
            dimensions(2, 2)
        ));
        assert!(!is_exact_integer_downscale(
            dimensions(4, 4),
            dimensions(3, 3)
        ));
        assert!(!is_exact_integer_downscale(
            dimensions(2, 2),
            dimensions(4, 4)
        ));
    }

    #[test]
    fn benchmark_variants_match_baseline_across_resize_shapes() {
        for (source_dimensions, output_dimensions) in [
            (dimensions(6, 4), dimensions(3, 2)),
            (dimensions(7, 5), dimensions(5, 3)),
            (dimensions(3, 2), dimensions(7, 5)),
            (dimensions(4, 5), dimensions(4, 3)),
            (dimensions(4, 3), dimensions(4, 3)),
            (dimensions(1, 1), dimensions(5, 4)),
        ] {
            assert_variants_match_baseline(source_dimensions, output_dimensions);
        }
    }

    fn assert_variants_match_baseline(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
    ) {
        let source_rgba = patterned_rgba(source_dimensions);
        let baseline =
            resize_rgba_nearest_reference(&source_rgba, source_dimensions, output_dimensions)
                .unwrap();

        assert_eq!(
            resize_rgba_nearest(&source_rgba, source_dimensions, output_dimensions).unwrap(),
            baseline
        );
        assert_eq!(
            resize_rgba_nearest_precomputed_offsets(
                &source_rgba,
                source_dimensions,
                output_dimensions
            )
            .unwrap(),
            baseline
        );
        assert_eq!(
            resize_rgba_nearest_fast_paths(&source_rgba, source_dimensions, output_dimensions)
                .unwrap(),
            baseline
        );
        assert_eq!(
            resize_rgba_nearest_word_copy(&source_rgba, source_dimensions, output_dimensions)
                .unwrap(),
            baseline
        );
        let mut output_rgba = vec![0xA5; baseline.len()];
        resize_rgba_nearest_reference_into(
            &source_rgba,
            source_dimensions,
            output_dimensions,
            &mut output_rgba,
        )
        .unwrap();
        assert_eq!(output_rgba, baseline);

        let mut output_rgba = vec![0xA5; baseline.len()];
        resize_rgba_nearest_into(
            &source_rgba,
            source_dimensions,
            output_dimensions,
            &mut output_rgba,
        )
        .unwrap();
        assert_eq!(output_rgba, baseline);
    }

    fn patterned_rgba(dimensions: ImageDimensions) -> Vec<u8> {
        let pixel_count = dimensions.width() as usize * dimensions.height() as usize;

        (0..pixel_count)
            .flat_map(|index| {
                let value = index as u8;
                [value, value.wrapping_mul(3), value.wrapping_mul(5), 255]
            })
            .collect()
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
