//! Spec Lanczos resize.
//!
//! Lanczos is represented as a windowed-sinc reconstruction kernel applied
//! through the spec convolution engine. Radius is explicit so the same readable
//! oracle covers Lanczos2, Lanczos3, and future radius choices.

use std::f64::consts::PI;

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    spec::resize::{
        common::{alignment::ResizeAnchor, sample::ResizeSample},
        scalar::convolution::{resize_convolution_into, ReconstructionKernel, SupportPolicy},
    },
};

/// Resizes `source` into `output` with a Lanczos kernel of `radius` lobes.
pub fn resize_lanczos_into<F>(
    source: ImageView<'_, F>,
    output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
    radius: u32,
    support_policy: SupportPolicy,
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    assert!(radius > 0, "Lanczos radius must be greater than zero");
    resize_convolution_into(
        source,
        output,
        anchor,
        Lanczos {
            radius: f64::from(radius),
        },
        support_policy,
    );
}

/// Lanczos2 convenience wrapper.
pub fn resize_lanczos2_into<F>(
    source: ImageView<'_, F>,
    output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    resize_lanczos_into(source, output, anchor, 2, support_policy);
}

/// Lanczos3 convenience wrapper.
pub fn resize_lanczos3_into<F>(
    source: ImageView<'_, F>,
    output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
    support_policy: SupportPolicy,
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    resize_lanczos_into(source, output, anchor, 3, support_policy);
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
