use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::area::{
        resize_rgba_area, resize_rgba_area_into, resize_rgba_area_reference,
        resize_rgba_area_reference_into,
    },
};

/// Resizes RGBA with a box filter.
///
/// For image minification, a box filter and area resampling are the same family:
/// source samples are averaged over a rectangular footprint. This module keeps
/// the box-filter name separate so callers and benchmarks can use common image
/// processing vocabulary without duplicating the implementation.
// TODO(perf): Decide whether box should remain a pure area alias or grow a
// cheaper approximate path. A classic unscaled box filter can use constant-size
// footprints, while exact area supports arbitrary fractional coverage.
pub fn resize_rgba_box(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_rgba_area(source_rgba, source_dimensions, output_dimensions)
}

/// Resizes into a caller-provided output buffer with a box filter.
// TODO(perf): Add integer-ratio box fast paths here if we want box semantics to
// prefer speed over exact area generality for common 2x/4x downscales.
// TODO(perf): Benchmark whether dispatching to a separable/rolling-sum box
// implementation beats the shared area code for large minification ratios.
pub fn resize_rgba_box_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_area_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

/// Straightforward reference implementation for the box filter.
#[doc(hidden)]
pub fn resize_rgba_box_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    resize_rgba_area_reference(source_rgba, source_dimensions, output_dimensions)
}

/// Allocation-free form of the box filter reference implementation.
#[doc(hidden)]
pub fn resize_rgba_box_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    resize_rgba_area_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

#[cfg(test)]
mod tests {
    use super::{resize_rgba_box, resize_rgba_box_reference};
    use crate::{image::ImageDimensions, resize::area::resize_rgba_area};

    #[test]
    fn box_is_area_alias() {
        let source_rgba: Vec<u8> = (0..9)
            .flat_map(|value| [value, value, value, 255])
            .collect();

        assert_eq!(
            resize_rgba_box(&source_rgba, dimensions(3, 3), dimensions(2, 2)).unwrap(),
            resize_rgba_area(&source_rgba, dimensions(3, 3), dimensions(2, 2)).unwrap()
        );
        assert_eq!(
            resize_rgba_box_reference(&source_rgba, dimensions(3, 3), dimensions(2, 2)).unwrap(),
            resize_rgba_area(&source_rgba, dimensions(3, 3), dimensions(2, 2)).unwrap()
        );
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
