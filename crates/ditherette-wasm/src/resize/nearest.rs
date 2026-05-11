use crate::{
    error::ProcessingError,
    image::{rgba, ImageDimensions},
    resize::buffers::{allocate_output_rgba, validate_resize_buffers},
};

/// Resizes a tightly packed RGBA image with nearest-neighbor sampling.
///
/// The mapping is pixel-center aligned:
/// `source_x = floor((output_x + 0.5) * source_width / output_width)`, with the
/// same rule for y. This matches the coordinate convention used by common image
/// resampling libraries while keeping the calculation integer-only at the Wasm
/// boundary.
pub fn resize_rgba_nearest(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_nearest_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

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

    // TODO(perf): Benchmark axis-specific fast paths for same-height resize and
    // pure horizontal downscale/upscale. The same-width row copy is valuable,
    // but common UI previews may hit one-axis changes too.
    if source_dimensions.width() == output_dimensions.width() {
        copy_same_width_rows(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    if is_exact_integer_downscale(source_dimensions, output_dimensions) {
        resize_exact_integer_downscale_word(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        )?;
        return Ok(());
    }

    // TODO(perf): Replace the static span-copy gate with measured thresholds by
    // scale/CPU target. Pixel-center mapping changed span distributions, so the
    // old average-span heuristic should be revalidated.
    if has_wide_source_x_spans(source_dimensions, output_dimensions)? {
        return resize_span_copy_into(
            source_rgba,
            source_dimensions,
            output_dimensions,
            output_rgba,
        );
    }

    // TODO(perf): Revisit unchecked offset helpers and SIMD once broader
    // resize/dither kernels are in place.
    resize_precomputed_offsets_word_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

/// Straightforward reference implementation used by tests and benchmarks.
#[doc(hidden)]
pub fn resize_rgba_nearest_reference(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<Vec<u8>, ProcessingError> {
    let mut output_rgba = allocate_output_rgba(source_rgba, source_dimensions, output_dimensions)?;
    resize_rgba_nearest_reference_into(
        source_rgba,
        source_dimensions,
        output_dimensions,
        &mut output_rgba,
    )?;
    Ok(output_rgba)
}

/// Allocation-free form of the straightforward reference implementation.
#[doc(hidden)]
pub fn resize_rgba_nearest_reference_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    write_reference_resize(
        source_rgba,
        source_dimensions,
        output_dimensions,
        output_rgba,
    )
}

fn write_reference_resize(
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

    for output_y in 0..output_height {
        let source_y = map_output_coordinate(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
        );

        for output_x in 0..output_width {
            let source_x = map_output_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
            );

            copy_pixel_bytes(
                source_rgba,
                rgba::pixel_byte_offset(source_width, source_x, source_y),
                output_rgba,
                rgba::pixel_byte_offset(output_width, output_x, output_y),
            );
        }
    }

    Ok(())
}

const MIN_SPAN_COPY_AVERAGE_PIXELS: usize = 8;

// TODO(perf): Thread a reusable scratch object through repeated nearest resizes
// so span/offset buffers do not allocate for every frame in interactive previews.
#[derive(Default)]
struct SpanCopyScratch {
    source_row_byte_offsets: Vec<usize>,
    source_x_copy_spans: Vec<SourceXCopySpan>,
}

fn copy_same_width_rows(
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

    // TODO(perf): Detect repeated source_y values during vertical upscales and
    // copy the already-written output row instead of re-reading the source row.
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

    // TODO(perf): Add specialized 2x/4x exact-downscale loops that unroll source
    // strides and output writes. Center sampling is now `step / 2`, so these are
    // simple fixed-offset copies.
    for output_y in 0..output_height {
        let source_y = output_y * y_step + y_step / 2;

        for output_x in 0..output_width {
            let source_x = output_x * x_step + x_step / 2;

            copy_pixel_word(
                source_rgba,
                rgba::pixel_byte_offset(source_width, source_x, source_y),
                output_rgba,
                rgba::pixel_byte_offset(output_width, output_x, output_y),
            );
        }
    }

    Ok(())
}

fn resize_precomputed_offsets_word_into(
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

    // TODO(perf): Reuse source_x/source_y offset buffers across calls via a
    // resize plan or scratch object; these allocations are pure setup overhead.
    let source_x_byte_offsets: Vec<usize> = (0..output_width)
        .map(|output_x| {
            let source_x = map_output_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
            );
            source_x * rgba::RGBA_CHANNEL_COUNT
        })
        .collect();

    let source_row_byte_offsets: Vec<usize> = (0..output_height)
        .map(|output_y| {
            let source_y = map_output_coordinate(
                output_y,
                source_dimensions.height(),
                output_dimensions.height(),
            );
            source_y * source_width * rgba::RGBA_CHANNEL_COUNT
        })
        .collect();

    for (output_y, source_row_offset) in source_row_byte_offsets.into_iter().enumerate() {
        let output_row_offset = output_y * output_width * rgba::RGBA_CHANNEL_COUNT;

        // TODO(perf): Write output offsets incrementally instead of recomputing
        // pixel_byte_offset for every pixel in the fallback path.
        for (output_x, source_x_offset) in source_x_byte_offsets.iter().copied().enumerate() {
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

fn resize_span_copy_into(
    source_rgba: &[u8],
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    output_rgba: &mut [u8],
) -> Result<(), ProcessingError> {
    let mut scratch = SpanCopyScratch::default();
    // TODO(perf): Avoid rebuilding span-copy scratch when the same source/output
    // dimensions are resized repeatedly; only source bytes change between frames.
    prepare_source_row_byte_offsets(source_dimensions, output_dimensions, &mut scratch)?;
    prepare_source_x_copy_spans(source_dimensions, output_dimensions, &mut scratch)?;

    let output_row_byte_len = output_dimensions.width_usize()? * rgba::RGBA_CHANNEL_COUNT;
    let mut output_row_offset = 0;

    for source_row_offset in scratch.source_row_byte_offsets.iter().copied() {
        // TODO(perf): For repeated source rows during vertical upscales, copy the
        // previous output row rather than repeating every span copy.
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

fn prepare_source_row_byte_offsets(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    scratch: &mut SpanCopyScratch,
) -> Result<(), ProcessingError> {
    let source_width = source_dimensions.width_usize()?;
    let output_height = output_dimensions.height_usize()?;

    scratch.source_row_byte_offsets.clear();
    scratch.source_row_byte_offsets.reserve(output_height);

    let source_height = u64::from(source_dimensions.height());
    let output_height_u64 = u64::from(output_dimensions.height());
    let denominator = 2 * output_height_u64;
    let step = 2 * source_height;
    let base_step = step / denominator;
    let step_remainder = step % denominator;
    let mut source_y = source_height / denominator;
    let mut remainder = source_height % denominator;
    let source_row_byte_len = source_width * rgba::RGBA_CHANNEL_COUNT;

    for _ in 0..output_height {
        let clamped_source_y = source_y.min(source_height - 1) as usize;
        scratch
            .source_row_byte_offsets
            .push(clamped_source_y * source_row_byte_len);

        source_y += base_step;
        remainder += step_remainder;
        if remainder >= denominator {
            source_y += 1;
            remainder -= denominator;
        }
    }

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

    // TODO(perf): Generate spans from rational breakpoints rather than mapping
    // every output_x. This would make large near-identity images cheaper to plan.
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

fn has_wide_source_x_spans(
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
) -> Result<bool, ProcessingError> {
    if !is_span_copy_candidate(source_dimensions, output_dimensions) {
        return Ok(false);
    }

    // TODO(perf): Replace average-span heuristic with direct planning-cost vs
    // copy-savings estimate using expected span count and output byte volume.
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

fn is_exact_integer_downscale(
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

fn copy_pixel_bytes(
    source_rgba: &[u8],
    source_offset: usize,
    output_rgba: &mut [u8],
    output_offset: usize,
) {
    output_rgba[output_offset..output_offset + rgba::RGBA_CHANNEL_COUNT]
        .copy_from_slice(&source_rgba[source_offset..source_offset + rgba::RGBA_CHANNEL_COUNT]);
}

// TODO(perf): Benchmark native-endian u32 copies against plain 4-byte slice
// copies on wasm-opt output; engine lowering may make one consistently faster.
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

fn map_output_coordinate(output_coordinate: usize, source_size: u32, output_size: u32) -> usize {
    // TODO(perf): For hot loops that cannot precompute offsets, use an
    // incremental Bresenham-style mapper to avoid per-pixel division.
    let mapped = (((2 * output_coordinate as u64 + 1) * u64::from(source_size))
        / (2 * u64::from(output_size))) as usize;

    mapped.min(source_size as usize - 1)
}

#[cfg(test)]
mod tests {
    use super::{
        is_exact_integer_downscale, map_output_coordinate, resize_rgba_nearest,
        resize_rgba_nearest_into, resize_rgba_nearest_reference, write_reference_resize,
    };
    use crate::image::ImageDimensions;

    #[test]
    fn maps_coordinates_with_pixel_center_alignment() {
        assert_eq!(map_output_coordinate(0, 4, 2), 1);
        assert_eq!(map_output_coordinate(1, 4, 2), 3);
        assert_eq!(map_output_coordinate(0, 2, 4), 0);
        assert_eq!(map_output_coordinate(3, 2, 4), 1);
    }

    #[test]
    fn detects_exact_integer_downscale() {
        assert!(is_exact_integer_downscale(
            dimensions(4, 4),
            dimensions(2, 2)
        ));
        assert!(!is_exact_integer_downscale(
            dimensions(4, 4),
            dimensions(3, 3)
        ));
        assert!(!is_exact_integer_downscale(
            dimensions(2, 2),
            dimensions(4, 4)
        ));
    }

    #[test]
    fn benchmark_variants_match_baseline_across_resize_shapes() {
        for (source_dimensions, output_dimensions) in [
            (dimensions(6, 4), dimensions(3, 2)),
            (dimensions(7, 5), dimensions(5, 3)),
            (dimensions(3, 2), dimensions(7, 5)),
            (dimensions(4, 5), dimensions(4, 3)),
            (dimensions(4, 3), dimensions(4, 3)),
            (dimensions(1, 1), dimensions(5, 4)),
        ] {
            assert_variants_match_baseline(source_dimensions, output_dimensions);
        }
    }

    fn assert_variants_match_baseline(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
    ) {
        let source_rgba = patterned_rgba(source_dimensions);
        let baseline =
            resize_rgba_nearest_reference(&source_rgba, source_dimensions, output_dimensions)
                .unwrap();

        assert_eq!(
            resize_rgba_nearest(&source_rgba, source_dimensions, output_dimensions).unwrap(),
            baseline
        );
        let mut output_rgba = vec![0xA5; baseline.len()];
        write_reference_resize(
            &source_rgba,
            source_dimensions,
            output_dimensions,
            &mut output_rgba,
        )
        .unwrap();
        assert_eq!(output_rgba, baseline);

        let mut output_rgba = vec![0xA5; baseline.len()];
        resize_rgba_nearest_into(
            &source_rgba,
            source_dimensions,
            output_dimensions,
            &mut output_rgba,
        )
        .unwrap();
        assert_eq!(output_rgba, baseline);
    }

    fn patterned_rgba(dimensions: ImageDimensions) -> Vec<u8> {
        let pixel_count = dimensions.width() as usize * dimensions.height() as usize;

        (0..pixel_count)
            .flat_map(|index| {
                let value = index as u8;
                [value, value.wrapping_mul(3), value.wrapping_mul(5), 255]
            })
            .collect()
    }

    fn dimensions(width: u32, height: u32) -> ImageDimensions {
        ImageDimensions::new(width, height).unwrap()
    }
}
