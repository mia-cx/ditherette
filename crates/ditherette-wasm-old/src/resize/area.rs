//! Exact pixel-area resampling.
//!
//! This implementation is optimized where it can be without changing results,
//! but it is intentionally byte-for-byte aligned with the independent area
//! reference. That exactness prevents the broad fast paths that normally make a
//! box/area filter cheap, such as separable passes, prefix sums, integral images,
//! f32 accumulation, or reordered accumulation: those change rounding or have
//! benchmarked slower in `pnpm bench:resize:area`. If area must beat bilinear
//! across fractional scales, add a separate approximate/fast-area mode with its
//! own documented rounding contract instead of weakening this exact filter.

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
        crate::resize::tiling::area::resize_rgba_area_with_dynamic_tiling_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
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
pub fn resize_rgba_area_dynamic_tiling_plan(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Option<crate::resize::cpu_tiling::RowBandPlan>, ProcessingError> {
    crate::resize::tiling::area::dynamic_area_tiling_plan(source_dimensions, output_dimensions)
}

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_area_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    crate::resize::tiling::area::resize_rgba_area_with_dynamic_tiling_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_area_with_row_band_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling: crate::resize::cpu_tiling::RowBandTiling,
) -> Result<(), ProcessingError> {
    crate::resize::tiling::area::resize_rgba_area_with_tiling_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        tiling,
    )
}
