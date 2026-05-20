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
// TODO(perf:path, rank=4, after perf:kernel rgba8-word-copy): Restore the old
// nearest span-copy path for near-1x downscales where source x coordinates form
// long contiguous runs. Keep the old average-span threshold as the first
// hypothesis; verify with `--oracle spec:resize:nearest:scalar` and benchmark
// `ditherette-bench run nearest --baseline accepted` on 0.85x-0.99x Celeste box
// art.

/// Reusable nearest-neighbor resize metadata for one source/output shape.
pub struct NearestResizePlan {
    source_dimensions: ImageDimensions,
    output_dimensions: ImageDimensions,
    anchor: ResizeAnchor,
    x_source_starts: Vec<usize>,
    y_coordinates: Vec<u32>,
    exact_downscale: Option<(u32, u32)>,
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

        Self {
            source_dimensions,
            output_dimensions,
            anchor,
            x_source_starts,
            y_coordinates,
            exact_downscale,
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

    let (x_alignment, y_alignment) = plan.anchor.axes();
    if let Some((x_factor, y_factor)) = plan.exact_downscale {
        resize_exact_downscale_rgba8(source, output, x_factor, y_factor, x_alignment, y_alignment);
        return;
    }

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
