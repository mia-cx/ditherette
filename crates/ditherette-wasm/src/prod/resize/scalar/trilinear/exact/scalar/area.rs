//! Exact f64 area reduction for trilinear mip levels.
//!
//! Area resize treats each output pixel as a rectangle in source pixel space and
//! computes the coverage-weighted average of every source pixel it overlaps. It
//! mirrors the spec's direct evaluation order so mip levels stay byte-identical;
//! per-axis overlaps are planned once instead of per output pixel.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::scalar::trilinear::exact::common::{
        sample::ResizeSample,
        taps::{AxisPlan, AxisTap},
    },
};

/// Resizes `source` into `output` using exact source-area coverage averaging.
pub fn resize_area_into<F>(source: ImageView<'_, F>, output: ImageViewMut<'_, F>)
where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    let (source_dimensions, output_dimensions) = (source.dimensions(), output.dimensions());
    let mut x = AxisPlan::default();
    let mut y = AxisPlan::default();
    plan_area_axis(&mut x, source_dimensions.width(), output_dimensions.width());
    plan_area_axis(
        &mut y,
        source_dimensions.height(),
        output_dimensions.height(),
    );
    let mut accumulated = vec![0.0; F::CHANNEL_COUNT];
    resize_area_planned_into(source, output, &x, &y, &mut accumulated);
}

/// Nonzero source overlaps of one output interval, clamped and in source order.
pub fn area_taps(source_len: u32, output_len: u32, output: u32) -> impl Iterator<Item = AxisTap> {
    let scale = f64::from(source_len) / f64::from(output_len);
    let start = f64::from(output) * scale;
    let end = f64::from(output + 1) * scale;
    (start.floor() as i64..end.ceil() as i64).filter_map(move |source| {
        let overlap = interval_overlap(start, end, source as f64, source as f64 + 1.0);
        (overlap != 0.0).then(|| AxisTap {
            index: source.clamp(0, i64::from(source_len) - 1) as u32,
            weight: overlap,
        })
    })
}

/// Fill `plan` with every output coordinate's area overlaps for one axis.
pub fn plan_area_axis(plan: &mut AxisPlan, source_len: u32, output_len: u32) {
    plan.fill(output_len, |output| {
        area_taps(source_len, output_len, output)
    });
}

/// Applies planned x/y overlaps: y outer, x inner, weight `x * y / area` as the spec does.
pub fn resize_area_planned_into<F>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    x: &AxisPlan,
    y: &AxisPlan,
    accumulated: &mut [f64],
) where
    F: ImageFormat,
    F::Storage: ResizeSample,
{
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());
    let output_area = x_scale * y_scale;

    for output_y in 0..output_dimensions.height() {
        let y_taps = y.taps(output_y as usize);
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for (output_x, output_pixel) in output_row.chunks_exact_mut(F::CHANNEL_COUNT).enumerate() {
            let x_taps = x.taps(output_x);
            accumulated.fill(0.0);

            for y_tap in y_taps {
                let source_row = source
                    .row(y_tap.index)
                    .expect("clamped source y should stay in bounds");

                for x_tap in x_taps {
                    let weight = x_tap.weight * y_tap.weight / output_area;
                    let source_start = x_tap.index as usize * F::CHANNEL_COUNT;
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
