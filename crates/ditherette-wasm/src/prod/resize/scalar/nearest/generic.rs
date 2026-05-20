use crate::image::{ImageFormat, ImageView, ImageViewMut};

use super::{alignment::AxisAlignment, alignment_offset, NearestResizePlan};

pub(super) fn resize_with_plan_into<F: ImageFormat>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    plan: &NearestResizePlan,
) {
    debug_assert_eq!(source.dimensions(), plan.source_dimensions);
    debug_assert_eq!(output.dimensions(), plan.output_dimensions);

    let (x_alignment, y_alignment) = plan.anchor.axes();
    if let Some((x_factor, y_factor)) = plan.exact_downscale {
        resize_exact_downscale(source, output, x_factor, y_factor, x_alignment, y_alignment);
        return;
    }

    for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
        let source_row = source
            .row(source_y)
            .expect("mapped source y should stay in bounds");
        let output_row = output
            .row_mut(output_y as u32)
            .expect("output y from dimensions should stay in bounds");

        // REJECT(perf): Generic exact-upscale run-fill regressed 2x by -21.03%
        // in `ditherette-bench run nearest --baseline accepted`; do not retry
        // without a 2x-specific strategy or different representative workload.
        // REJECT(perf): Replacing slice `copy_from_slice` with four scalar
        // channel assignments regressed every default nearest case by roughly
        // -45% to -56% in `ditherette-bench run nearest --baseline accepted`.
        for (output_x, source_start) in plan.x_source_starts.iter().copied().enumerate() {
            let output_start = output_x * F::CHANNEL_COUNT;
            let source_pixel = &source_row[source_start..source_start + F::CHANNEL_COUNT];
            let output_pixel = &mut output_row[output_start..output_start + F::CHANNEL_COUNT];

            output_pixel.copy_from_slice(source_pixel);
        }
    }
}

// REJECT(perf): Incrementing exact-downscale source/output offsets instead of
// multiplying per pixel did not improve the targeted 0.1x/0.125x/0.25x/0.5x
// cases and regressed 0.75x by -2.95% in `ditherette-bench run nearest
// --baseline accepted`.
fn resize_exact_downscale<F: ImageFormat>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    x_factor: u32,
    y_factor: u32,
    x_alignment: AxisAlignment,
    y_alignment: AxisAlignment,
) {
    let x_offset = alignment_offset(x_factor, x_alignment) as usize * F::CHANNEL_COUNT;
    let y_offset = alignment_offset(y_factor, y_alignment);
    let x_step = x_factor as usize * F::CHANNEL_COUNT;
    let output_width = output.dimensions().width_usize();

    for output_y in 0..output.dimensions().height() {
        let source_y = output_y * y_factor + y_offset;
        let source_row = source
            .row(source_y)
            .expect("mapped source y should stay in bounds");
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for output_x in 0..output_width {
            let source_start = output_x * x_step + x_offset;
            let output_start = output_x * F::CHANNEL_COUNT;
            let source_pixel = &source_row[source_start..source_start + F::CHANNEL_COUNT];
            let output_pixel = &mut output_row[output_start..output_start + F::CHANNEL_COUNT];

            output_pixel.copy_from_slice(source_pixel);
        }
    }
}
