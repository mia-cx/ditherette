use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::lanczos::{
        resize_rgba_lanczos, resize_rgba_lanczos_into, resize_rgba_lanczos_reference,
        resize_rgba_lanczos_reference_into,
    },
};

pub(crate) const LANCZOS2_LOBES: f64 = 2.0;

/// Resizes RGBA with a fixed-radius Lanczos2 filter.
// TODO(perf): Inline identity and equal-dimension checks here before allocating
// through the shared Lanczos path; this named mode knows it has no extra setup.
// TODO(perf): For small preview outputs, consider a direct single-pass Lanczos2
// implementation to avoid contribution-table allocation overhead.
pub fn resize_rgba_lanczos2(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_rgba_lanczos(
        source_rgba,
        source_dimensions,
        output_dimensions,
        LANCZOS2_LOBES,
        false,
    )
}

/// Resizes into a caller-provided output buffer with a fixed-radius Lanczos2 filter.
// TODO(perf): Add fixed 4-tap Lanczos2 hot loops and precomputed fixed-point
// weights if the generic convolution path is too slow for this mode.
// TODO(perf): Precompute exactly four x indices/weights per output column and
// four y indices/weights per output row for normal upscales; Lanczos2 support is
// fixed, so Vec-backed variable tap lists are unnecessary there.
// TODO(perf): Split unclamped interior pixels from clamped edge pixels. Interior
// Lanczos2 samples can use four predictable taps per axis with no bounds merges.
// TODO(perf): Add a two-pass separable implementation specialized for 2 lobes:
// horizontal 4-tap RGBA sums into a scratch row buffer, then vertical 4-tap sums.
// TODO(perf): Pack RGBA accumulation so each source tap is loaded once and all
// channels are accumulated together instead of re-walking taps per channel.
// TODO(perf): Use a smaller scratch/contribution representation than Lanczos3;
// Lanczos2's narrower support should reduce memory traffic and cache pressure.
// TODO(perf): Add same-width and same-height one-axis paths, especially for
// thumbnails constrained by only one dimension.
pub fn resize_rgba_lanczos2_into(
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
        LANCZOS2_LOBES,
        false,
    )
}

/// Straightforward reference implementation for Lanczos2 resize.
#[doc(hidden)]
pub fn resize_rgba_lanczos2_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_rgba_lanczos_reference(
        source_rgba,
        source_dimensions,
        output_dimensions,
        LANCZOS2_LOBES,
        false,
    )
}

/// Allocation-free form of the Lanczos2 reference implementation.
#[doc(hidden)]
pub fn resize_rgba_lanczos2_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_lanczos_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        LANCZOS2_LOBES,
        false,
    )
}

#[cfg(test)]
mod tests {
    use super::resize_rgba_lanczos2;
    use crate::image::ImageDimensions;

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 13, value * 7, value * 3, 255])
            .collect();
        let dimensions = dimensions(3, 3);

        assert_eq!(
            resize_rgba_lanczos2(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
