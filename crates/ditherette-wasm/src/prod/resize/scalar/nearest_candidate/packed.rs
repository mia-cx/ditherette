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
    alignment::AxisAlignment,
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
    if plan.same_width() {
        resize_vertical_only_rows(source, source_dimensions, output, plan, y_start, y_end);
        return;
    }

    match plan.scale_class {
        NearestScaleClass::ExactDownscale => {
            let (x_factor, y_factor) = plan
                .exact_downscale
                .expect("exact-downscale class should have factors");
            let (x_alignment, y_alignment) = plan.anchor.axes();
            resize_exact_downscale_rows(
                source,
                source_dimensions,
                output,
                plan.output_dimensions,
                x_factor,
                y_factor,
                x_alignment,
                y_alignment,
                y_start,
                y_end,
            );
        }
        NearestScaleClass::ExactUpscale => {
            let (x_factor, y_factor) = plan
                .exact_upscale
                .expect("exact-upscale class should have factors");
            resize_exact_upscale_rows(
                source,
                source_dimensions,
                output,
                plan.output_dimensions,
                x_factor,
                y_factor,
                y_start,
                y_end,
            );
        }
        NearestScaleClass::NearIdentityDownscale => {
            resize_span_copy_rows(source, source_dimensions, output, plan, y_start, y_end);
        }
        NearestScaleClass::OtherDownscale => {
            resize_word_copy_rows(source, source_dimensions, output, plan, y_start, y_end);
        }
        NearestScaleClass::Upscale => {
            resize_upscale_row_repeat_rows(source, source_dimensions, output, plan, y_start, y_end);
        }
    }
}

fn resize_vertical_only(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    resize_vertical_only_rows(
        source,
        source_dimensions,
        output,
        plan,
        0,
        plan.output_dimensions.height(),
    );
}

fn resize_vertical_only_rows(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
    y_start: u32,
    y_end: u32,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);

    for (local_y, source_y) in plan.y_coordinates[y_start as usize..y_end as usize]
        .iter()
        .copied()
        .enumerate()
    {
        let source_start = source_y as usize * source_row_len;
        let output_start = local_y * output_row_len;
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
    resize_word_copy_rows(
        source,
        source_dimensions,
        output,
        plan,
        0,
        plan.output_dimensions.height(),
    );
}

fn resize_word_copy_rows(
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
    let x_source_starts = &plan.x_source_starts;

    for (local_y, source_y) in plan.y_coordinates[y_start as usize..y_end as usize]
        .iter()
        .copied()
        .enumerate()
    {
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

fn resize_upscale_row_repeat(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    resize_upscale_row_repeat_rows(
        source,
        source_dimensions,
        output,
        plan,
        0,
        plan.output_dimensions.height(),
    );
}

fn resize_upscale_row_repeat_rows(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
    y_start: u32,
    y_end: u32,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);
    let mut absolute_y = y_start as usize;
    let end = y_end as usize;

    while absolute_y < end {
        let source_y = plan.y_coordinates[absolute_y];
        let source_row_start = source_y as usize * source_row_len;
        let local_y = absolute_y - y_start as usize;
        let output_row_start = local_y * output_row_len;

        copy_mapped_row_words(
            source,
            source_row_start,
            output,
            output_row_start,
            &plan.x_source_starts,
            plan.output_dimensions.width_usize(),
        );

        let mut next_y = absolute_y + 1;
        while next_y < end && plan.y_coordinates[next_y] == source_y {
            let duplicate_row_start = (next_y - y_start as usize) * output_row_len;
            output.copy_within(
                output_row_start..output_row_start + output_row_len,
                duplicate_row_start,
            );
            next_y += 1;
        }
        absolute_y = next_y;
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
    resize_span_copy_rows(
        source,
        source_dimensions,
        output,
        plan,
        0,
        plan.output_dimensions.height(),
    );
}

fn resize_span_copy_rows(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
    y_start: u32,
    y_end: u32,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(plan.output_dimensions);

    for (local_y, source_y) in plan.y_coordinates[y_start as usize..y_end as usize]
        .iter()
        .copied()
        .enumerate()
    {
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = local_y * output_row_len;

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

fn resize_exact_upscale_rows(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    output_dimensions: ImageDimensions,
    x_factor: u32,
    y_factor: u32,
    y_start: u32,
    y_end: u32,
) {
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(output_dimensions);
    let output_width = output_dimensions.width_usize();
    let x_factor = x_factor as usize;
    let y_factor = y_factor as usize;
    let mut absolute_y = y_start as usize;
    let end = y_end as usize;

    while absolute_y < end {
        let source_y = absolute_y / y_factor;
        let source_row_start = source_y * source_row_len;
        let local_y = absolute_y - y_start as usize;
        let output_row_start = local_y * output_row_len;

        for output_x in 0..output_width {
            let source_x = output_x / x_factor;
            copy_pixel_word(
                source,
                source_row_start + source_x * rgba8::RGBA8_CHANNELS,
                output,
                output_row_start + output_x * rgba8::RGBA8_CHANNELS,
            );
        }

        let mut next_y = absolute_y + 1;
        while next_y < end && next_y / y_factor == source_y {
            let duplicate_row_start = (next_y - y_start as usize) * output_row_len;
            output.copy_within(
                output_row_start..output_row_start + output_row_len,
                duplicate_row_start,
            );
            next_y += 1;
        }
        absolute_y = next_y;
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
    resize_exact_downscale_rows(
        source,
        source_dimensions,
        output,
        output_dimensions,
        x_factor,
        y_factor,
        x_alignment,
        y_alignment,
        0,
        output_dimensions.height(),
    );
}

#[allow(clippy::too_many_arguments)]
fn resize_exact_downscale_rows(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    output_dimensions: ImageDimensions,
    x_factor: u32,
    y_factor: u32,
    x_alignment: AxisAlignment,
    y_alignment: AxisAlignment,
    y_start: u32,
    y_end: u32,
) {
    let x_offset = alignment_offset(x_factor, x_alignment) as usize * rgba8::RGBA8_CHANNELS;
    let y_offset = alignment_offset(y_factor, y_alignment);
    let x_step = x_factor as usize * rgba8::RGBA8_CHANNELS;
    let source_row_len = rgba8::packed_row_byte_len(source_dimensions);
    let output_row_len = rgba8::packed_row_byte_len(output_dimensions);

    for output_y in y_start..y_end {
        let source_y = output_y * y_factor + y_offset;
        let source_row_start = source_y as usize * source_row_len;
        let local_y = output_y - y_start;
        let output_row_start = local_y as usize * output_row_len;

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
