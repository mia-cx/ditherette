//! Packed RGBA8 scalar kernel for production area resize.
//!
//! This kernel assumes the shared production resize boundary has already
//! normalized source and output rows to packed RGBA8. It mirrors the spec area
//! coverage formula while keeping RGBA8-specific row traversal in one place.

use crate::image::{rgba8, ImageView, ImageViewMut, Rgba8};

use super::AreaResizePlan;

// TODO(perf:path, rank=7, after perf:layout area-resize-plan): Split identity
// resize into a direct packed-row copy before coverage planning. Benchmark the
// identity area case in `ditherette-bench run area --baseline
// accepted`.
// TODO(perf:path, rank=8, after perf:layout area-resize-plan): Split exact
// integer upscales into pixel replication plus row repeat, matching the old area
// scalar strategy. Benchmark 2x and 4x upscale cases with `ditherette-bench run
// area --baseline accepted`.
// TODO(perf:path, rank=9, after perf:layout area-resize-plan): Split exact 2x
// downscale into a local four-pixel average kernel before generic integer
// downscale. Benchmark 0.5x cases with `ditherette-bench run area --baseline
// accepted`.
// TODO(perf:path, rank=10, after perf:path area-exact-2x): Split remaining exact
// integer downscales into a local uniform-weight block-average kernel; old
// shared extraction regressed, so keep the kernel local to area. Benchmark
// 0.25x and 0.125x cases with `ditherette-bench run area --baseline
// accepted`.
// TODO(perf:path, rank=11, after perf:layout area-resize-plan): Route fractional
// minification through compact precomputed x/y coverage, using the old
// fractional-minify area-style path as the first candidate. Benchmark 0.95x,
// 0.9x, 0.75x, and 0.5x with `ditherette-bench run area --baseline
// accepted`.
// TODO(perf:kernel, rank=12, after perf:path area-fractional-minify): Recreate
// old dynamic row-band tiling for large area outputs once scalar path classes
// are settled; use four-band vs near-source tiling choices as hypotheses, not
// constants. Benchmark with `ditherette-bench run area --baseline
// accepted` and representative large fractional cases.
// TODO(perf:path, rank=13, after perf:layout area-resize-plan): Test a
// separable horizontal-then-vertical area path with reusable scratch rows only
// under the new packed-RGBA8/custom-harness conditions; older notes warned that
// separable/prefix/integral variants can alter rounding or lose to direct
// coverage. Verify with `--oracle spec:resize:area:scalar` and benchmark
// `ditherette-bench run area --baseline accepted`.

pub(super) fn resize_with_plan_into(
    source: ImageView<'_, Rgba8>,
    mut output: ImageViewMut<'_, Rgba8>,
    plan: &AreaResizePlan,
) {
    for output_y in 0..plan.output_dimensions.height() {
        let output_row = output
            .row_mut(output_y)
            .expect("output y from dimensions should stay in bounds");
        let y_spans = &plan.y_spans[output_y as usize];

        for output_x in 0..plan.output_dimensions.width() {
            let x_spans = &plan.x_spans[output_x as usize];
            let output_start = output_x as usize * rgba8::RGBA8_CHANNELS;
            let output_pixel = &mut output_row[output_start..output_start + rgba8::RGBA8_CHANNELS];
            let accumulated = accumulate_pixel(source, x_spans, y_spans, plan.area);

            for channel in 0..rgba8::RGBA8_CHANNELS {
                output_pixel[channel] = accumulated[channel].clamp(0.0, 255.0).round() as u8;
            }
        }
    }
}

// TODO(perf:kernel, rank=14, after perf:layout area-x-byte-spans): Drive this
// loop from precomputed overlap spans so hot pixels avoid `floor`, `ceil`,
// interval-overlap branches, and per-source-pixel coordinate clamps. Benchmark
// with `ditherette-bench run area --baseline accepted`.
fn accumulate_pixel(
    source: ImageView<'_, Rgba8>,
    x_spans: &[super::plan::AxisOverlap],
    y_spans: &[super::plan::AxisOverlap],
    area: f64,
) -> [f64; rgba8::RGBA8_CHANNELS] {
    let mut accumulated = [0.0; rgba8::RGBA8_CHANNELS];

    for y_span in y_spans {
        let source_row = source
            .row(y_span.source_index as u32)
            .expect("planned source y should stay in bounds");

        for x_span in x_spans {
            let weight = x_span.overlap * y_span.overlap / area;
            let source_start = x_span.source_index * rgba8::RGBA8_CHANNELS;
            let source_pixel = &source_row[source_start..source_start + rgba8::RGBA8_CHANNELS];

            // TODO(perf:micro, rank=15, after perf:kernel area-span-kernel):
            // Compare f32 accumulators only after the new custom harness and
            // span kernel are in place; old attempts changed rounding or lost,
            // so accept only if byte-identical to `spec:resize:area:scalar`
            // across `ditherette-bench run area --oracle spec:resize:area:scalar
            // --baseline accepted`.
            for channel in 0..rgba8::RGBA8_CHANNELS {
                accumulated[channel] += f64::from(source_pixel[channel]) * weight;
            }
        }
    }

    accumulated
}
