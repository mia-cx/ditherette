//! Spec bilinear resize.
//!
//! Bilinear resize is expressed as a direct separable triangle-filter sum per
//! output pixel. The filter widens during minification, making the oracle a
//! mathematically clear triangle resampler rather than a production two-tap fast
//! path.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::scalar::trilinear::exact::common::{
        alignment::ResizeAnchor, coordinates::map_axis_position, sample::ResizeSample,
    },
};

/// Resizes `source` into `output` with a triangle filter.
pub fn resize_bilinear_into<F>(
    source: ImageView<'_, F>,
    output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    let mut accumulated = vec![0.0; F::CHANNEL_COUNT];
    resize_bilinear_with_scratch_into(source, output, anchor, &mut accumulated);
}

pub fn resize_bilinear_with_scratch_into<F>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
    accumulated: &mut [f64],
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let (x_alignment, y_alignment) = anchor.axes();
    let x_scale =
        (f64::from(source_dimensions.width()) / f64::from(output_dimensions.width())).max(1.0);
    let y_scale =
        (f64::from(source_dimensions.height()) / f64::from(output_dimensions.height())).max(1.0);

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
            accumulated.fill(0.0);
            let mut total_weight = 0.0;

            for source_y in support_range(source_y_position, y_scale) {
                let y_weight = triangle_weight((source_y as f64 - source_y_position) / y_scale);
                if y_weight == 0.0 {
                    continue;
                }
                let clamped_y =
                    clamp_i64(source_y, 0, i64::from(source_dimensions.height()) - 1) as u32;
                let source_row = source
                    .row(clamped_y)
                    .expect("clamped source y should stay in bounds");

                for source_x in support_range(source_x_position, x_scale) {
                    let x_weight = triangle_weight((source_x as f64 - source_x_position) / x_scale);
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

fn support_range(position: f64, scale: f64) -> std::ops::RangeInclusive<i64> {
    let support = scale;
    (position - support).floor() as i64..=(position + support).ceil() as i64
}

fn triangle_weight(distance: f64) -> f64 {
    if distance.abs() < 1.0 {
        1.0 - distance.abs()
    } else {
        0.0
    }
}

fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}
