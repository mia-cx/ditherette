use crate::{
    error::ProcessingError, image::ImageDimensions, resize::buffers::allocate_output_rgba,
};

/// Resizes a tightly packed RGBA image with nearest-neighbor sampling.
///
/// The mapping is pixel-center aligned:
/// `source_x = floor((output_x + 0.5) * source_width / output_width)`, with the
/// same rule for y. This matches the coordinate convention used by common image
/// resampling libraries while keeping the calculation integer-only at the Wasm
/// boundary.
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
/// strategy: identity and row-copy fast paths, exact integer downscale, gated
/// span-copy for wide contiguous source-x runs, then a precomputed-offset
/// fallback that copies each RGBA pixel as one unaligned 32-bit word. The
/// sampling rule remains the deterministic pixel-center mapping documented on
/// [`resize_rgba_nearest`].
pub fn resize_rgba_nearest_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    crate::resize::scalar::nearest::resize_rgba_nearest_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

#[doc(hidden)]
pub use crate::resize::reference::nearest::{
    resize_rgba_nearest_reference, resize_rgba_nearest_reference_into,
};

#[doc(hidden)]
pub use crate::resize::scalar::nearest::resize_rgba_nearest_scalar_into;

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_nearest_tiling_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    crate::resize::tiling::nearest::resize_rgba_nearest_with_tiling_after_fast_paths(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        crate::resize::cpu_tiling::DEFAULT_ROW_BAND_TILING,
        true,
    )
}
