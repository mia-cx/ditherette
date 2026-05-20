//! Spec exact area resize.
//!
//! Area resize treats each output pixel as a rectangle in source pixel space and
//! computes the coverage-weighted average of every source pixel it overlaps. The
//! implementation is intentionally direct so it can serve as an oracle for exact
//! coverage/integration behavior.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    spec::resize::common::sample::ResizeSample,
};

/// Resizes `source` into `output` using exact source-area coverage averaging.
pub fn resize_area_into<F>(source: ImageView<'_, F>, mut output: ImageViewMut<'_, F>)
where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
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
            let output_start = output_x as usize * F::CHANNEL_COUNT;
            let output_pixel = &mut output_row[output_start..output_start + F::CHANNEL_COUNT];
            let mut accumulated = vec![0.0; F::CHANNEL_COUNT];

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
                    let source_start = clamped_x * F::CHANNEL_COUNT;
                    let source_pixel = &source_row[source_start..source_start + F::CHANNEL_COUNT];

                    for channel in 0..F::CHANNEL_COUNT {
                        accumulated[channel] += source_pixel[channel].to_f64() * weight;
                    }
                }
            }

            for channel in 0..F::CHANNEL_COUNT {
                output_pixel[channel] = F::Storage::from_f64(accumulated[channel]);
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
