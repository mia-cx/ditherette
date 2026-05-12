use std::cell::RefCell;

#[cfg(feature = "tiling")]
use crate::resize::cpu_tiling::{process_row_bands_with_plan, RowBand, RowBandPlan};
use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::validate_resize_buffers,
};

/// Resizes into a caller-provided output buffer with the production nearest path.
///
/// After validating the boundary buffers once, this uses the fastest measured
/// strategy: identity and row-copy fast paths, exact integer downscale, gated
/// span-copy for wide contiguous source-x runs, then a precomputed-offset
/// fallback that copies each RGBA pixel as one unaligned 32-bit word. The
/// sampling rule remains the deterministic pixel-center mapping documented on
/// [`resize_rgba_nearest`].
pub fn resize_rgba_nearest_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    if source_dimensions == output_dimensions {
        output_rgba.copy_from_slice(source_rgba);
        return Ok(());
    }

    if source_dimensions.width() == output_dimensions.width() {
        copy_same_width_rows(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    #[cfg(feature = "tiling")]
    {
        crate::resize::tiling::nearest::resize_rgba_nearest_with_tiling_after_fast_paths(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
            crate::resize::cpu_tiling::DEFAULT_ROW_BAND_TILING,
            true,
        )
    }

    #[cfg(not(feature = "tiling"))]
    {
        resize_rgba_nearest_scalar_after_fast_paths(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )
    }
}

pub(crate) fn resize_rgba_nearest_scalar_after_fast_paths(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    if is_exact_integer_downscale(source_dimensions, output_dimensions) {
        resize_exact_integer_downscale_word(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    NEAREST_RESIZE_SCRATCH.with(|scratch| {
        let mut scratch = scratch.borrow_mut();

        if has_wide_source_x_spans(source_dimensions, output_dimensions)? {
            return resize_span_copy_into(
                source_rgba,
                source_dimensions,
                output_dimensions,
                output_rgba,
                &mut scratch.span_copy,
            );
        }

        // TODO(perf): Revisit SIMD once broader resize/dither kernels are in place.
        resize_precomputed_offsets_word_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
            &mut scratch.precomputed_offsets,
        )
    })
}

#[cfg(feature = "tiling")]
#[doc(hidden)]
pub fn resize_rgba_nearest_scalar_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    if source_dimensions == output_dimensions {
        output_rgba.copy_from_slice(source_rgba);
        return Ok(());
    }

    if source_dimensions.width() == output_dimensions.width() {
        copy_same_width_rows(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    resize_rgba_nearest_scalar_after_fast_paths(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

const MIN_SPAN_COPY_AVERAGE_PIXELS: usize = 8;

thread_local! {
    static NEAREST_RESIZE_SCRATCH: RefCell<NearestResizeScratch> = RefCell::default();
}

#[derive(Default)]
struct NearestResizeScratch {
    precomputed_offsets: PrecomputedOffsetScratch,
    span_copy: SpanCopyScratch,
}

#[derive(Default)]
pub(crate) struct PrecomputedOffsetScratch {
    source_dimensions: Option<ImageDimensions>,
    output_dimensions: Option<ImageDimensions>,
    source_row_byte_offsets: Vec<usize>,
    source_x_byte_offsets: Vec<usize>,
}

#[cfg(feature = "tiling")]
struct ExactIntegerDownscalePlan {
    source_row_byte_len: usize,
    output_width: usize,
    y_step: usize,
    first_source_x_byte_offset: usize,
    source_x_byte_step: usize,
}

#[derive(Default)]
pub(crate) struct SpanCopyScratch {
    source_dimensions: Option<ImageDimensions>,
    output_dimensions: Option<ImageDimensions>,
    source_row_byte_offsets: Vec<usize>,
    source_x_copy_spans: Vec<SourceXCopySpan>,
}

pub(crate) fn copy_same_width_rows(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    let row_byte_len = source_dimensions.width_usize()? * rgba::RGBA_CHANNEL_COUNT;
    let output_height = output_dimensions.height_usize()?;

    for output_y in 0..output_height {
        let source_y = map_output_coordinate(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
        );
        let source_offset = source_y * row_byte_len;
        let output_offset = output_y * row_byte_len;

        output_rgba[output_offset..output_offset + row_byte_len]
            .copy_from_slice(&source_rgba[source_offset..source_offset + row_byte_len]);
    }

    Ok(())
}

fn resize_exact_integer_downscale_word(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;
    let x_step = source_dimensions.width_usize()? / output_width;
    let y_step = source_dimensions.height_usize()? / output_height;
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;
    let first_source_x_byte_offset = x_step / 2 * rgba::RGBA_CHANNEL_COUNT;
    let source_x_byte_step = x_step * rgba::RGBA_CHANNEL_COUNT;

    for output_y in 0..output_height {
        let source_y = output_y * y_step + y_step / 2;
        let source_row_offset = source_y * source_row_byte_len;
        let output_row_offset = output_y * output_row_byte_len;
        let mut source_x_offset = first_source_x_byte_offset;
        let mut output_offset = output_row_offset;

        for _ in 0..output_width {
            copy_pixel_word(
                source_rgba,
                source_row_offset + source_x_offset,
                output_rgba,
                output_offset,
            );
            source_x_offset += source_x_byte_step;
            output_offset += rgba::RGBA_CHANNEL_COUNT;
        }
    }

    Ok(())
}

#[cfg(feature = "tiling")]
pub(crate) fn resize_exact_integer_downscale_tiled_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    tiling_plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let x_step = source_dimensions.width_usize()? / output_width;
    let y_step = source_dimensions.height_usize()? / output_dimensions.height_usize()?;
    let plan = ExactIntegerDownscalePlan {
        source_row_byte_len: source_width * rgba::RGBA_CHANNEL_COUNT,
        output_width,
        y_step,
        first_source_x_byte_offset: x_step / 2 * rgba::RGBA_CHANNEL_COUNT,
        source_x_byte_step: x_step * rgba::RGBA_CHANNEL_COUNT,
    };

    process_row_bands_with_plan(output_rgba, tiling_plan, |band, output_rows| {
        resize_exact_integer_downscale_rows_into(source_rgba, &plan, band, output_rows);
        Ok(())
    })
}

#[cfg(feature = "tiling")]
fn resize_exact_integer_downscale_rows_into(
    source_rgba: &[u8],
    plan: &ExactIntegerDownscalePlan,
    band: RowBand,
    output_rows: &mut [u8],
) {
    let output_row_byte_len = plan.output_width * rgba::RGBA_CHANNEL_COUNT;

    for output_y in band.output_y_start..band.output_y_end {
        let source_y = output_y * plan.y_step + plan.y_step / 2;
        let source_row_offset = source_y * plan.source_row_byte_len;
        let local_output_row_offset = (output_y - band.output_y_start) * output_row_byte_len;
        let mut source_x_offset = plan.first_source_x_byte_offset;
        let mut output_offset = local_output_row_offset;

        for _ in 0..plan.output_width {
            copy_pixel_word(
                source_rgba,
                source_row_offset + source_x_offset,
                output_rows,
                output_offset,
            );
            source_x_offset += plan.source_x_byte_step;
            output_offset += rgba::RGBA_CHANNEL_COUNT;
        }
    }
}

fn resize_precomputed_offsets_word_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    scratch: &mut PrecomputedOffsetScratch,
) -> Result<(), ProcessingError> {
    validate_resize_buffers(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )?;

    prepare_precomputed_offsets(source_dimensions, output_dimensions, scratch)?;

    let output_width = output_dimensions.width_usize()?;
    for (output_y, source_row_offset) in scratch.source_row_byte_offsets.iter().copied().enumerate()
    {
        let output_row_offset = output_y * output_width * rgba::RGBA_CHANNEL_COUNT;

        for (output_x, source_x_offset) in scratch.source_x_byte_offsets.iter().copied().enumerate()
        {
            copy_pixel_word(
                source_rgba,
                source_row_offset + source_x_offset,
                output_rgba,
                output_row_offset + output_x * rgba::RGBA_CHANNEL_COUNT,
            );
        }
    }

    Ok(())
}

#[cfg(feature = "tiling")]
pub(crate) fn resize_precomputed_offsets_tiled_into(
    source_rgba: &[u8],
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    scratch: &PrecomputedOffsetScratch,
    tiling_plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    let output_width = output_dimensions.width_usize()?;

    process_row_bands_with_plan(output_rgba, tiling_plan, |band, output_rows| {
        resize_precomputed_offset_rows_into(
            source_rgba,
            output_width,
            &scratch.source_row_byte_offsets,
            &scratch.source_x_byte_offsets,
            band,
            output_rows,
        );
        Ok(())
    })
}

#[cfg(feature = "tiling")]
fn resize_precomputed_offset_rows_into(
    source_rgba: &[u8],
    output_width: usize,
    source_row_byte_offsets: &[usize],
    source_x_byte_offsets: &[usize],
    band: RowBand,
    output_rows: &mut [u8],
) {
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;

    for (output_y, source_row_offset) in source_row_byte_offsets
        .iter()
        .copied()
        .enumerate()
        .take(band.output_y_end)
        .skip(band.output_y_start)
    {
        let local_output_row_offset = (output_y - band.output_y_start) * output_row_byte_len;

        for (output_x, source_x_offset) in source_x_byte_offsets.iter().copied().enumerate() {
            copy_pixel_word(
                source_rgba,
                source_row_offset + source_x_offset,
                output_rows,
                local_output_row_offset + output_x * rgba::RGBA_CHANNEL_COUNT,
            );
        }
    }
}

pub(crate) fn prepare_precomputed_offsets(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    scratch: &mut PrecomputedOffsetScratch,
) -> Result<(), ProcessingError> {
    if scratch.source_dimensions == Some(source_dimensions)
        && scratch.output_dimensions == Some(output_dimensions)
    {
        return Ok(());
    }

    let source_width = source_dimensions.width_usize()?;
    let output_width = output_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;

    scratch.source_x_byte_offsets.clear();
    scratch.source_x_byte_offsets.reserve(output_width);
    scratch
        .source_x_byte_offsets
        .extend((0..output_width).map(|output_x| {
            let source_x = map_output_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
            );
            source_x * rgba::RGBA_CHANNEL_COUNT
        }));

    scratch.source_row_byte_offsets.clear();
    scratch.source_row_byte_offsets.reserve(output_height);
    scratch
        .source_row_byte_offsets
        .extend((0..output_height).map(|output_y| {
            let source_y = map_output_coordinate(
                output_y,
                source_dimensions.height(),
                output_dimensions.height(),
            );
            source_y * source_width * rgba::RGBA_CHANNEL_COUNT
        }));

    scratch.source_dimensions = Some(source_dimensions);
    scratch.output_dimensions = Some(output_dimensions);

    Ok(())
}

fn resize_span_copy_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    scratch: &mut SpanCopyScratch,
) -> Result<(), ProcessingError> {
    prepare_span_copy_scratch(source_dimensions, output_dimensions, scratch)?;

    let output_row_byte_len = output_dimensions.width_usize()? * rgba::RGBA_CHANNEL_COUNT;
    let mut output_row_offset = 0;

    for source_row_offset in scratch.source_row_byte_offsets.iter().copied() {
        for span in &scratch.source_x_copy_spans {
            let source_start = source_row_offset + span.source_x_byte_offset;
            let output_start = output_row_offset + span.output_x_byte_offset;
            output_rgba[output_start..output_start + span.byte_len]
                .copy_from_slice(&source_rgba[source_start..source_start + span.byte_len]);
        }
        output_row_offset += output_row_byte_len;
    }

    Ok(())
}

#[cfg(feature = "tiling")]
pub(crate) fn resize_span_copy_tiled_into(
    source_rgba: &[u8],
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
    scratch: &SpanCopyScratch,
    tiling_plan: RowBandPlan,
) -> Result<(), ProcessingError> {
    let output_width = output_dimensions.width_usize()?;

    process_row_bands_with_plan(output_rgba, tiling_plan, |band, output_rows| {
        resize_span_copy_rows_into(source_rgba, output_width, scratch, band, output_rows);
        Ok(())
    })
}

#[cfg(feature = "tiling")]
fn resize_span_copy_rows_into(
    source_rgba: &[u8],
    output_width: usize,
    scratch: &SpanCopyScratch,
    band: RowBand,
    output_rows: &mut [u8],
) {
    let output_row_byte_len = output_width * rgba::RGBA_CHANNEL_COUNT;

    for (output_y, source_row_offset) in scratch
        .source_row_byte_offsets
        .iter()
        .copied()
        .enumerate()
        .take(band.output_y_end)
        .skip(band.output_y_start)
    {
        let local_output_row_offset = (output_y - band.output_y_start) * output_row_byte_len;
        for span in &scratch.source_x_copy_spans {
            let source_start = source_row_offset + span.source_x_byte_offset;
            let output_start = local_output_row_offset + span.output_x_byte_offset;
            output_rows[output_start..output_start + span.byte_len]
                .copy_from_slice(&source_rgba[source_start..source_start + span.byte_len]);
        }
    }
}

pub(crate) fn prepare_span_copy_scratch(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    scratch: &mut SpanCopyScratch,
) -> Result<(), ProcessingError> {
    if scratch.source_dimensions == Some(source_dimensions)
        && scratch.output_dimensions == Some(output_dimensions)
    {
        return Ok(());
    }

    prepare_source_row_byte_offsets(source_dimensions, output_dimensions, scratch)?;
    prepare_source_x_copy_spans(source_dimensions, output_dimensions, scratch)?;
    scratch.source_dimensions = Some(source_dimensions);
    scratch.output_dimensions = Some(output_dimensions);

    Ok(())
}

fn prepare_source_row_byte_offsets(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    scratch: &mut SpanCopyScratch,
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;

    scratch.source_row_byte_offsets.clear();
    scratch
        .source_row_byte_offsets
        .extend((0..output_height).map(|output_y| {
            let source_y = map_output_coordinate(
                output_y,
                source_dimensions.height(),
                output_dimensions.height(),
            );
            source_y * source_width * rgba::RGBA_CHANNEL_COUNT
        }));

    Ok(())
}

fn prepare_source_x_copy_spans(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    scratch: &mut SpanCopyScratch,
) -> Result<(), ProcessingError> {
    let output_width = output_dimensions.width_usize()?;
    scratch.source_x_copy_spans.clear();

    let mut span_output_start = 0;
    let mut span_source_start =
        map_output_coordinate(0, source_dimensions.width(), output_dimensions.width());
    let mut previous_source_x = span_source_start;

    for output_x in 1..output_width {
        let source_x = map_output_coordinate(
            output_x,
            source_dimensions.width(),
            output_dimensions.width(),
        );

        if source_x != previous_source_x + 1 {
            scratch.source_x_copy_spans.push(SourceXCopySpan::new(
                span_source_start,
                span_output_start,
                output_x - span_output_start,
            ));
            span_output_start = output_x;
            span_source_start = source_x;
        }

        previous_source_x = source_x;
    }

    scratch.source_x_copy_spans.push(SourceXCopySpan::new(
        span_source_start,
        span_output_start,
        output_width - span_output_start,
    ));

    Ok(())
}

fn is_span_copy_candidate(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    source_dimensions.width() > output_dimensions.width()
        && source_dimensions.height() > output_dimensions.height()
        && !is_exact_integer_downscale(source_dimensions, output_dimensions)
}

pub(crate) fn has_wide_source_x_spans(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<bool, ProcessingError> {
    if !is_span_copy_candidate(source_dimensions, output_dimensions) {
        return Ok(false);
    }

    let skipped_source_columns =
        source_dimensions.width_usize()? - output_dimensions.width_usize()?;
    let average_span_pixels = output_dimensions.width_usize()? / skipped_source_columns.max(1);

    Ok(average_span_pixels >= MIN_SPAN_COPY_AVERAGE_PIXELS)
}

#[derive(Clone, Copy, Debug)]
struct SourceXCopySpan {
    source_x_byte_offset: usize,
    output_x_byte_offset: usize,
    byte_len: usize,
}

impl SourceXCopySpan {
    fn new(source_x: usize, output_x: usize, pixel_len: usize) -> Self {
        Self {
            source_x_byte_offset: source_x * rgba::RGBA_CHANNEL_COUNT,
            output_x_byte_offset: output_x * rgba::RGBA_CHANNEL_COUNT,
            byte_len: pixel_len * rgba::RGBA_CHANNEL_COUNT,
        }
    }
}

pub(crate) fn is_exact_integer_downscale(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> bool {
    source_dimensions.width() > output_dimensions.width()
        && source_dimensions.height() > output_dimensions.height()
        && source_dimensions
            .width()
            .is_multiple_of(output_dimensions.width())
        && source_dimensions
            .height()
            .is_multiple_of(output_dimensions.height())
}

fn copy_pixel_word(
    source_rgba: &[u8],
    source_offset: usize,
    output_rgba: &mut [u8],
    output_offset: usize,
) {
    // SAFETY: Callers pass offsets produced from validated image dimensions and
    // in-bounds loop coordinates. `read_unaligned` and `write_unaligned` avoid
    // alignment requirements for byte-backed Wasm memory.
    unsafe {
        let source_ptr = source_rgba.as_ptr().add(source_offset).cast::<u32>();
        let output_ptr = output_rgba.as_mut_ptr().add(output_offset).cast::<u32>();
        output_ptr.write_unaligned(source_ptr.read_unaligned());
    }
}

pub(crate) fn map_output_coordinate(
    output_coordinate: usize,
    source_size: u32,
    output_size: u32,
) -> usize {
    // TODO(perf): For hot loops that cannot precompute offsets, use an
    // incremental Bresenham-style mapper to avoid per-pixel division.
    let mapped = (((2 * output_coordinate as u64 + 1) * u64::from(source_size))
        / (2 * u64::from(output_size))) as usize;

    mapped.min(source_size as usize - 1)
}

#[cfg(test)]
#[path = "nearest_tests.rs"]
mod nearest_tests;
