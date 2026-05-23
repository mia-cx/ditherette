//! Packed RGBA8 convolution kernel.
//!
//! The kernel consumes preplanned x/y support taps and preserves the direct
//! y-major, x-minor contribution grouping used by the scalar convolution oracle.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::plan::{AxisTap, ConvolutionResizePlan};

pub(super) fn resize_packed_rgba8_with_convolution_filter_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &ConvolutionResizePlan,
) {
    let source_width = source.dimensions().width_usize();
    let output_width = output.dimensions().width_usize();
    let source_row_byte_len = source_width * rgba8::RGBA8_CHANNELS;
    let output_row_byte_len = output_width * rgba8::RGBA8_CHANNELS;
    let source_data = source.data();

    for (output_row, y_taps) in output
        .data_mut()
        .chunks_exact_mut(output_row_byte_len)
        .zip(&plan.y_taps)
    {
        for (output_pixel, x_taps) in output_row
            .chunks_exact_mut(rgba8::RGBA8_CHANNELS)
            .zip(&plan.x_taps)
        {
            write_convolution_pixel(
                output_pixel,
                source_data,
                source_row_byte_len,
                x_taps,
                y_taps,
            );
        }
    }
}

fn write_convolution_pixel(
    output_pixel: &mut [u8],
    source: &[u8],
    source_row_byte_len: usize,
    x_taps: &[AxisTap],
    y_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];
    let mut total_weight = 0.0;

    for y_tap in y_taps {
        let source_row_start = y_tap.index * source_row_byte_len;

        for x_tap in x_taps {
            let weight = x_tap.weight * y_tap.weight;
            let source_start = source_row_start + x_tap.index * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source[source_start..source_start + rgba8::RGBA8_CHANNELS];

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
