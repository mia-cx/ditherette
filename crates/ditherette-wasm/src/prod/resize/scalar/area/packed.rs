//! Packed RGBA8 scalar kernel for production area resize.
//!
//! This kernel assumes the shared production resize boundary has already
//! normalized source and output rows to packed RGBA8. It mirrors the spec area
//! coverage formula while keeping RGBA8-specific row traversal in one place.

use crate::image::{rgba8, ImageDimensions, ImageView, ImageViewMut, Rgba8};

use super::coverage::{clamp_i64, interval_overlap, output_coverage};

// TODO(perf:path, rank=7, after perf:layout area-resize-plan): Split identity
// resize into a direct packed-row copy before coverage planning. Benchmark the
// identity area case in `ditherette-bench run area --baseline
// perf-loop-resize-area`.
// TODO(perf:path, rank=8, after perf:layout area-resize-plan): Split exact
// integer upscales into pixel replication plus row repeat, matching the old area
// scalar strategy. Benchmark 2x and 4x upscale cases with `ditherette-bench run
// area --baseline perf-loop-resize-area`.
// TODO(perf:path, rank=9, after perf:layout area-resize-plan): Split exact 2x
// downscale into a local four-pixel average kernel before generic integer
// downscale. Benchmark 0.5x cases with `ditherette-bench run area --baseline
// perf-loop-resize-area`.
// TODO(perf:path, rank=10, after perf:path area-exact-2x): Split remaining exact
// integer downscales into a local uniform-weight block-average kernel; old
// shared extraction regressed, so keep the kernel local to area. Benchmark
// 0.25x and 0.125x cases with `ditherette-bench run area --baseline
// perf-loop-resize-area`.
// TODO(perf:path, rank=11, after perf:layout area-resize-plan): Route fractional
// minification through compact precomputed x/y coverage, using the old
// fractional-minify area-style path as the first candidate. Benchmark 0.95x,
// 0.9x, 0.75x, and 0.5x with `ditherette-bench run area --baseline
// perf-loop-resize-area`.
// TODO(perf:kernel, rank=12, after perf:path area-fractional-minify): Recreate
// old dynamic row-band tiling for large area outputs once scalar path classes
// are settled; use four-band vs near-source tiling choices as hypotheses, not
// constants. Benchmark with `ditherette-bench run area --baseline
// perf-loop-resize-area` and representative large fractional cases.
// TODO(perf:path, rank=13, after perf:layout area-resize-plan): Test a
// separable horizontal-then-vertical area path with reusable scratch rows only
// under the new packed-RGBA8/custom-harness conditions; older notes warned that
// separable/prefix/integral variants can alter rounding or lose to direct
// coverage. Verify with `--oracle spec:resize:area:scalar` and benchmark
// `ditherette-bench run area --baseline perf-loop-resize-area`.

pub(super) fn resize_into(source: ImageView<'_, Rgba8>, mut output: ImageViewMut<'_, Rgba8>) {
    let source_dimensions = source.dimensions();
    let output_dimensions = output.dimensions();
    let x_scale = f64::from(source_dimensions.width()) / f64::from(output_dimensions.width());
    let y_scale = f64::from(source_dimensions.height()) / f64::from(output_dimensions.height());

    for output_y in 0..output_dimensions.height() {
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");

        for output_x in 0..output_dimensions.width() {
            let coverage = output_coverage(output_x, output_y, x_scale, y_scale);
            let output_start = output_x as usize * rgba8::RGBA8_CHANNELS;
            let output_pixel = &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS];
            let accumulated = accumulate_pixel(source, source_dimensions, coverage);

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

// TODO(perf:kernel, rank=14, after perf:layout area-x-byte-spans): Drive this
// loop from precomputed overlap spans so hot pixels avoid `floor`, `ceil`,
// interval-overlap branches, and per-source-pixel coordinate clamps. Benchmark
// with `ditherette-bench run area --baseline perf-loop-resize-area`.
fn accumulate_pixel(
    source: ImageView<'_, Rgba8>,
    source_dimensions: ImageDimensions,
    coverage: super::coverage::OutputCoverage,
) -> [f64; rgba8::RGBA8_CHANNELS] {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];

    for source_y in coverage.y_start.floor() as i64..coverage.y_end.ceil() as i64 {
        let y_overlap = interval_overlap(
            coverage.y_start,
            coverage.y_end,
            source_y as f64,
            source_y as f64 + 1.0,
        );
        if y_overlap == 0.0 {
            continue;
        }

        let clamped_y = clamp_i64(source_y, 0, i64::from(source_dimensions.height()) - 1) as u32;
        let source_row = source
            .row(clamped_y)
            .expect("clamped source y should stay in bounds");

        for source_x in coverage.x_start.floor() as i64..coverage.x_end.ceil() as i64 {
            let x_overlap = interval_overlap(
                coverage.x_start,
                coverage.x_end,
                source_x as f64,
                source_x as f64 + 1.0,
            );
            if x_overlap == 0.0 {
                continue;
            }

            let clamped_x =
                clamp_i64(source_x, 0, i64::from(source_dimensions.width()) - 1) as usize;
            let weight = x_overlap * y_overlap / coverage.area;
            let source_start = clamped_x * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];

            // TODO(perf:micro, rank=15, after perf:kernel area-span-kernel):
            // Compare f32 accumulators only after the new custom harness and
            // span kernel are in place; old attempts changed rounding or lost,
            // so accept only if byte-identical to `spec:resize:area:scalar`
            // across `ditherette-bench run area --oracle spec:resize:area:scalar
            // --baseline perf-loop-resize-area`.
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    accumulated
}
