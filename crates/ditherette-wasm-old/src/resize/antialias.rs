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
    // REJECT(perf): Branching per pixel to split corners/edges/interior kept
    // correctness but regressed antialias by ~30-60%; the generic combined
    // footprint loop stays faster.
    // NOTE(perf): Separable blur would add scratch-row plumbing; the accepted
    // one-pass RGBA accumulator now makes memory bandwidth the likely limiter.
    // REJECT(perf): Constant-divisor interior specialization was part of the
    // split-path attempt above and regressed; keep tracked count/division.
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
