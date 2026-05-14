use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        reference::convolution::{
            resize_with_convolution_reference, resize_with_convolution_reference_into,
        },
        shared::bicubic::Bicubic,
    },
};

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
