use crate::{
    error::ProcessingError, image::ImageDimensions, resize::buffers::allocate_output_rgba,
};

/// Resizes RGBA with mipmapped trilinear interpolation.
///
/// Trilinear filtering is primarily a texture-sampling technique: build a
/// mipmap pyramid, bilinearly sample the two mip levels nearest the requested
/// minification, then blend between those levels. It is included for comparison,
/// but area or scale-aware Lanczos are more direct choices for one-shot CPU image
/// resizing.
// NOTE(perf): Keep this wrapper simple; identity/upscale fast paths and
// caller-owned mip pyramids belong in the scalar implementation or a future
// resize-plan API, not the public allocation wrapper.
pub fn resize_rgba_trilinear(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_trilinear_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with mipmapped trilinear filtering.
pub fn resize_rgba_trilinear_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // NOTE(perf): Trilinear remains a comparison filter. The obvious production
    // optimizations (mip cache, partial pyramid, fused bilinear/LOD sampling,
    // fixed-point blend, exact power-of-two path, anisotropic LOD, and compact
    // pyramid storage) should be designed together if trilinear becomes a real
    // product path; piecemeal wrapper TODOs are not actionable.
    crate::resize::scalar::trilinear::resize_rgba_trilinear_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

#[doc(hidden)]
pub use crate::resize::reference::trilinear::{
    resize_rgba_trilinear_reference, resize_rgba_trilinear_reference_into,
};

#[cfg(test)]
mod tests {
    use super::{resize_rgba_trilinear, resize_rgba_trilinear_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba: Vec<u8> = (0..4)
            .flat_map(|value| [value * 40, value * 20, value * 10, 255])
            .collect();
        let dimensions = dimensions(2, 2);

        assert_eq!(
            resize_rgba_trilinear(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
        assert_eq!(
            resize_rgba_trilinear_reference(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
    }

    #[test]
    fn minifying_resize_matches_reference() {
        let source_rgba: Vec<u8> = (0..8 * 6 * 4)
            .map(|value| (value * 31 % 251) as u8)
            .collect();

        let output_rgba =
            resize_rgba_trilinear(&source_rgba, dimensions(8, 6), dimensions(3, 2)).unwrap();
        let reference_rgba =
            resize_rgba_trilinear_reference(&source_rgba, dimensions(8, 6), dimensions(3, 2))
                .unwrap();

        assert_eq!(output_rgba, reference_rgba);
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
