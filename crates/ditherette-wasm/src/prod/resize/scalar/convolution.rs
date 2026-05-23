//! Packed RGBA8 production convolution resize.
//!
//! This is the shared production engine for finite-support separable filters
//! such as bicubic and Lanczos. It duplicates the spec formulas instead of
//! importing the oracle, while specializing the implementation to the production
//! packed-RGBA8 boundary.

use std::ops::RangeInclusive;

use crate::{
    image::{rgba8, ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

/// Support policy for convolution kernels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportPolicy {
    /// Use the kernel's native support radius for every scale.
    Fixed,
    /// Widen support during minification to preserve more source information.
    ScaleAware,
}

/// One-dimensional anchor used when mapping output coordinates to source positions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AxisAlignment {
    /// Align the start edge of each axis.
    Start,
    /// Align pixel centers.
    Center,
    /// Align the end edge of each axis.
    End,
}

/// Two-dimensional resize anchor composed from x/y axis alignments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeAnchor {
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl ResizeAnchor {
    pub const fn axes(self) -> (AxisAlignment, AxisAlignment) {
        match self {
            Self::TopLeft => (AxisAlignment::Start, AxisAlignment::Start),
            Self::Top => (AxisAlignment::Center, AxisAlignment::Start),
            Self::TopRight => (AxisAlignment::End, AxisAlignment::Start),
            Self::Left => (AxisAlignment::Start, AxisAlignment::Center),
            Self::Center => (AxisAlignment::Center, AxisAlignment::Center),
            Self::Right => (AxisAlignment::End, AxisAlignment::Center),
            Self::BottomLeft => (AxisAlignment::Start, AxisAlignment::End),
            Self::Bottom => (AxisAlignment::Center, AxisAlignment::End),
            Self::BottomRight => (AxisAlignment::End, AxisAlignment::End),
        }
    }
}

impl Default for ResizeAnchor {
    fn default() -> Self {
        Self::Center
    }
}

/// Finite-support one-dimensional reconstruction kernel.
pub trait ReconstructionKernel {
    fn radius(&self) -> f64;
    fn weight(&self, distance: f64) -> f64;
}

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with a separable kernel.
pub fn resize_convolution_rgba8_into<K>(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
    kernel: K,
    support_policy: SupportPolicy,
) where
    K: ReconstructionKernel,
{
    common::rgba8::assert_packed_source(source, "convolution");
    common::rgba8::assert_packed_output(&output, "convolution");

    if source.dimensions() == output.dimensions() {
        output.data_mut().copy_from_slice(source.data());
        return;
    }

    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let source_row_byte_len = source_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_dimensions.width_usize() * rgba8::RGBA8_CHANNELS;
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
    let source_data = source.data();

    for (output_y, output_row) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .enumerate()
    {
        let source_y_position = map_axis_position(
            output_y as u32,
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
        );

        for (output_x, output_pixel) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .enumerate()
        {
            let source_x_position = map_axis_position(
                output_x as u32,
                source_dimensions.width(),
                output_dimensions.width(),
                x_alignment,
            );
            let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];
            let mut total_weight = 0.0;

            for source_y in support_range(source_y_position, y_support) {
                let y_weight = kernel.weight((source_y as f64 - source_y_position) / y_scale);
                if y_weight == 0.0 {
                    continue;
                }
                let clamped_y = clamp_i64(source_y, 0, i64::from(source_dimensions.height()) - 1);
                let source_row_start = clamped_y as usize * source_row_byte_len;

                for source_x in support_range(source_x_position, x_support) {
                    let x_weight = kernel.weight((source_x as f64 - source_x_position) / x_scale);
                    if x_weight == 0.0 {
                        continue;
                    }

                    let weight = x_weight * y_weight;
                    let clamped_x =
                        clamp_i64(source_x, 0, i64::from(source_dimensions.width()) - 1);
                    let source_start =
                        source_row_start + clamped_x as usize * rgba8::RGBA8_CHANNELS;
                    let source_pixel =
                        &source_data[source_start..source_start + rgba8::RGBA8_CHANNELS];

                    total_weight += weight;
                    for channel in 0..rgba8::RGBA8_CHANNELS {
                        accumulated[channel] += f64::from(source_pixel[channel]) * weight;
                    }
                }
            }

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = (accumulated[channel] / total_weight)
                    .clamp(0.0, 255.0)
                    .round() as u8;
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

fn map_axis_position(
    output_coordinate: u32,
    source_len: u32,
    output_len: u32,
    alignment: AxisAlignment,
) -> f64 {
    let output = f64::from(output_coordinate);
    let source_len = f64::from(source_len);
    let output_len = f64::from(output_len);

    match alignment {
        AxisAlignment::Start => output * source_len / output_len,
        AxisAlignment::Center => (output + 0.5) * source_len / output_len - 0.5,
        AxisAlignment::End => (output + 1.0) * source_len / output_len - 1.0,
    }
}

fn support_range(position: f64, support: f64) -> RangeInclusive<i64> {
    (position - support).floor() as i64..=(position + support).ceil() as i64
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}
