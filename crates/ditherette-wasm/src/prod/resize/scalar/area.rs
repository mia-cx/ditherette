//! Scalar production area resize.
//!
//! This is the first production area kernel. It intentionally mirrors the spec
//! area algorithm while enforcing the production resize boundary: normalized
//! packed RGBA8 input and output. Later benchmark-driven work can precompute
//! spans, reuse accumulators, or split rows without changing the coverage rule.

use crate::{
    image::{rgba8, ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with exact area averaging.
///
/// Each output pixel covers a rectangle in source-pixel space. The output value
/// is the coverage-weighted average of every overlapped source pixel, rounded
/// per channel to `u8`. This function duplicates the spec formula instead of
/// importing it so production remains independent from the oracle.
pub fn resize_area_rgba8_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, Rgba8>) {
    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");

    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());
    let output_area = x_scale * y_scale;

    for output_y in 0..output_dimensions.height() {
        let source_y_start = f64::from(output_y) * y_scale;
        let source_y_end = f64::from(output_y + 1) * y_scale;
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for output_x in 0..output_dimensions.width() {
            let source_x_start = f64::from(output_x) * x_scale;
            let source_x_end = f64::from(output_x + 1) * x_scale;
            let output_start = output_x as usize * rgba8::RGBA8_CHANNELS;
            let output_pixel = &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS];
            let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];

            for source_y in source_y_start.floor() as i64..source_y_end.ceil() as i64 {
                let y_overlap = interval_overlap(
                    source_y_start,
                    source_y_end,
                    source_y as f64,
                    source_y as f64 + 1.0,
                );
                if y_overlap == 0.0 {
                    continue;
                }

                let clamped_y =
                    clamp_i64(source_y, 0, i64::from(source_dimensions.height()) - 1) as u32;
                let source_row = source
                    .row(clamped_y)
                    .expect("clamped source y should stay in bounds");

                for source_x in source_x_start.floor() as i64..source_x_end.ceil() as i64 {
                    let x_overlap = interval_overlap(
                        source_x_start,
                        source_x_end,
                        source_x as f64,
                        source_x as f64 + 1.0,
                    );
                    if x_overlap == 0.0 {
                        continue;
                    }

                    let clamped_x =
                        clamp_i64(source_x, 0, i64::from(source_dimensions.width()) - 1) as usize;
                    let weight = x_overlap * y_overlap / output_area;
                    let source_start = clamped_x * rgba8::RGBA8_CHANNELS;
                    let source_pixel =
                        &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];

                    for channel in 0..rgba8::RGBA8_CHANNELS {
                        accumulated[channel] += f64::from(source_pixel[channel]) * weight;
                    }
                }
            }

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

fn interval_overlap(a_start: f64, a_end: f64, b_start: f64, b_end: f64) -> f64 {
    (a_end.min(b_end) - a_start.max(b_start)).max(0.0)
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}
