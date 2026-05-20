//! Scalar production nearest-neighbor resize.
//!
//! This starts as an intentionally duplicated implementation of nearest resize
//! semantics. It does not import spec code; future perf work can precompute maps
//! or specialize loops while preserving oracle compatibility.

use crate::{
    image::{ImageFormat, ImageView, ImageViewMut},
    prod::resize::common::alignment::{map_axis_coordinate, ResizeAnchor},
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

    // TODO(perf:layout, rank=3, after perf:layout nearest-axis-map): Precompute
    // x byte offsets once per resize so the inner loop loads offsets instead of
    // recomputing anchor math and `CHANNEL_COUNT` products. Benchmark with
    // `ditherette-bench run nearest --baseline perf-loop-nearest`.
    for output_y in 0..output_dimensions.height() {
        let source_y = map_axis_coordinate(
            output_y,
            source_dimensions.height(),
            output_dimensions.height(),
            y_alignment,
        );
        let source_row = source
            .row(source_y)
            .expect("mapped source y should stay in bounds");
        let output_row = output
            .row_mut(output_y)
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
        for output_x in 0..output_dimensions.width() {
            let source_x = map_axis_coordinate(
                output_x,
                source_dimensions.width(),
                output_dimensions.width(),
                x_alignment,
            );
            let source_start = source_x as usize * F::CHANNEL_COUNT;
            let output_start = output_x as usize * F::CHANNEL_COUNT;
            let source_pixel = &source_row[source_start..source_start + F::CHANNEL_COUNT];
            let output_pixel = &mut output_row[output_start..output_start + F::CHANNEL_COUNT];

            output_pixel.copy_from_slice(source_pixel);
        }
    }
}
