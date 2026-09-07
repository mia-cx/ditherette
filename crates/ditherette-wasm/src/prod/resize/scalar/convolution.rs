//! Spec convolution resize for windowed reconstruction kernels.
//!
//! This module is the readable shared oracle for filters like bicubic and
//! Lanczos. It directly evaluates a separable finite-support kernel over each
//! output pixel instead of precomputing contribution tables or using two-pass
//! scratch buffers.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::common::{
        alignment::ResizeAnchor, coordinates::map_axis_position, sample::ResizeSample,
    },
};

/// Support policy for convolution kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportPolicy {
    /// Use the kernel's native support radius for every scale.
    Fixed,
    /// Widen support during minification to preserve more source information.
    ScaleAware,
}

/// Finite-support one-dimensional reconstruction kernel.
pub trait ReconstructionKernel {
    fn radius(&self) -> f64;
    fn weight(&self, distance: f64) -> f64;
}

/// Resizes `source` into `output` with a separable reconstruction kernel.
pub fn resize_convolution_into<F, K>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
    kernel: K,
    support_policy: SupportPolicy,
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
    K: ReconstructionKernel,
{
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let (x_alignment, y_alignment) = anchor.axes();
    let x_scale = axis_kernel_scale(
        source_dimensions.width(),
        output_dimensions.width(),
        support_policy,
    );
    let y_scale = axis_kernel_scale(
        source_dimensions.height(),
        output_dimensions.height(),
        support_policy,
    );
    let x_support = kernel.radius() * x_scale;
    let y_support = kernel.radius() * y_scale;

    for output_y in 0..output_dimensions.height() {
        let source_y_position = map_axis_position(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
        );
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for output_x in 0..output_dimensions.width() {
            let source_x_position = map_axis_position(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
                x_alignment,
            );
            let output_start = output_x as usize * F::CHANNEL_COUNT;
            let output_pixel = &mut output_row[output_start..output_start + F::CHANNEL_COUNT];
            let mut accumulated = vec![0.0; F::CHANNEL_COUNT];
            let mut total_weight = 0.0;

            for source_y in support_range(source_y_position, y_support) {
                let y_weight = kernel.weight((source_y as f64 - source_y_position) / y_scale);
                if y_weight == 0.0 {
                    continue;
                }
                let clamped_y =
                    clamp_i64(source_y, 0, i64::from(source_dimensions.height()) - 1) as u32;
                let source_row = source
                    .row(clamped_y)
                    .expect("clamped source y should stay in bounds");

                for source_x in support_range(source_x_position, x_support) {
                    let x_weight = kernel.weight((source_x as f64 - source_x_position) / x_scale);
                    if x_weight == 0.0 {
                        continue;
                    }

                    let weight = x_weight * y_weight;
                    let clamped_x =
                        clamp_i64(source_x, 0, i64::from(source_dimensions.width()) - 1) as usize;
                    let source_start = clamped_x * F::CHANNEL_COUNT;
                    let source_pixel = &source_row[source_start..source_start + F::CHANNEL_COUNT];

                    total_weight += weight;
                    for channel in 0..F::CHANNEL_COUNT {
                        accumulated[channel] += source_pixel[channel].to_f64() * weight;
                    }
                }
            }

            for channel in 0..F::CHANNEL_COUNT {
                output_pixel[channel] = F::Storage::from_f64(accumulated[channel] / total_weight);
            }
        }
    }
}

fn axis_kernel_scale(source_len: u32, output_len: u32, support_policy: SupportPolicy) -> f64 {
    match support_policy {
        SupportPolicy::Fixed => 1.0,
        SupportPolicy::ScaleAware => (f64::from(source_len) / f64::from(output_len)).max(1.0),
    }
}

fn support_range(position: f64, support: f64) -> std::ops::RangeInclusive<i64> {
    (position - support).floor() as i64..=(position + support).ceil() as i64
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}
