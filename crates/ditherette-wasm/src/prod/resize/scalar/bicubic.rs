//! Packed RGBA8 production bicubic resize.
//!
//! Bicubic is represented as a Catmull-Rom cubic reconstruction kernel applied
//! through the production convolution engine.

use crate::image::{ImageView, ImageViewMut, Rgba8};

use super::convolution::{
    resize_convolution_rgba8_into, ReconstructionKernel, ResizeAnchor, SupportPolicy,
};

/// Resizes packed RGBA8 `source` into packed RGBA8 `output` with Catmull-Rom bicubic filtering.
pub fn resize_bicubic_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) {
    resize_convolution_rgba8_into(source, output, anchor, CatmullRom, support_policy);
}

#[derive(Debug, Clone, Copy)]
struct CatmullRom;

impl ReconstructionKernel for CatmullRom {
    fn radius(&self) -> f64 {
        2.0
    }

    fn weight(&self, distance: f64) -> f64 {
        cubic_weight(distance, -0.5)
    }
}

fn cubic_weight(distance: f64, a: f64) -> f64 {
    let x = distance.abs();

    if x < 1.0 {
        (a + 2.0) * x * x * x - (a + 3.0) * x * x + 1.0
    } else if x < 2.0 {
        a * x * x * x - 5.0 * a * x * x + 8.0 * a * x - 4.0 * a
    } else {
        0.0
    }
}
