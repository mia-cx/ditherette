use std::f64::consts::PI;

use crate::{
    error::ProcessingError,
    image::ImageDimensions,
    resize::{
        scalar::convolution::resize_with_convolution_into,
        shared::{convolution::Kernel, lanczos::validate_window_size},
    },
};

#[derive(Debug, Clone, Copy)]
struct ScalarLanczos {
    window_size: f64,
}

impl ScalarLanczos {
    fn new(window_size: f64) -> Result<Self, ProcessingError> {
        validate_window_size(window_size)?;
        Ok(Self { window_size })
    }
}

impl Kernel for ScalarLanczos {
    fn radius(self) -> f64 {
        self.window_size
    }

    fn weight(self, distance: f64) -> f64 {
        // TODO(perf): Split the x==0 and x>=window_size guards into
        // contribution-table construction so the inner kernel can assume valid
        // non-zero support.
        // TODO(perf): Replace two sinc calls with a small polynomial/table
        // approximation once visual error is benchmarked against the reference.
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

/// Shared allocation-free implementation for Lanczos resize variants.
// TODO(perf): Add Lanczos2/Lanczos3-specific dispatch so fixed window sizes can
// use unrolled 4-tap/6-tap separable loops instead of the dynamic Kernel trait
// path.
// TODO(perf): Approximate or table sinc weights during contribution planning
// for Lanczos filters; sin() is expensive but only depends on axis samples.
// TODO(perf): Store normalized weights as fixed-point integers once planned;
// hot sampling can then use integer multiply-adds and deterministic rounding.
// TODO(perf): Specialize scale-aware minification separately from enlargement:
// downscales have wider support and benefit more from separable scratch buffers,
// while upscales can keep compact fixed tap counts.
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

fn sinc(x: f64) -> f64 {
    let x_pi = x * PI;
    x_pi.sin() / x_pi
}
