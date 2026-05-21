//! Packed RGBA8 scalar kernel for production area resize.
//!
//! This kernel assumes the shared production resize boundary has already
//! normalized source and output rows to packed RGBA8. It mirrors the spec area
//! coverage formula while keeping RGBA8-specific row traversal in one place.

use crate::image::{rgba8, ImageDimensions, ImageView, ImageViewMut, Rgba8};

use super::coverage::{clamp_i64, interval_overlap, output_coverage};

pub(super) fn resize_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, Rgba8>) {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    for output_y in 0..output_dimensions.height() {
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for output_x in 0..output_dimensions.width() {
            let coverage = output_coverage(output_x, output_y, x_scale, y_scale);
            let output_start = output_x as usize * rgba8::RGBA8_CHANNELS;
            let output_pixel = &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS];
            let accumulated = accumulate_pixel(source, source_dimensions, coverage);

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

fn accumulate_pixel(
    source: ImageView<'_, Rgba8>,
    source_dimensions: ImageDimensions,
    coverage: super::coverage::OutputCoverage,
) -> [f64; rgba8::RGBA8_CHANNELS] {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];

    for source_y in coverage.y_start.floor() as i64..coverage.y_end.ceil() as i64 {
        let y_overlap = interval_overlap(
            coverage.y_start,
            coverage.y_end,
            source_y as f64,
            source_y as f64 + 1.0,
        );
        if y_overlap == 0.0 {
            continue;
        }

        let clamped_y = clamp_i64(source_y, 0, i64::from(source_dimensions.height()) - 1) as u32;
        let source_row = source
            .row(clamped_y)
            .expect("clamped source y should stay in bounds");

        for source_x in coverage.x_start.floor() as i64..coverage.x_end.ceil() as i64 {
            let x_overlap = interval_overlap(
                coverage.x_start,
                coverage.x_end,
                source_x as f64,
                source_x as f64 + 1.0,
            );
            if x_overlap == 0.0 {
                continue;
            }

            let clamped_x =
                clamp_i64(source_x, 0, i64::from(source_dimensions.width()) - 1) as usize;
            let weight = x_overlap * y_overlap / coverage.area;
            let source_start = clamped_x * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];

            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    accumulated
}
