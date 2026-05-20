//! Spec bicubic resize.
//!
//! Bicubic is represented as a Catmull-Rom cubic reconstruction kernel applied
//! through the spec convolution engine. The preset is intentionally small: the
//! convolution module owns the sampling loop, and this file owns the cubic math.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    spec::resize::{
        common::{alignment::ResizeAnchor, sample::ResizeSample},
        scalar::convolution::{resize_convolution_into, ReconstructionKernel, SupportPolicy},
    },
};

/// Resizes `source` into `output` with Catmull-Rom bicubic filtering.
pub fn resize_bicubic_into<F>(
    source: ImageView<'_, F>,
    output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    resize_convolution_into(source, output, anchor, CatmullRom, support_policy);
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
