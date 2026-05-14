use crate::{
    error::ProcessingError, image::ImageDimensions, resize::buffers::allocate_output_rgba,
};

/// Resizes RGBA with exact pixel-area averaging.
///
/// This is the practical "box" downsampling filter: every output pixel covers a
/// rectangle in source-pixel space, and each covered source pixel contributes by
/// its overlap area. It is a strong reference implementation for antialiased
/// minification, though it can look blocky for upscaling.
pub fn resize_rgba_area(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_area_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with exact area averaging.
pub fn resize_rgba_area_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    #[cfg(feature = "tiling")]
    {
        crate::resize::tiling::area::resize_rgba_area_with_tiling_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
            crate::resize::tiling::area::AREA_ROW_BAND_TILING,
        )
    }

    #[cfg(not(feature = "tiling"))]
    {
        crate::resize::scalar::area::resize_rgba_area_scalar_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )
    }
}

#[doc(hidden)]
pub use crate::resize::reference::area::{
    resize_rgba_area_reference, resize_rgba_area_reference_into,
};

#[doc(hidden)]
pub use crate::resize::scalar::area::resize_rgba_area_scalar_into;

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_area_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    crate::resize::tiling::area::resize_rgba_area_with_tiling_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        crate::resize::tiling::area::AREA_ROW_BAND_TILING,
    )
}
