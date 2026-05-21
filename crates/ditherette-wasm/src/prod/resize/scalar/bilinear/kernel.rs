//! Packed RGBA8 bilinear kernel.
//!
//! The current production kernel is intentionally direct: map one output pixel
//! to source space, apply triangle-filter support, normalize by total weight,
//! then round exactly like the independent spec oracle.

use crate::image::{ImageView, ImageViewMut, Rgba8};

use super::{AxisTap, BilinearResizePlan};

const RGBA8_CHANNELS: usize = 4;

pub(super) fn resize_packed_rgba8_with_triangle_filter_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &BilinearResizePlan,
) {
    // NOTE(perf): Nonzero x/y support taps are cached in `BilinearResizePlan`;
    // keep duplicate clamped edge taps separate so accumulation order matches the
    // direct exact oracle path.
    for (output_y, y_taps) in plan.y_taps.iter().enumerate() {
        let output_row = output
            .row_mut(output_y as u32)
            .expect("output y from dimensions should stay in bounds");

        for (output_x, x_taps) in plan.x_taps.iter().enumerate() {
            let output_start = output_x * RGBA8_CHANNELS;
            let output_pixel = &mut output_row[output_start..output_start + RGBA8_CHANNELS];
            write_resized_pixel(source, output_pixel, x_taps, y_taps);
        }
    }
}

// NOTE(perf): Edge/interior splitting became unnecessary after the plan started
// storing clamped taps; the hot kernel no longer performs per-tap clamping.
// REJECT(perf): Passing packed source data plus row byte length directly into
// this writer preserved exactness but regressed representative large/upscale
// cases by roughly -5% to -19% in `ditherette-bench run bilinear --baseline
// accepted`; keep the `ImageView::row` lookup shape after tap planning.
fn write_resized_pixel(
    source: ImageView<'_, Rgba8>,
    output_pixel: &mut [u8],
    x_taps: &[AxisTap],
    y_taps: &[AxisTap],
) {
    let mut accumulated = [0.0; RGBA8_CHANNELS];
    let mut total_weight = 0.0;

    // REJECT(perf): Precomputing x/y weight sums and replacing the nested
    // `total_weight += x_weight * y_weight` accumulation changed f64 rounding
    // and failed exact oracle output in `prod_resize_bilinear` for anchored
    // cases. Keep denominator accumulation in the same order as channel sums.
    for y_tap in y_taps {
        let source_row = source
            .row(y_tap.index as u32)
            .expect("clamped source y should stay in bounds");

        for x_tap in x_taps {
            let weight = x_tap.weight * y_tap.weight;
            let source_start = x_tap.index * RGBA8_CHANNELS;
            let source_pixel = &source_row[source_start..source_start + RGBA8_CHANNELS];

            total_weight += weight;
            // REJECT(perf): Reading RGBA8 as an unaligned `u32` and widening
            // bytes locally preserved exactness but regressed representative
            // large/upscale cases, including box 0.5x and selfie 2x, in
            // `ditherette-bench run bilinear --baseline accepted`. Keep slice
            // channel loads unless the surrounding kernel shape changes.
            for channel in 0..RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    for channel in 0..RGBA8_CHANNELS {
        output_pixel[channel] = (accumulated[channel] / total_weight)
            .clamp(0.0, 255.0)
            .round() as u8;
    }
}
