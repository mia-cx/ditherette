//! Scalar production area resize.
//!
//! This is the first production area kernel. It intentionally mirrors the spec
//! area algorithm while enforcing the production resize boundary: normalized
//! packed RGBA8 input and output. Later benchmark-driven work can precompute
//! spans, reuse accumulators, or split rows without changing the coverage rule.

mod coverage;
mod packed;

use crate::{
    image::{ImageView, ImageViewMut, Rgba8},
    prod::resize::common,
};

// TODO(perf:harness, rank=2): Add area
// fixture/scale groups that distinguish identity, exact integer downscale,
// exact integer upscale, near-source fractional minify, and large fractional
// minify; the old area code had separate winners for these classes. Benchmark
// with `ditherette-bench run area --oracle spec:resize:area:scalar --baseline
// perf-loop-resize-area` before accepting path splits.
// TODO(perf:api, rank=3): Add a cached
// `AreaResizePlan` entrypoint analogous to nearest so repeated Ditherette calls
// and the bench adapter can own plan reuse instead of rebuilding coverage data
// per image. Benchmark with `ditherette-bench run area --baseline
// perf-loop-resize-area`.

/// Resize packed RGBA8 `source` into packed RGBA8 `output` with exact area averaging.
///
/// Each output pixel covers a rectangle in source-pixel space. The output value
/// is the coverage-weighted average of every overlapped source pixel, rounded
/// per channel to `u8`. This function duplicates the spec formula instead of
/// importing it so production remains independent from the oracle.
pub fn resize_area_rgba8_into(source: ImageView<'_, Rgba8>, output: ImageViewMut<'_, Rgba8>) {
    common::rgba8::assert_packed_source(source, "area");
    common::rgba8::assert_packed_output(&output, "area");

    packed::resize_into(source, output);
}
