use crate::{
    error::ProcessingError, image::ImageDimensions, resize::buffers::allocate_output_rgba,
};

/// Resizes RGBA with a Catmull-Rom bicubic filter.
pub fn resize_rgba_bicubic(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_bicubic_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with a Catmull-Rom bicubic filter.
pub fn resize_rgba_bicubic_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // TODO(perf): Add a bicubic-specific separable implementation once benchmarks
    // show this filter matters. The generic convolution path is readable, but it
    // cannot exploit the fixed 4-tap Catmull-Rom footprint.
    // TODO(perf): Precompute four source indices and four fixed-point weights per
    // output coordinate. Bicubic has a constant support radius, so all per-pixel
    // dynamic contribution assembly can move out of the hot loop.
    crate::resize::scalar::bicubic::resize_rgba_bicubic_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

#[doc(hidden)]
pub use crate::resize::reference::bicubic::{
    resize_rgba_bicubic_reference, resize_rgba_bicubic_reference_into,
};

#[cfg(test)]
mod tests {
    use super::{resize_rgba_bicubic, resize_rgba_bicubic_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 13, value * 7, value * 3, 255])
            .collect();
        let dimensions = dimensions(3, 3);

        let output_rgba = resize_rgba_bicubic(&source_rgba, dimensions, dimensions).unwrap();
        let reference_rgba =
            resize_rgba_bicubic_reference(&source_rgba, dimensions, dimensions).unwrap();

        assert_eq!(output_rgba, source_rgba);
        assert_eq!(reference_rgba, source_rgba);
        assert_eq!(output_rgba, reference_rgba);
    }

    #[test]
    fn non_uniform_resize_matches_reference() {
        let source_rgba: Vec<u8> = (0..5 * 4 * 4)
            .map(|value| (value * 37 % 251) as u8)
            .collect();

        let output_rgba =
            resize_rgba_bicubic(&source_rgba, dimensions(5, 4), dimensions(3, 7)).unwrap();
        let reference_rgba =
            resize_rgba_bicubic_reference(&source_rgba, dimensions(5, 4), dimensions(3, 7))
                .unwrap();

        assert_eq!(output_rgba, reference_rgba);
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
