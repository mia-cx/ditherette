use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        buffers::allocate_output_rgba, lanczos::resize_rgba_lanczos_into,
        lanczos2::LANCZOS2_WINDOW_SIZE,
    },
};

/// Resizes RGBA with Lanczos2 widened for minification.
///
/// Scale-aware convolution expands the source footprint when downsampling so the
/// filter acts as an antialiasing low-pass instead of sampling a fixed small
/// neighborhood.
pub fn resize_rgba_lanczos2_scale_aware(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_lanczos2_scale_aware_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with scale-aware Lanczos2.
pub fn resize_rgba_lanczos2_scale_aware_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        LANCZOS2_WINDOW_SIZE,
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::{resize_rgba_lanczos2_scale_aware, LANCZOS2_WINDOW_SIZE};
    use crate::{image::ImageDimensions, resize::lanczos::resize_rgba_lanczos_reference};

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 13, value * 7, value * 3, 255])
            .collect();
        let dimensions = dimensions(3, 3);

        assert_eq!(
            resize_rgba_lanczos2_scale_aware(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
        assert_eq!(
            resize_rgba_lanczos_reference(
                &source_rgba,
                dimensions,
                dimensions,
                LANCZOS2_WINDOW_SIZE,
                true,
            )
            .unwrap(),
            source_rgba
        );
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
