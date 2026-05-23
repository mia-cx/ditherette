//! Packed RGBA8 production Lanczos resize.
//!
//! Lanczos is represented as a windowed-sinc reconstruction kernel applied
//! through the production convolution engine.

use std::f64::consts::PI;

use crate::image::{ImageView, ImageViewMut, Rgba8};

use super::convolution::{
    resize_convolution_rgba8_into, ReconstructionKernel, ResizeAnchor, SupportPolicy,
};

/// Resizes packed RGBA8 `source` into packed RGBA8 `output` with a Lanczos kernel.
pub fn resize_lanczos_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    radius: u32,
    support_policy: SupportPolicy,
) {
    assert!(radius > 0, "Lanczos radius must be greater than zero");
    resize_convolution_rgba8_into(
        source,
        output,
        anchor,
        Lanczos {
            radius: f64::from(radius),
        },
        support_policy,
    );
}

/// Lanczos2 convenience wrapper for packed RGBA8.
pub fn resize_lanczos2_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    resize_lanczos_rgba8_into(source, output, anchor, 2, support_policy);
}

/// Lanczos3 convenience wrapper for packed RGBA8.
pub fn resize_lanczos3_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    resize_lanczos_rgba8_into(source, output, anchor, 3, support_policy);
}

#[derive(Debug, Clone, Copy)]
struct Lanczos {
    radius: f64,
}

impl ReconstructionKernel for Lanczos {
    fn radius(&self) -> f64 {
        self.radius
    }

    fn weight(&self, distance: f64) -> f64 {
        let x = distance.abs();
        if x == 0.0 {
            1.0
        } else if x < self.radius {
            sinc(x) * sinc(x / self.radius)
        } else {
            0.0
        }
    }
}

fn sinc(value: f64) -> f64 {
    if value == 0.0 {
        1.0
    } else {
        let x = PI * value;
        x.sin() / x
    }
}
