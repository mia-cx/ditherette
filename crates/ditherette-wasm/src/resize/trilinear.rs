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
// TODO(perf): Fast-path identity and non-minifying resizes before allocating the
// output Vec. Upscales can dispatch directly to optimized bilinear once that path
// exists.
// TODO(perf): Accept a caller-owned mip pyramid for repeated preview sizes of
// the same source so this allocating API does not rebuild levels each call.
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
    // TODO(perf): Cache mip pyramids for repeated previews of the same source.
    // TODO(perf): Build only the two mip levels needed for the target scale.
    // TODO(perf): Build mip levels incrementally in reusable scratch buffers to
    // avoid allocating and copying a fresh Vec at each level.
    // TODO(perf): Downsample mip levels with exact 2x box fast paths instead of
    // the general area reference path.
    // TODO(perf): Sample the two selected mip levels into one output pass. The
    // current reference path bilinearly resizes both levels into full buffers,
    // then walks the output a third time to blend them.
    // TODO(perf): Fuse bilinear sampling and LOD blending per pixel so the lower
    // and upper samples share coordinate math and avoid two intermediate images.
    // TODO(perf): Blend mip levels in a fixed-point pass instead of f64 per byte.
    // TODO(perf): Precompute bilinear source indices and weights per output axis
    // once for both mip levels; the output grid is shared.
    // TODO(perf): Special-case exact power-of-two minification. If blend is zero,
    // only one mip level is needed and the final LOD blend can be skipped.
    // TODO(perf): Add anisotropic handling instead of using max-axis LOD for both
    // axes; resizing width-only or height-only should not over-blur the other
    // axis and may need fewer mip levels.
    // TODO(perf): Store mip levels in a compact pyramid object with dimensions
    // and byte offsets to improve locality and reduce Vec metadata churn.
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
