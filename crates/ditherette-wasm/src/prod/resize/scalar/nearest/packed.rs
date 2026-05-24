//! Packed RGBA8 production kernels for nearest-neighbor resize.
//!
//! This module assumes the shared production resize boundary has already
//! rejected padded rows and non-RGBA formats. Its nearest-specific optimization
//! is copying unchanged RGBA pixels as unaligned `u32` words while preserving
//! the public byte layout.

use crate::{
    image::{rgba8, ImageDimensions},
    prod::resize::common,
};

use super::{
    alignment::{axis_coordinate_map, AxisAlignment},
    plan::NearestResizePlan,
    scale::{alignment_offset, NearestScaleClass},
};

pub(super) fn resize_with_plan_into(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    if plan.same_width() {
        resize_vertical_only(source, source_dimensions, output, plan);
        return;
    }

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
        NearestScaleClass::ExactUpscale => {
            let (x_factor, y_factor) = plan
                .exact_upscale
                .expect("exact-upscale class should have factors");
            resize_exact_upscale(
                source,
                source_dimensions,
                output,
                plan.output_dimensions,
                x_factor,
                y_factor,
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

pub(super) fn resize_rows_with_plan_into(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
    y_start: u32,
    y_end: u32,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);
    let output_width = plan.output_dimensions.width_usize();
    let start = y_start as usize;
    let end = y_end as usize;
    let (x_alignment, y_alignment) = plan.anchor.axes();
    let x_source_starts;
    let x_source_starts = if plan.x_source_starts.is_empty() {
        x_source_starts = axis_coordinate_map(
            source_dimensions.width(),
            plan.output_dimensions.width(),
            x_alignment,
        )
        .into_iter()
        .map(|x| x as usize * rgba8::RGBA8_CHANNELS)
        .collect::<Vec<_>>();
        &x_source_starts
    } else {
        &plan.x_source_starts
    };
    let y_coordinates;
    let y_coordinates = if plan.y_coordinates.is_empty() {
        y_coordinates = axis_coordinate_map(
            source_dimensions.height(),
            plan.output_dimensions.height(),
            y_alignment,
        );
        &y_coordinates
    } else {
        &plan.y_coordinates
    };

    for (local_y, source_y) in y_coordinates[start..end].iter().copied().enumerate() {
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = local_y * output_row_len;
        copy_mapped_row_words(
            source,
            source_row_start,
            output,
            output_row_start,
            x_source_starts,
            output_width,
        );
    }
}

fn resize_vertical_only(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);

    for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
        let source_start = source_y as usize * source_row_len;
        let output_start = output_y * output_row_len;
        output[output_start..output_start + output_row_len]
            .copy_from_slice(&source[source_start..source_start + source_row_len]);
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
    let output_width = plan.output_dimensions.width_usize();
    let x_source_starts = &plan.x_source_starts;

    for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = output_y * output_row_len;
        copy_mapped_row_words(
            source,
            source_row_start,
            output,
            output_row_start,
            x_source_starts,
            output_width,
        );
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

        copy_mapped_row_words(
            source,
            source_row_start,
            output,
            output_row_start,
            &plan.x_source_starts,
            plan.output_dimensions.width_usize(),
        );

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

#[inline(never)]
fn copy_mapped_row_words(
    source: &[u8],
    source_row_start: usize,
    output: &mut [u8],
    output_row_start: usize,
    x_source_starts: &[usize],
    output_width: usize,
) {
    debug_assert!(output_width <= x_source_starts.len());
    debug_assert!(source_row_start < source.len());
    debug_assert!(output_row_start + output_width * rgba8::RGBA8_CHANNELS <= output.len());

    // SAFETY: Prod nearest validates packed RGBA8 rows at the boundary. Source
    // offsets are precomputed from in-bounds nearest coordinate maps and output
    // offsets walk the validated packed row one RGBA8 word at a time.
    unsafe {
        let source_row = source.as_ptr().add(source_row_start);
        let output_row = output.as_mut_ptr().add(output_row_start);
        let x_starts = x_source_starts.as_ptr();
        for output_x in 0..output_width {
            let source_start = *x_starts.add(output_x);
            let pixel = source_row.add(source_start).cast::<u32>().read_unaligned();
            output_row
                .add(output_x * rgba8::RGBA8_CHANNELS)
                .cast::<u32>()
                .write_unaligned(pixel);
        }
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

fn resize_exact_upscale(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    output_dimensions: ImageDimensions,
    x_factor: u32,
    y_factor: u32,
) {
    common::rgba8::resize_exact_pixel_repeat_into(
        source,
        source_dimensions,
        output,
        output_dimensions,
        x_factor as usize,
        y_factor as usize,
    );
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
    write_pixel_word(
        output_row,
        output_start,
        read_pixel_word(source_row, source_start),
    );
}

fn read_pixel_word(source_row: &[u8], source_start: usize) -> u32 {
    // SAFETY: Source offsets are derived from validated rows and in-bounds
    // nearest coordinate maps. Unaligned access is intentional for packed
    // byte-backed RGBA memory.
    unsafe {
        source_row
            .as_ptr()
            .add(source_start)
            .cast::<u32>()
            .read_unaligned()
    }
}

fn write_pixel_word(output_row: &mut [u8], output_start: usize, pixel: u32) {
    // SAFETY: Output offsets are derived from validated rows and in-bounds
    // nearest coordinate maps. Unaligned access is intentional for packed
    // byte-backed RGBA memory. This nearest-only word copy preserves the
    // byte-level RGBA8 contract; it does not expose a native-endian pixel format.
    unsafe {
        output_row
            .as_mut_ptr()
            .add(output_start)
            .cast::<u32>()
            .write_unaligned(pixel);
    }
}
