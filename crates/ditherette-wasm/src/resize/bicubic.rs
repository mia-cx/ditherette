use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        convolution::{resize_with_convolution, resize_with_convolution_into, Kernel},
        convolution_reference::{
            resize_with_convolution_reference, resize_with_convolution_reference_into,
        },
    },
};

/// Resizes RGBA with a Catmull-Rom bicubic filter.
pub fn resize_rgba_bicubic(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_with_convolution(
        source_rgba,
        source_dimensions,
        output_dimensions,
        Bicubic,
        false,
    )
}

/// Resizes into a caller-provided output buffer with a Catmull-Rom bicubic filter.
// TODO(perf): Add a bicubic-specific separable implementation once benchmarks
// show this filter matters. The generic convolution path is readable, but it
// cannot exploit the fixed 4-tap Catmull-Rom footprint.
// TODO(perf): Precompute four source indices and four fixed-point weights per
// output coordinate. Bicubic has a constant support radius, so all per-pixel
// dynamic contribution assembly can move out of the hot loop.
pub fn resize_rgba_bicubic_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Bicubic,
        false,
    )
}

/// Straightforward reference implementation for Catmull-Rom bicubic resize.
#[doc(hidden)]
pub fn resize_rgba_bicubic_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_with_convolution_reference(
        source_rgba,
        source_dimensions,
        output_dimensions,
        Bicubic,
        false,
    )
}

/// Allocation-free form of the Catmull-Rom bicubic reference implementation.
#[doc(hidden)]
pub fn resize_rgba_bicubic_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_with_convolution_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Bicubic,
        false,
    )
}

#[derive(Debug, Clone, Copy)]
struct Bicubic;

impl Kernel for Bicubic {
    fn radius(self) -> f64 {
        2.0
    }

    fn weight(self, distance: f64) -> f64 {
        let x = distance.abs();
        let a = -0.5;

        if x < 1.0 {
            (a + 2.0) * x.powi(3) - (a + 3.0) * x.powi(2) + 1.0
        } else if x < 2.0 {
            a * x.powi(3) - 5.0 * a * x.powi(2) + 8.0 * a * x - 4.0 * a
        } else {
            0.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{resize_rgba_bicubic, resize_rgba_bicubic_reference};
    use crate::image::ImageDimensions;

    #[test]
    fn identity_resize_returns_same_bytes() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value * 13, value * 7, value * 3, 255])
            .collect();
        let dimensions = dimensions(3, 3);

        assert_eq!(
            resize_rgba_bicubic(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
        assert_eq!(
            resize_rgba_bicubic_reference(&source_rgba, dimensions, dimensions).unwrap(),
            source_rgba
        );
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
