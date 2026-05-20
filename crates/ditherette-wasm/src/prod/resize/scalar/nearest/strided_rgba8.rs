use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::{alignment::AxisAlignment, alignment_offset, packed, NearestResizePlan};

pub(super) fn resize_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &NearestResizePlan,
) {
    let (x_alignment, y_alignment) = plan.anchor.axes();
    if let Some((x_factor, y_factor)) = plan.exact_downscale {
        resize_exact_downscale(source, output, x_factor, y_factor, x_alignment, y_alignment);
        return;
    }

    if plan.source_x_copy_spans.is_empty() {
        for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
            let source_row = source
                .row(source_y)
                .expect("mapped source y should stay in bounds");
            let output_row = output
                .row_mut(output_y as u32)
                .expect("output y from dimensions should stay in bounds");

            for (output_x, source_start) in plan.x_source_starts.iter().copied().enumerate() {
                packed::copy_pixel_word(
                    source_row,
                    source_start,
                    output_row,
                    output_x * rgba8::RGBA8_CHANNELS,
                );
            }
        }
        return;
    }

    for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
        let source_row = source
            .row(source_y)
            .expect("mapped source y should stay in bounds");
        let output_row = output
            .row_mut(output_y as u32)
            .expect("output y from dimensions should stay in bounds");

        for span in &plan.source_x_copy_spans {
            output_row[span.output_start..span.output_start + span.byte_len]
                .copy_from_slice(&source_row[span.source_start..span.source_start + span.byte_len]);
        }
    }
}

fn resize_exact_downscale(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    x_factor: u32,
    y_factor: u32,
    x_alignment: AxisAlignment,
    y_alignment: AxisAlignment,
) {
    let x_offset = alignment_offset(x_factor, x_alignment) as usize * rgba8::RGBA8_CHANNELS;
    let y_offset = alignment_offset(y_factor, y_alignment);
    let x_step = x_factor as usize * rgba8::RGBA8_CHANNELS;
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
            let output_start = output_x * rgba8::RGBA8_CHANNELS;
            packed::copy_pixel_word(source_row, source_start, output_row, output_start);
        }
    }
}
