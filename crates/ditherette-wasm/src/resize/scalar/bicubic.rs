use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        scalar::convolution::{resize_with_convolution, resize_with_convolution_into},
        shared::bicubic::Bicubic,
    },
};

#[allow(dead_code)]
pub(crate) fn resize_rgba_bicubic(
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

pub(crate) fn resize_rgba_bicubic_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    // TODO(perf): Add a bicubic-specific separable implementation once benchmarks
    // show this filter matters. The generic convolution path is readable, but it
    // cannot exploit the fixed 4-tap Catmull-Rom footprint.
    // TODO(perf): Precompute four source indices and four fixed-point weights per
    // output coordinate. Bicubic has a constant support radius, so all per-pixel
    // dynamic contribution assembly can move out of the hot loop.
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        Bicubic,
        false,
    )
}
