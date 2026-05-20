//! Scalar production nearest-neighbor resize.
//!
//! This starts as an intentionally duplicated implementation of nearest resize
//! semantics. It does not import spec code; future perf work can precompute maps
//! or specialize loops while preserving oracle compatibility.

use crate::{
    image::{ImageDimensions, ImageFormat, ImageView, ImageViewMut, Rgba8},
    prod::resize::common::alignment::{axis_coordinate_map, AxisAlignment, ResizeAnchor},
};

// REJECT(perf): Adding an identity-only path was not represented in the default
// `nearest` profile and regressed/noised small cases by up to -9.67% in
// `ditherette-bench run nearest --baseline accepted`.
// REJECT(perf): A generic exact-upscale span-fill path regressed 2x by -21.03%
// in `ditherette-bench run nearest --baseline accepted`; wider upscale gains do
// not justify hurting the common 2x case.
// DEFER(perf): Fractional nearest path splits have no concrete benchmarkable
// shape yet; exact downscale is accepted, while identity-only and exact-upscale
// paths were rejected under the default `nearest` profile.
// REJECT(perf): Adding a same-width RGBA8 row-copy branch before the hot path
// found no represented 1x/same-width case in the current nearest profile and
// regressed most measured cases in `ditherette-bench run nearest --baseline
// accepted`; do not retry without a dedicated identity/same-width subject.
// TODO(perf:path, rank=2): Split the packed
// nearest dispatcher into measured scale classes (`exact-downscale`,
// `near-identity-downscale`, `other-downscale`, `upscale`) before adding more
// kernels. Hypothesis: old wins and losses are strongly scale-class dependent
// (`0.9x` current faster, `0.75x`/upscales old faster), so one fallback loop is
// leaving class-specific wins hidden. Benchmark with `ditherette-bench run
// nearest --baseline accepted` and compare against the old parity table in
// `crates/ditherette-bench/ARTIFACTS.md`.
// TODO(perf:layout, rank=3, after perf:path nearest-scale-classes): Replace
// per-output-row `u32` y coordinates in the packed path with byte row offsets
// and repeated-y run metadata. Hypothesis: upscales repeatedly revisit the same
// source rows, and old's cached row-byte offsets plus flat buffer addressing may
// explain part of its 36-43% large-upscale lead. Benchmark `ditherette-bench run
// nearest --baseline accepted`, focusing on 1.01x, 1.05x, 1.25x, 1.5x, 2x, and
// 4x cases.
// TODO(perf:path, rank=4, after perf:layout nearest-row-runs): Add an upscale
// row-repeat path that computes a source row once per y-run and copies/expands it
// into all repeated output rows. Hypothesis: this targets the remaining large
// upscale gap against old without retrying the rejected generic exact-upscale
// span-fill. Benchmark `ditherette-bench run nearest --baseline accepted`; reject
// if 2x regresses like the previous generic exact-upscale experiment.
// TODO(perf:kernel, rank=5, after perf:path nearest-scale-classes): Specialize
// the 0.75x/other-downscale packed kernel separately from near-identity span
// copy. Hypothesis: current span-copy wins at 0.9x but old is still 35-39%
// faster at 0.75x, so a mid-downscale kernel may need different span threshold
// or precomputed offset shape. Benchmark `ditherette-bench run nearest
// --baseline accepted`; do not lower the near-identity span threshold globally
// because the old crate already rejected that class of change.

/// Reusable nearest-neighbor resize metadata for one source/output shape.
pub struct NearestResizePlan {
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    anchor: ResizeAnchor,
    x_source_starts: Vec<usize>,
    y_coordinates: Vec<u32>,
    exact_downscale: Option<(u32, u32)>,
    source_x_copy_spans: Vec<SourceXCopySpan>,
}

#[derive(Debug, Clone, Copy)]
struct SourceXCopySpan {
    source_start: usize,
    output_start: usize,
    byte_len: usize,
}

impl NearestResizePlan {
    /// Builds reusable coordinate metadata for nearest-neighbor resize.
    pub fn new<F: ImageFormat>(
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> Self {
        let (x_alignment, y_alignment) = anchor.axes();
        let exact_downscale = exact_downscale_factors(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
        );
        let x_source_starts = if exact_downscale.is_some() {
            Vec::new()
        } else {
            axis_coordinate_map(
                source_dimensions.width(),
                output_dimensions.width(),
                x_alignment,
            )
            .into_iter()
            .map(|source_x| source_x as usize * F::CHANNEL_COUNT)
            .collect()
        };
        let y_coordinates = if exact_downscale.is_some() {
            Vec::new()
        } else {
            axis_coordinate_map(
                source_dimensions.height(),
                output_dimensions.height(),
                y_alignment,
            )
        };
        let source_x_copy_spans = source_x_copy_spans(
            source_dimensions.width(),
            source_dimensions.height(),
            output_dimensions.width(),
            output_dimensions.height(),
            &x_source_starts,
        );

        Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_source_starts,
            y_coordinates,
            exact_downscale,
            source_x_copy_spans,
        }
    }

    pub fn matches(
        &self,
        source_dimensions: ImageDimensions,
        output_dimensions: ImageDimensions,
        anchor: ResizeAnchor,
    ) -> bool {
        self.source_dimensions == source_dimensions
            && self.output_dimensions == output_dimensions
            && self.anchor == anchor
    }
}

/// Resize RGBA8 `source` into `output` by copying the nearest source pixel.
pub fn resize_nearest_rgba8_into(
    source: ImageView<'_, Rgba8>,
    output: ImageViewMut<'_, Rgba8>,
    anchor: ResizeAnchor,
) {
    let plan = NearestResizePlan::new::<Rgba8>(source.dimensions(), output.dimensions(), anchor);
    resize_nearest_rgba8_with_plan_into(source, output, &plan);
}

/// Resize RGBA8 `source` into `output` using precomputed nearest-neighbor metadata.
pub fn resize_nearest_rgba8_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &NearestResizePlan,
) {
    debug_assert_eq!(source.dimensions(), plan.source_dimensions);
    debug_assert_eq!(output.dimensions(), plan.output_dimensions);

    if is_packed_rgba8(source.dimensions(), source.stride().elements())
        && is_packed_rgba8(output.dimensions(), output.stride().elements())
    {
        resize_nearest_packed_rgba8_with_plan_into(
            source.data(),
            source.dimensions(),
            output.data_mut(),
            plan,
        );
        return;
    }

    let (x_alignment, y_alignment) = plan.anchor.axes();
    if let Some((x_factor, y_factor)) = plan.exact_downscale {
        resize_exact_downscale_rgba8(source, output, x_factor, y_factor, x_alignment, y_alignment);
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
                copy_rgba8_pixel_word(source_row, source_start, output_row, output_x * 4);
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

/// Resize `source` into `output` by copying the nearest source pixel.
pub fn resize_nearest_into<F: ImageFormat>(
    source: ImageView<'_, F>,
    output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
) {
    let plan = NearestResizePlan::new::<F>(source.dimensions(), output.dimensions(), anchor);
    resize_nearest_with_plan_into(source, output, &plan);
}

/// Resize `source` into `output` using precomputed nearest-neighbor metadata.
fn resize_nearest_with_plan_into<F: ImageFormat>(
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

const MIN_SPAN_COPY_AVERAGE_PIXELS: usize = 10;

fn is_packed_rgba8(dimensions: ImageDimensions, stride_elements: usize) -> bool {
    stride_elements == dimensions.width_usize() * 4
}

fn resize_nearest_packed_rgba8_with_plan_into(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    plan: &NearestResizePlan,
) {
    let (x_alignment, y_alignment) = plan.anchor.axes();
    if let Some((x_factor, y_factor)) = plan.exact_downscale {
        resize_exact_downscale_packed_rgba8(
            source,
            source_dimensions,
            output,
            plan.output_dimensions,
            x_factor,
            y_factor,
            x_alignment,
            y_alignment,
        );
        return;
    }

    let source_row_len = source_dimensions.width_usize() * 4;
    let output_row_len = plan.output_dimensions.width_usize() * 4;

    if plan.source_x_copy_spans.is_empty() {
        for (output_y, source_y) in plan.y_coordinates.iter().copied().enumerate() {
            let source_row_start = source_y as usize * source_row_len;
            let output_row_start = output_y * output_row_len;

            for (output_x, source_start) in plan.x_source_starts.iter().copied().enumerate() {
                copy_rgba8_pixel_word(
                    source,
                    source_row_start + source_start,
                    output,
                    output_row_start + output_x * 4,
                );
            }
        }
        return;
    }

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
fn resize_exact_downscale_packed_rgba8(
    source: &[u8],
    source_dimensions: ImageDimensions,
    output: &mut [u8],
    output_dimensions: ImageDimensions,
    x_factor: u32,
    y_factor: u32,
    x_alignment: AxisAlignment,
    y_alignment: AxisAlignment,
) {
    let x_offset = alignment_offset(x_factor, x_alignment) as usize * 4;
    let y_offset = alignment_offset(y_factor, y_alignment);
    let x_step = x_factor as usize * 4;
    let source_row_len = source_dimensions.width_usize() * 4;
    let output_row_len = output_dimensions.width_usize() * 4;

    for output_y in 0..output_dimensions.height_usize() {
        let source_y = output_y as u32 * y_factor + y_offset;
        let source_row_start = source_y as usize * source_row_len;
        let output_row_start = output_y * output_row_len;

        for output_x in 0..output_dimensions.width_usize() {
            let source_start = source_row_start + output_x * x_step + x_offset;
            let output_start = output_row_start + output_x * 4;
            copy_rgba8_pixel_word(source, source_start, output, output_start);
        }
    }
}

fn source_x_copy_spans(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
    x_source_starts: &[usize],
) -> Vec<SourceXCopySpan> {
    if source_width <= output_width || source_height <= output_height || x_source_starts.is_empty()
    {
        return Vec::new();
    }

    let skipped_source_columns = source_width as usize - output_width as usize;
    let average_span_pixels = output_width as usize / skipped_source_columns.max(1);
    if average_span_pixels < MIN_SPAN_COPY_AVERAGE_PIXELS {
        return Vec::new();
    }

    let mut spans = Vec::new();
    let mut span_output_start = 0;
    let mut span_source_start = x_source_starts[0];
    let mut previous_source_start = span_source_start;

    for (output_x, source_start) in x_source_starts.iter().copied().enumerate().skip(1) {
        if source_start != previous_source_start + 4 {
            spans.push(SourceXCopySpan {
                source_start: span_source_start,
                output_start: span_output_start * 4,
                byte_len: (output_x - span_output_start) * 4,
            });
            span_output_start = output_x;
            span_source_start = source_start;
        }
        previous_source_start = source_start;
    }

    spans.push(SourceXCopySpan {
        source_start: span_source_start,
        output_start: span_output_start * 4,
        byte_len: (x_source_starts.len() - span_output_start) * 4,
    });
    spans
}

fn exact_downscale_factors(
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Option<(u32, u32)> {
    if source_width <= output_width || source_height <= output_height {
        return None;
    }
    if source_width % output_width != 0 || source_height % output_height != 0 {
        return None;
    }

    Some((source_width / output_width, source_height / output_height))
}

fn resize_exact_downscale_rgba8(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    x_factor: u32,
    y_factor: u32,
    x_alignment: AxisAlignment,
    y_alignment: AxisAlignment,
) {
    let x_offset = alignment_offset(x_factor, x_alignment) as usize * 4;
    let y_offset = alignment_offset(y_factor, y_alignment);
    let x_step = x_factor as usize * 4;
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
            let output_start = output_x * 4;
            copy_rgba8_pixel_word(source_row, source_start, output_row, output_start);
        }
    }
}

fn copy_rgba8_pixel_word(
    source_row: &[u8],
    source_start: usize,
    output_row: &mut [u8],
    output_start: usize,
) {
    // SAFETY: Source and output offsets are derived from validated rows and
    // in-bounds nearest coordinate maps. Unaligned access is intentional for
    // packed byte-backed RGBA memory.
    unsafe {
        let source_ptr = source_row.as_ptr().add(source_start).cast::<u32>();
        let output_ptr = output_row.as_mut_ptr().add(output_start).cast::<u32>();
        output_ptr.write_unaligned(source_ptr.read_unaligned());
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

fn alignment_offset(factor: u32, alignment: AxisAlignment) -> u32 {
    match alignment {
        AxisAlignment::Start => 0,
        AxisAlignment::Center => factor / 2,
        AxisAlignment::End => factor - 1,
    }
}
