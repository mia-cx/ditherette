//! Packed RGBA8 production kernels for nearest-neighbor resize.
//!
//! This module assumes the shared production resize boundary has already
//! rejected padded rows and non-RGBA formats. Its nearest-specific optimization
//! is copying unchanged RGBA pixels as unaligned `u32` words while preserving
//! the public byte layout.

use crate::image::{rgba8, ImageDimensions};

use super::alignment::AxisAlignment;
use super::plan::{alignment_offset, NearestResizePlan, NearestScaleClass};

pub(super) fn resize_with_plan_into(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    match plan.scale_class {
        NearestScaleClass::ExactDownscale => {
            let (x_factor, y_factor) = plan
                .exact_downscale
                .expect("exact-downscale class should have factors");
            let (x_alignment, y_alignment) = plan.anchor.axes();
            resize_exact_downscale(
                source,
                source_dimensions,
                output,
                plan.output_dimensions,
                x_factor,
                y_factor,
                x_alignment,
                y_alignment,
            );
        }
        NearestScaleClass::NearIdentityDownscale => {
            resize_span_copy(source, source_dimensions, output, plan);
        }
        NearestScaleClass::OtherDownscale => {
            resize_word_copy(source, source_dimensions, output, plan);
        }
        NearestScaleClass::Upscale => {
            resize_upscale_row_repeat(source, source_dimensions, output, plan);
        }
    }
}

fn resize_word_copy(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);

    for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = output_y * output_row_len;

        for (output_x, source_start) in plan.x_source_starts.iter().copied().enumerate() {
            copy_pixel_word(
                source,
                source_row_start + source_start,
                output,
                output_row_start + output_x * rgba8::RGBA8_CHANNELS,
            );
        }
    }
}

fn resize_upscale_row_repeat(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);
    let mut output_y = 0;

    while output_y < plan.y_coordinates.len() {
        let source_y = plan.y_coordinates[output_y];
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = output_y * output_row_len;

        for (output_x, source_start) in plan.x_source_starts.iter().copied().enumerate() {
            copy_pixel_word(
                source,
                source_row_start + source_start,
                output,
                output_row_start + output_x * rgba8::RGBA8_CHANNELS,
            );
        }

        let mut next_output_y = output_y + 1;
        while next_output_y < plan.y_coordinates.len()
            && plan.y_coordinates[next_output_y] == source_y
        {
            let duplicate_row_start = next_output_y * output_row_len;
            output.copy_within(
                output_row_start..output_row_start + output_row_len,
                duplicate_row_start,
            );
            next_output_y += 1;
        }
        output_y = next_output_y;
    }
}

fn resize_span_copy(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);

    for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = output_y * output_row_len;

        for span in &plan.source_x_copy_spans {
            let source_start = source_row_start + span.source_start;
            let output_start = output_row_start + span.output_start;
            output[output_start..output_start + span.byte_len]
                .copy_from_slice(&source[source_start..source_start + span.byte_len]);
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn resize_exact_downscale(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    output_dimensions: ImageDimensions,
    x_factor: u32,
    y_factor: u32,
    x_alignment: AxisAlignment,
    y_alignment: AxisAlignment,
) {
    let x_offset = alignment_offset(x_factor, x_alignment) as usize * rgba8::RGBA8_CHANNELS;
    let y_offset = alignment_offset(y_factor, y_alignment);
    let x_step = x_factor as usize * rgba8::RGBA8_CHANNELS;
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(output_dimensions);

    for output_y in 0..output_dimensions.height_usize() {
        let source_y = output_y as u32 * y_factor + y_offset;
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = output_y * output_row_len;

        for output_x in 0..output_dimensions.width_usize() {
            let source_start = source_row_start + output_x * x_step + x_offset;
            let output_start = output_row_start + output_x * rgba8::RGBA8_CHANNELS;
            copy_pixel_word(source, source_start, output, output_start);
        }
    }
}

pub(super) fn copy_pixel_word(
    source_row: &[u8],
    source_start: usize,
    output_row: &mut [u8],
    output_start: usize,
) {
    // SAFETY: Source and output offsets are derived from validated rows and
    // in-bounds nearest coordinate maps. Unaligned access is intentional for
    // packed byte-backed RGBA memory. This nearest-only word copy preserves the
    // byte-level RGBA8 contract; it does not expose a native-endian pixel format.
    unsafe {
        let source_ptr = source_row.as_ptr().add(source_start).cast::<u32>();
        let output_ptr = output_row.as_mut_ptr().add(output_start).cast::<u32>();
        output_ptr.write_unaligned(source_ptr.read_unaligned());
    }
}
