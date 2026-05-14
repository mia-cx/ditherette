use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{buffers::allocate_output_rgba, scalar::lanczos as scalar_lanczos},
};

/// Resizes RGBA with a Lanczos filter.
///
/// `window_size` controls the Lanczos support radius. The common Lanczos2 and
/// Lanczos3 presets use window sizes `2.0` and `3.0` respectively. When
/// `scale_aware` is true, minification widens the filter footprint to reduce
/// aliasing.
pub fn resize_rgba_lanczos(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    window_size: f64,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_lanczos_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
        window_size,
        scale_aware,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with a Lanczos filter.
///
/// `window_size` must be finite and greater than zero. Set `scale_aware` to true
/// to widen the footprint for minification.
pub fn resize_rgba_lanczos_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    window_size: f64,
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    scalar_lanczos::resize_rgba_lanczos_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        window_size,
        scale_aware,
    )
}

#[doc(hidden)]
pub use crate::resize::reference::lanczos::{
    resize_rgba_lanczos_reference, resize_rgba_lanczos_reference_into,
};

#[cfg(test)]
mod tests {
    use super::{resize_rgba_lanczos, resize_rgba_lanczos_reference};
    use crate::{error::ProcessingError, image::ImageDimensions};

    #[test]
    fn identity_resize_returns_same_bytes_for_common_window_sizes() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 13, value * 7, value * 3, 255])
            .collect();
        let dimensions = dimensions(3, 3);

        for window_size in [2.0, 3.0] {
            for scale_aware in [false, true] {
                assert_eq!(
                    resize_rgba_lanczos(
                        &source_rgba,
                        dimensions,
                        dimensions,
                        window_size,
                        scale_aware,
                    )
                    .unwrap(),
                    source_rgba
                );
                assert_eq!(
                    resize_rgba_lanczos_reference(
                        &source_rgba,
                        dimensions,
                        dimensions,
                        window_size,
                        scale_aware,
                    )
                    .unwrap(),
                    source_rgba
                );
            }
        }
    }

    #[test]
    fn rejects_invalid_window_size() {
        let source_rgba = [0, 0, 0, 255];
        let dimensions = dimensions(1, 1);

        assert_eq!(
            resize_rgba_lanczos(&source_rgba, dimensions, dimensions, 0.0, false),
            Err(ProcessingError::InvalidParameter {
                name: "window_size",
                reason: "must be finite and greater than zero",
            })
        );
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
