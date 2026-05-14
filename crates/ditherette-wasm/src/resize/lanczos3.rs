use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{buffers::allocate_output_rgba, lanczos::resize_rgba_lanczos_into},
};

pub(crate) const LANCZOS3_WINDOW_SIZE: f64 = 3.0;

/// Resizes RGBA with a fixed-window Lanczos3 filter.
// TODO(perf): Inline identity and equal-dimension checks here before allocating
// through the shared Lanczos path; this named mode has no additional setup.
// TODO(perf): Cache reusable Lanczos3 plans for repeated preview renders. Six
// taps per axis makes planning cost and memory traffic more noticeable than
// Lanczos2.
pub fn resize_rgba_lanczos3(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_lanczos3_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Resizes into a caller-provided output buffer with a fixed-window Lanczos3 filter.
// TODO(perf): Add fixed 6-tap Lanczos3 hot loops and precomputed fixed-point
// weights if the generic convolution path is too slow for this mode.
// TODO(perf): Precompute exactly six source indices/weights per output column
// and six y indices/weights per output row for normal upscales; variable tap
// Vecs are unnecessary when the support radius is fixed.
// TODO(perf): Split interior pixels from edge pixels. Interior Lanczos3 samples
// can avoid clamp/duplicate-edge handling and use predictable 6x6 footprints.
// TODO(perf): Add a two-pass separable implementation specialized for 3 lobes:
// horizontal 6-tap RGBA sums into scratch rows, then vertical 6-tap sums.
// TODO(perf): Use a ring buffer for horizontal scratch rows so vertical sampling
// reuses the six rows needed by adjacent output scanlines.
// TODO(perf): Accumulate RGBA channels together per source tap to reduce source
// loads and loop overhead across the wider Lanczos3 footprint.
// TODO(perf): Quantize normalized weights to fixed-point once, then use integer
// multiply-adds in the hot loop with one final deterministic rounding step.
// TODO(perf): Add one-axis paths for same-width or same-height resizes; Lanczos3
// is expensive enough that skipping one convolution axis should be visible.
pub fn resize_rgba_lanczos3_into(
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
        LANCZOS3_WINDOW_SIZE,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::{resize_rgba_lanczos3, LANCZOS3_WINDOW_SIZE};
    use crate::{image::ImageDimensions, resize::lanczos::resize_rgba_lanczos_reference};

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 13, value * 7, value * 3, 255])
            .collect();
        let dimensions = dimensions(3, 3);

        assert_eq!(
            resize_rgba_lanczos3(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
        assert_eq!(
            resize_rgba_lanczos_reference(
                &source_rgba,
                dimensions,
                dimensions,
                LANCZOS3_WINDOW_SIZE,
                false,
            )
            .unwrap(),
            source_rgba
        );
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
