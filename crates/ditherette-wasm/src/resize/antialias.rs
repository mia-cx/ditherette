use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
};

/// Applies a small 3x3 box blur as a post-resize antialiasing pass.
///
/// This is intentionally a utility rather than a recommended resize strategy.
/// Integrated scale-aware filters such as area or scale-aware Lanczos usually
/// preserve detail better because they antialias while sampling from the
/// original image.
pub fn antialias_rgba_box3(
    source_rgba: &[u8],
    dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    rgba::validate_rgba_buffer(source_rgba, dimensions)?;
    let mut output_rgba = vec![0; rgba::checked_rgba_byte_len(dimensions)?];
    antialias_rgba_box3_into(source_rgba, dimensions, &mut output_rgba)?;
    Ok(output_rgba)
}

/// Applies a 3x3 box antialiasing pass into a caller-provided output buffer.
pub fn antialias_rgba_box3_into(
    source_rgba: &[u8],
    dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // TODO(perf): Split corners, edges, and interior. Interior pixels always use
    // a 3x3 footprint, so they can avoid saturating/min bounds checks and the
    // variable divisor in the hot path.
    // TODO(perf): Use a separable box blur: horizontal 3-pixel sums into a
    // scratch row buffer, then vertical sums over those intermediates.
    // TODO(perf): Sum RGBA channels together per sample instead of re-walking
    // the same 3x3 footprint once per channel.
    // TODO(perf): For interior pixels, divide by the constant 9 with a fixed
    // reciprocal multiply instead of tracking count and using integer division.
    crate::resize::scalar::antialias::antialias_rgba_box3_into(source_rgba, dimensions, output_rgba)
}

#[doc(hidden)]
pub use crate::resize::reference::antialias::{
    antialias_rgba_box3_reference, antialias_rgba_box3_reference_into,
};

#[cfg(test)]
mod tests {
    use super::{antialias_rgba_box3, antialias_rgba_box3_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn blurs_center_with_neighbor_average() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 10, value * 10, value * 10, 255])
            .collect();

        let output_rgba = antialias_rgba_box3(&source_rgba, dimensions(3, 3)).unwrap();
        let reference_rgba = antialias_rgba_box3_reference(&source_rgba, dimensions(3, 3)).unwrap();

        assert_eq!(output_rgba, reference_rgba);
        assert_eq!(&output_rgba[16..20], &[40, 40, 40, 255]);
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
