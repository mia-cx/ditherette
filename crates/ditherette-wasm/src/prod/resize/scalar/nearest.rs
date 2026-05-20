//! Scalar production nearest-neighbor resize.
//!
//! This starts as an intentionally duplicated implementation of nearest resize
//! semantics. It does not import spec code; future perf work can precompute maps
//! or specialize loops while preserving oracle compatibility.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::common::alignment::{axis_coordinate_map, ResizeAnchor},
};

// TODO(perf:path, rank=2): Define nearest-paths by splitting prod nearest into
// identity, exact-ratio, and fractional resize paths before tuning row kernels;
// the generic anchor path may be hiding simpler copies. Verify with
// `ditherette-bench run nearest --oracle spec:resize:nearest:scalar`; benchmark
// with `ditherette-bench run nearest --baseline perf-loop-nearest` plus explicit
// `--scales 1,0.5,2`.
/// Resize `source` into `output` by copying the nearest source pixel.
pub fn resize_nearest_into<F: ImageFormat>(
    source: ImageView<'_, F>,
    mut output: ImageViewMut<'_, F>,
    anchor: ResizeAnchor,
) {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let (x_alignment, y_alignment) = anchor.axes();

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

        // TODO(perf:kernel, rank=4, after perf:path nearest-paths): Specialize
        // the hot RGBA8 path so four-channel copies use typed loads/stores or
        // chunked rows instead of per-pixel slice construction. Verify with
        // `--oracle spec:resize:nearest:scalar`; benchmark `ditherette-bench run
        // nearest --baseline perf-loop-nearest`.
        // TODO(perf:kernel, rank=6, after perf:path nearest-paths): Add an
        // upscale run-fill kernel that writes repeated destination spans from
        // one source pixel when adjacent output x coordinates map to the same
        // input. Benchmark with `ditherette-bench run nearest --scales 2,4
        // --baseline perf-loop-nearest-upscale`.
        for (output_x, source_start) in x_source_starts.iter().copied().enumerate() {
            let output_start = output_x * F::CHANNEL_COUNT;
            let source_pixel = &source_row[source_start..source_start + F::CHANNEL_COUNT];
            let output_pixel = &mut output_row[output_start..output_start + F::CHANNEL_COUNT];

            output_pixel.copy_from_slice(source_pixel);
        }
    }
}
