use std::f32::consts::PI;

use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        scalar::{
            convolution::resize_with_convolution_into,
            convolution_2::resize_with_convolution_2_into,
        },
        shared::{convolution::Kernel, lanczos::validate_window_size},
    },
};

#[derive(Debug, Clone, Copy)]
struct ScalarLanczos {
    window_size: f32,
}

impl ScalarLanczos {
    fn new(window_size: f64) -> Result<Self, ProcessingError> {
        validate_window_size(window_size)?;
        Ok(Self {
            window_size: window_size as f32,
        })
    }
}

impl Kernel for ScalarLanczos {
    fn support(self) -> f32 {
        self.window_size
    }

    fn weight(self, distance: f32) -> f32 {
        // REJECT(perf): Moving support guards into contribution planning would
        // duplicate the generic convolution edge/zero-tap trimming logic, and
        // sinc approximation/table lookup would change the exact scalar oracle.
        let x = distance.abs();

        if x < f32::EPSILON {
            1.0
        } else if x >= self.window_size {
            0.0
        } else {
            sinc(x) * sinc(x / self.window_size)
        }
    }
}

/// Shared allocation-free implementation for Lanczos resize variants.
// REJECT(perf): Fixed-window dispatch, fixed-point weights, and scale-aware
// split planners overlap the convolution trials that regressed bicubic or risk
// changing f32 rounding. Keep Lanczos on the shared convolution base until a
// separate Lanczos benchmark loop justifies a dedicated implementation.
pub(crate) fn resize_rgba_lanczos_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    window_size: f64,
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    resize_with_convolution_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        ScalarLanczos::new(window_size)?,
        scale_aware,
    )
}

pub(crate) fn resize_rgba_lanczos_2_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    window_size: f64,
    scale_aware: bool,
) -> Result<(), ProcessingError> {
    // NOTE(perf): convolution_2 source-row vertical accumulation improved lanczos3_2
    // downscales by ~28-54% in `pnpm crit:resize:convolution_2 --baseline conv2_accepted`.
    // Small-source 2x upscale stays on the original vertical loop to avoid a ~7% regression.
    resize_with_convolution_2_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
        ScalarLanczos::new(window_size)?,
        scale_aware,
    )
}

fn sinc(x: f32) -> f32 {
    let x_pi = x * PI;
    x_pi.sin() / x_pi
}
