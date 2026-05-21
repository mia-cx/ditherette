//! Coverage math for scalar production area resize.
//!
//! Area resize is defined by source-space interval overlap. These helpers keep
//! rectangle math separate from RGBA8 row traversal so later optimization work
//! can precompute spans without changing the semantic rule.

// TODO(perf:layout, rank=4, after perf:api area-cached-plan): Introduce an
// `AreaResizePlan` that precomputes x/y overlap spans and normalized weights
// per output coordinate so the packed kernel stops rebuilding coverage ranges
// for every pixel. Start from the old `XCoverage`/`AxisCoverage` shape, but
// verify with `--oracle spec:resize:area:scalar`, then benchmark with
// `ditherette-bench run area --baseline perf-loop-resize-area`.
// TODO(perf:layout, rank=5, after perf:layout area-resize-plan): Store x spans
// with byte offsets and contiguous weights so each row can zip weights with
// `chunks_exact(RGBA8_CHANNELS)` like the old fractional-minify path. Benchmark
// 0.95x, 0.75x, 0.5x, and 0.125x with `ditherette-bench run area --baseline
// perf-loop-resize-area`.
// TODO(perf:layout, rank=6, after perf:layout area-resize-plan): Preserve
// total-weight accumulation order in the plan-driven kernel; old
// precomputed-total/reciprocal variants changed f64 rounding. Verify every
// plan layout with `ditherette-bench run area --oracle spec:resize:area:scalar
// --baseline perf-loop-resize-area`.

/// Source-space coverage for one output pixel.
#[derive(Debug, Clone, Copy)]
pub(super) struct OutputCoverage {
    pub(super) x_start: f64,
    pub(super) x_end: f64,
    pub(super) y_start: f64,
    pub(super) y_end: f64,
    pub(super) area: f64,
}

/// Build the source-space rectangle covered by one output pixel.
pub(super) fn output_coverage(
    output_x: u32,
    output_y: u32,
    x_scale: f64,
    y_scale: f64,
) -> OutputCoverage {
    let x_start = f64::from(output_x) * x_scale;
    let x_end = f64::from(output_x + 1) * x_scale;
    let y_start = f64::from(output_y) * y_scale;
    let y_end = f64::from(output_y + 1) * y_scale;

    OutputCoverage {
        x_start,
        x_end,
        y_start,
        y_end,
        area: x_scale * y_scale,
    }
}

/// Return how much two one-dimensional source-space intervals overlap.
pub(super) fn interval_overlap(a_start: f64, a_end: f64, b_start: f64, b_end: f64) -> f64 {
    (a_end.min(b_end) - a_start.max(b_start)).max(0.0)
}

/// Clamp an integer coordinate to a closed interval.
pub(super) fn clamp_i64(value: i64, min: i64, max: i64) -> i64 {
    value.clamp(min, max)
}
