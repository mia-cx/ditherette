//! Development tripwires for the production packed-RGBA8 resize invariant.
//!
//! These helpers are not public input validation. Production resize is an
//! internal subsystem whose callers must normalize inputs before reaching prod
//! kernels. The debug assertions make architecture violations obvious during
//! development without adding release-mode hot-path checks.

use crate::image::{rgba8, ImageDimensions, ImageView, ImageViewMut, Rgba8};

/// Debug-assert that a production resize source view uses contiguous RGBA8 rows.
///
/// Use this at prod resize entrypoints while APIs still accept generic image
/// views. A failure means code crossed the prod boundary without normalization;
/// it is a development-time architecture bug, not recoverable user input.
pub fn assert_packed_source(source: ImageView<'_, Rgba8>, filter: &str) {
    debug_assert!(
        rgba8::is_packed_stride(source.dimensions(), source.stride()),
        "production {filter} resize requires packed RGBA8 source rows"
    );
}

/// Debug-assert that a production resize output view uses contiguous RGBA8 rows.
///
/// A packed output lets filters compute row offsets from dimensions alone.
/// Filters may optimize differently internally, but they all write the same
/// browser-`ImageData` byte layout: `R, G, B, A` repeated with no row padding.
pub fn assert_packed_output(output: &ImageViewMut<'_, Rgba8>, filter: &str) {
    debug_assert!(
        rgba8::is_packed_stride(output.dimensions(), output.stride()),
        "production {filter} resize requires packed RGBA8 output rows"
    );
}

/// Repeat each source RGBA8 pixel into an exact integer output block.
///
/// This is shared by filters whose exact integer upscale semantics collapse to
/// pixel replication. The caller owns deciding that the filter and anchor make
/// this exact for the requested resize; this helper only performs packed RGBA8
/// row expansion and row repetition.
pub(crate) fn resize_exact_pixel_repeat_into(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    output_dimensions: ImageDimensions,
    x_factor: usize,
    y_factor: usize,
) {
    let source_row_len = packed_row_byte_len(source_dimensions);
    let output_row_len = packed_row_byte_len(output_dimensions);
    let source_width = source_dimensions.width_usize();

    for source_y in 0..source_dimensions.height_usize() {
        let source_row_start = source_y * source_row_len;
        let output_row_start = source_y * y_factor * output_row_len;
        write_exact_pixel_repeat_row(
            source,
            source_row_start,
            source_width,
            output,
            output_row_start,
            x_factor,
        );

        for duplicate_y in 1..y_factor {
            copy_output_row(
                output,
                output_row_start,
                output_row_start + duplicate_y * output_row_len,
                output_row_len,
            );
        }
    }
}

fn packed_row_byte_len(dimensions: ImageDimensions) -> usize {
    dimensions.width_usize() * rgba8::RGBA8_CHANNELS
}

fn write_exact_pixel_repeat_row(
    source: &[u8],
    source_row_start: usize,
    source_width: usize,
    output: &mut [u8],
    output_row_start: usize,
    x_factor: usize,
) {
    match x_factor {
        2 => write_exact_2x_repeat_row(
            source,
            source_row_start,
            source_width,
            output,
            output_row_start,
        ),
        4 => write_exact_4x_repeat_row(
            source,
            source_row_start,
            source_width,
            output,
            output_row_start,
        ),
        _ => write_exact_nx_repeat_row(
            source,
            source_row_start,
            source_width,
            output,
            output_row_start,
            x_factor,
        ),
    }
}

fn write_exact_2x_repeat_row(
    source: &[u8],
    source_row_start: usize,
    source_width: usize,
    output: &mut [u8],
    output_row_start: usize,
) {
    let mut output_start = output_row_start;
    for source_x in 0..source_width {
        let pixel = u64::from(read_pixel_word(
            source,
            source_row_start + source_x * rgba8::RGBA8_CHANNELS,
        ));
        write_unaligned_word(output, output_start, pixel | (pixel << 32));
        output_start += rgba8::RGBA8_CHANNELS * 2;
    }
}

fn write_exact_4x_repeat_row(
    source: &[u8],
    source_row_start: usize,
    source_width: usize,
    output: &mut [u8],
    output_row_start: usize,
) {
    let mut output_start = output_row_start;
    for source_x in 0..source_width {
        let pixel = u128::from(read_pixel_word(
            source,
            source_row_start + source_x * rgba8::RGBA8_CHANNELS,
        ));
        write_unaligned_word(
            output,
            output_start,
            pixel | (pixel << 32) | (pixel << 64) | (pixel << 96),
        );
        output_start += rgba8::RGBA8_CHANNELS * 4;
    }
}

fn write_exact_nx_repeat_row(
    source: &[u8],
    source_row_start: usize,
    source_width: usize,
    output: &mut [u8],
    output_row_start: usize,
    x_factor: usize,
) {
    let mut output_start = output_row_start;
    for source_x in 0..source_width {
        let pixel = read_pixel_word(source, source_row_start + source_x * rgba8::RGBA8_CHANNELS);
        for _ in 0..x_factor {
            write_unaligned_word(output, output_start, pixel);
            output_start += rgba8::RGBA8_CHANNELS;
        }
    }
}

fn read_pixel_word(source: &[u8], source_start: usize) -> u32 {
    // SAFETY: Source offsets are derived from validated packed RGBA8 dimensions.
    // Unaligned access is intentional for byte-backed Wasm/browser image data.
    unsafe {
        source
            .as_ptr()
            .add(source_start)
            .cast::<u32>()
            .read_unaligned()
    }
}

fn write_unaligned_word<T: Copy>(output: &mut [u8], output_start: usize, word: T) {
    // SAFETY: Output offsets are derived from validated packed RGBA8 dimensions.
    // Unaligned access is intentional for byte-backed Wasm/browser image data.
    unsafe {
        output
            .as_mut_ptr()
            .add(output_start)
            .cast::<T>()
            .write_unaligned(word);
    }
}

fn copy_output_row(output: &mut [u8], from_start: usize, to_start: usize, byte_len: usize) {
    debug_assert!(from_start + byte_len <= output.len());
    debug_assert!(to_start + byte_len <= output.len());
    debug_assert!(from_start + byte_len <= to_start || to_start + byte_len <= from_start);

    // SAFETY: Exact pixel repeats copy between distinct output rows. Offsets
    // are computed from validated packed RGBA8 dimensions and the debug
    // assertions document the non-overlap precondition.
    unsafe {
        let source = output.as_ptr().add(from_start);
        let destination = output.as_mut_ptr().add(to_start);
        destination.copy_from_nonoverlapping(source, byte_len);
    }
}
