use std::f64::consts::PI;

use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        reference::convolution::{
            resize_with_convolution_reference, resize_with_convolution_reference_into,
        },
        shared::{convolution::Kernel, lanczos::validate_window_size},
    },
};

#[derive(Debug, Clone, Copy)]
struct ReferenceLanczos {
    window_size: f64,
}

impl ReferenceLanczos {
    fn new(window_size: f64) -> Result<Self, ProcessingError> {
        validate_window_size(window_size)?;
        Ok(Self { window_size })
    }
}

impl Kernel for ReferenceLanczos {
    fn radius(self) -> f64 {
        self.window_size
    }

    fn weight(self, distance: f64) -> f64 {
        let x = distance.abs();

        if x < f64::EPSILON {
            1.0
        } else if x >= self.window_size {
            0.0
        } else {
            sinc(x) * sinc(x / self.window_size)
        }
    }
}

/// Straightforward reference implementation for Lanczos resize.
pub fn resize_rgba_lanczos_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    window_size: f64,
    scale_aware: bool,
) -> Result<Vec<u8>, ProcessingError> {
    resize_with_convolution_reference(
        source_rgba,
        source_dimensions,
        output_dimensions,
        ReferenceLanczos::new(window_size)?,
        scale_aware,
    )
}

/// Allocation-free form of the Lanczos reference implementation.
pub fn resize_rgba_lanczos_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    window_size: f64,
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    resize_with_convolution_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        ReferenceLanczos::new(window_size)?,
        scale_aware,
    )
}

fn sinc(x: f64) -> f64 {
    let x_pi = x * PI;
    x_pi.sin() / x_pi
}
