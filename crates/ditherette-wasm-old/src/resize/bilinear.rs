use crate::{
    error::ProcessingError, image::ImageDimensions, resize::buffers::allocate_output_rgba,
};

/// Resizes a tightly packed RGBA image with bilinear interpolation.
///
/// Bilinear sampling uses center-aligned pixel coordinates:
/// `source = (output + 0.5) * source_size / output_size - 0.5`, clamped to the
/// image edge. The implementation keeps the interpolation weights as integer
/// ratios so results are deterministic across native Rust and Wasm builds.
pub fn resize_rgba_bilinear(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_bilinear_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with bilinear interpolation.
pub fn resize_rgba_bilinear_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    #[cfg(feature = "tiling")]
    {
        crate::resize::tiling::bilinear::resize_rgba_bilinear_with_dynamic_tiling_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )
    }

    #[cfg(not(feature = "tiling"))]
    {
        crate::resize::scalar::bilinear::resize_rgba_bilinear_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )
    }
}

#[doc(hidden)]
pub use crate::resize::reference::bilinear::{
    resize_rgba_bilinear_reference, resize_rgba_bilinear_reference_into,
};

#[doc(hidden)]
pub fn resize_rgba_bilinear_scalar_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    crate::resize::scalar::bilinear::resize_rgba_bilinear_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

#[doc(hidden)]
pub use crate::resize::scalar::bilinear_2::{resize_rgba_bilinear_2, resize_rgba_bilinear_2_into};

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_bilinear_dynamic_tiling_plan(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Option<crate::resize::cpu_tiling::RowBandPlan>, ProcessingError> {
    crate::resize::tiling::bilinear::dynamic_bilinear_tiling_plan(
        source_dimensions,
        output_dimensions,
    )
}

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_bilinear_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    crate::resize::tiling::bilinear::resize_rgba_bilinear_with_dynamic_tiling_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_bilinear_with_row_band_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling: crate::resize::cpu_tiling::RowBandTiling,
) -> Result<(), ProcessingError> {
    crate::resize::tiling::bilinear::resize_rgba_bilinear_with_tiling_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        tiling,
    )
}

#[cfg(test)]
mod tests {
    use super::{resize_rgba_bilinear, resize_rgba_bilinear_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn non_uniform_resize_matches_reference() {
        let source_rgba: Vec<u8> = (0..5 * 4 * 4)
            .map(|value| (value * 29 % 253) as u8)
            .collect();

        let output_rgba =
            resize_rgba_bilinear(&source_rgba, dimensions(5, 4), dimensions(3, 7)).unwrap();
        let reference_rgba =
            resize_rgba_bilinear_reference(&source_rgba, dimensions(5, 4), dimensions(3, 7))
                .unwrap();

        assert_eq!(output_rgba, reference_rgba);
    }

    #[test]
    fn same_width_resize_matches_reference() {
        let source_rgba: Vec<u8> = (0..4 * 5 * 4)
            .map(|value| (value * 17 % 251) as u8)
            .collect();

        let output_rgba =
            resize_rgba_bilinear(&source_rgba, dimensions(4, 5), dimensions(4, 3)).unwrap();
        let reference_rgba =
            resize_rgba_bilinear_reference(&source_rgba, dimensions(4, 5), dimensions(4, 3))
                .unwrap();

        assert_eq!(output_rgba, reference_rgba);
    }

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba = vec![
            255, 0, 0, 255, // red
            0, 255, 0, 255, // green
            0, 0, 255, 255, // blue
            255, 255, 255, 255, // white
        ];

        let output_rgba =
            resize_rgba_bilinear(&source_rgba, dimensions(2, 2), dimensions(2, 2)).unwrap();
        let reference_rgba =
            resize_rgba_bilinear_reference(&source_rgba, dimensions(2, 2), dimensions(2, 2))
                .unwrap();

        assert_eq!(output_rgba, source_rgba);
        assert_eq!(reference_rgba, source_rgba);
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
