//! Exact f64 bilinear sampling of trilinear mip levels.
//!
//! Bilinear resize is expressed as a direct separable triangle-filter sum per
//! output pixel. The filter widens during minification. It mirrors the spec's
//! direct evaluation order, unlike the landed f32 separable bilinear kernel;
//! per-axis weights are planned once instead of per output pixel.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::scalar::trilinear::exact::common::{
        alignment::{map_axis_position, support_range, AxisAlignment, ResizeAnchor},
        sample::ResizeSample,
        taps::{AxisPlan, AxisTap},
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
    let (source_dimensions, output_dimensions) = (source.dimensions(), output.dimensions());
    let (x_alignment, y_alignment) = anchor.axes();
    let mut x = AxisPlan::default();
    let mut y = AxisPlan::default();
    plan_bilinear_axis(
        &mut x,
        source_dimensions.width(),
        output_dimensions.width(),
        x_alignment,
    );
    plan_bilinear_axis(
        &mut y,
        source_dimensions.height(),
        output_dimensions.height(),
        y_alignment,
    );
    let mut accumulated = vec![0.0; F::CHANNEL_COUNT];
    resize_bilinear_planned_into(source, output, &x, &y, &mut accumulated);
}

/// Nonzero triangle weights around one output position, clamped and in source order.
pub fn bilinear_taps(
    source_len: u32,
    output_len: u32,
    output: u32,
    alignment: AxisAlignment,
) -> impl Iterator<Item = AxisTap> {
    let scale = (f64::from(source_len) / f64::from(output_len)).max(1.0);
    let position = map_axis_position(output, source_len, output_len, alignment);
    support_range(position, scale).filter_map(move |source| {
        let weight = triangle_weight((source as f64 - position) / scale);
        (weight != 0.0).then(|| AxisTap {
            index: source.clamp(0, i64::from(source_len) - 1) as u32,
            weight,
        })
    })
}

/// Fill `plan` with every output coordinate's triangle weights for one axis.
pub fn plan_bilinear_axis(
    plan: &mut AxisPlan,
    source_len: u32,
    output_len: u32,
    alignment: AxisAlignment,
) {
    plan.fill(output_len, |output| {
        bilinear_taps(source_len, output_len, output, alignment)
    });
}

/// Applies planned x/y weights: y outer, x inner, weight `x * y`, normalized by their sum.
pub fn resize_bilinear_planned_into<F>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    x: &AxisPlan,
    y: &AxisPlan,
    accumulated: &mut [f64],
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    for output_y in 0..output.dimensions().height() {
        let y_taps = y.taps(output_y as usize);
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for (output_x, output_pixel) in output_row.chunks_exact_mut(F::CHANNEL_COUNT).enumerate() {
            let x_taps = x.taps(output_x);
            accumulated.fill(0.0);
            let mut total_weight = 0.0;

            for y_tap in y_taps {
                let source_row = source
                    .row(y_tap.index)
                    .expect("clamped source y should stay in bounds");

                for x_tap in x_taps {
                    let weight = x_tap.weight * y_tap.weight;
                    let source_start = x_tap.index as usize * F::CHANNEL_COUNT;
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

fn triangle_weight(distance: f64) -> f64 {
    if distance.abs() < 1.0 {
        1.0 - distance.abs()
    } else {
        0.0
    }
}
