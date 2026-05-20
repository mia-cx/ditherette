//! Scalar production nearest-neighbor resize.
//!
//! This starts as an intentionally duplicated implementation of nearest resize
//! semantics. It does not import spec code; future perf work can precompute maps
//! or specialize loops while preserving oracle compatibility.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
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
/// Resize `source` into `output` by copying the nearest source pixel.
pub fn resize_nearest_into<F: ImageFormat>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
) {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let (x_alignment, y_alignment) = anchor.axes();

    if let Some((x_factor, y_factor)) = exact_downscale_factors(
        source_dimensions.width(),
        source_dimensions.height(),
        output_dimensions.width(),
        output_dimensions.height(),
    ) {
        resize_exact_downscale(source, output, x_factor, y_factor, x_alignment, y_alignment);
        return;
    }

    let x_source_starts = axis_coordinate_map(
        source_dimensions.width(),
        output_dimensions.width(),
        x_alignment,
    )
    .into_iter()
    .map(|source_x| source_x as usize * F::CHANNEL_COUNT)
    .collect::<Vec<_>>();
    let y_coordinates = axis_coordinate_map(
        source_dimensions.height(),
        output_dimensions.height(),
        y_alignment,
    );

    for (output_y, source_y) in y_coordinates.into_iter().enumerate() {
        let source_row = source
            .row(source_y)
            .expect("mapped source y should stay in bounds");
        let output_row = output
            .row_mut(output_y as u32)
            .expect("output y from dimensions should stay in bounds");

        // TODO(perf:kernel, rank=4): Specialize
        // the hot RGBA8 path so four-channel copies use typed loads/stores or
        // chunked rows instead of per-pixel slice construction. Verify with
        // `--oracle spec:resize:nearest:scalar`; benchmark `ditherette-bench run
        // nearest --baseline perf-loop-nearest`.
        // REJECT(perf): Generic exact-upscale run-fill regressed 2x by -21.03%
        // in `ditherette-bench run nearest --baseline accepted`; do not retry
        // without a 2x-specific strategy or different representative workload.
        for (output_x, source_start) in x_source_starts.iter().copied().enumerate() {
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
