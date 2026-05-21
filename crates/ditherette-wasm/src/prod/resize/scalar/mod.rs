//! Scalar production resize kernels.
//!
//! Scalar kernels are the first production target for benchmark-driven work.

// Bilinear perf-search dependency map, current-code pass only:
// harness baseline/profile coverage -> reusable plan/API -> scale-class path
// split -> packed kernels -> arithmetic micro-tuning.
// NOTE(perf): The `bilinear` profile includes an explicit identity/same-size
// scale via the `identity` scale group, so later copy fast paths are measured by
// the default profile. Exact-oracle checks passed for the expanded matrix.
// NOTE(perf): `BilinearResizePlan` with cached direct-evaluated x/y source
// positions and thread-local bench reuse was accepted in `ditherette-bench run
// bilinear --baseline accepted`; it improved most default cases by ~5-14% while
// preserving exact oracle output.
// NOTE(perf): Cached nonzero support taps in `BilinearResizePlan` were accepted
// in `ditherette-bench run bilinear --baseline accepted`; preserving duplicate
// clamped edge taps kept exact output and improved default cases by ~50-160%.
// NOTE(perf): The identity/same-size class copies packed RGBA8 bytes directly;
// exact-oracle checks passed and `ditherette-bench run bilinear --scales 1
// --baseline accepted` improved identity cases by >99%.
// REJECT(perf): Splitting the remaining upscale/two-tap support class into a
// fixed 2x2 kernel preserved exactness but regressed every upscale case by about
// -52% to -55% in `ditherette-bench run bilinear --scales 1.05,1.5,2
// --baseline accepted`; keep the generic tap iterator until a different kernel
// shape is available.
// TODO(perf:path, rank=8, after perf:path bilinear-scale-classes): Evaluate a
// separable two-pass minify path with f64 scratch rows for large downscales;
// accept only if it remains byte-exact against `spec:resize:bilinear:scalar`
// and wins `ditherette-bench run bilinear --scales 0.1,0.125,0.25,0.5
// --baseline accepted`.
// REJECT(perf): Rewriting the packed kernel to index `source.data()` directly
// with row byte offsets preserved exactness but regressed representative
// large/upscale cases by roughly -5% to -19% in `ditherette-bench run bilinear
// --baseline accepted`; keep `ImageView::row` in the hot pixel writer.
// REJECT(perf): Specializing the upscale/two-tap class into a fixed 2x2
// contribution loop preserved exactness but regressed every upscale case by
// about -52% to -55% in `ditherette-bench run bilinear --scales 1.05,1.5,2
// --baseline accepted`.
// REJECT(perf): Unrolling RGBA accumulation and final rounding preserved exact
// output but broadly regressed non-identity cases in `ditherette-bench run
// bilinear --baseline accepted`; keep the small channel loops for optimizer/code
// layout until the surrounding kernel shape changes.
// CLOSE(perf): Precomputed f64 reciprocals for coordinate and weight scaling
// would only affect `BilinearResizePlan` construction; the benchmark subject now
// reuses that plan, so this micro-optimization is invisible to `run bilinear`
// unless plan setup itself becomes the measured target.
//
// Prior-art pass from `crates/ditherette-wasm-old`:
// - Old scalar bilinear used normalized per-axis contribution Vecs, trimmed zero
//   taps, a vertical-first separable pass, and a thread-local one-row f32 scratch.
// - Old tiling bilinear used row-band plans with scalar fallback below ~150k
//   output pixels, two-band mode below ~250k, and different band shapes for
//   minify vs near-source output sizes.
// - Closed old experiments: thread-local contribution Vec ownership, caller-owned
//   Bilinear2Plan, flat/SoA/inline contribution layouts, precomputed source byte
//   offsets, incremental coordinate updates, and horizontal-first minify either
//   regressed or changed exact output; only retest under materially changed exact
//   oracle/profile/layout conditions.
// TODO(perf:harness, rank=25): Add a pre-rewrite bilinear parity benchmark path
// or archived-comparison profile so perf-loop can measure current scalar against
// old scalar/tiled behavior before choosing kernels. Compare scales
// `2,1.8,1.5,0.99,0.95,0.875,0.75,0.5,0.25,0.125` with
// `ditherette-bench run bilinear --oracle spec:resize:bilinear:scalar`.
// NOTE(perf): The `bilinear` profile includes the old shootout's 0.875x and
// 1.8x scales through the `bilinear-prior-art` scale group, so prior-art
// comparisons cover the same important ratios.
// TODO(perf:harness, rank=27): Register a native row-band/tiled prod bilinear
// subject before porting old tiling thresholds, so scalar and tiled paths can be
// accepted independently. Judge with a dedicated `bilinear-tiling` profile
// plus exact oracle checks against `spec:resize:bilinear:scalar`.
// TODO(perf:layout, rank=28, after perf:api bilinear-resize-plan): Prototype the
// old normalized `AxisContribution { first, weights }` plan in f64 with zero-tap
// trimming, because the current direct 2D kernel recomputes ranges and weights
// per pixel. Retest is justified by the changed current layout; reject if
// `ditherette-bench run bilinear --baseline accepted` does not beat direct
// current code while staying byte-exact.
// TODO(perf:layout, rank=29, after perf:layout bilinear-axis-contributions):
// Compare the old vertical-first separable scratch row against the current direct
// 2D accumulation for all scale classes, not only large minify. Use f64 scratch
// first for exact oracle compatibility; benchmark `ditherette-bench run
// bilinear --baseline accepted`.
// TODO(perf:path, rank=30, after perf:harness bilinear-tiling-subject): Sweep old
// row-band cutoffs around 150k/250k output pixels and minify-vs-near-source band
// shapes in the new harness before hard-coding dynamic tiling. Accept only if the
// tiled subject wins default large fixtures without slowing scalar fallback.
// TODO(perf:kernel, rank=31, after perf:layout bilinear-separable-scratch):
// Port the old unrolled RGBA vertical and horizontal accumulation loops after the
// separable scratch shape is chosen; benchmark with exact oracle checks and
// compare against rank=12 direct-kernel unrolling.
// TODO(perf:micro, rank=32, after perf:layout bilinear-axis-contributions): Treat
// old f32 contribution math as a separate exactness experiment: run it only after
// f64 contribution parity is locked, verify against `spec:resize:bilinear:scalar`,
// and move any max-Δ1 wins to a bounded/fast-mode profile rather than exact
// `bilinear` if bytes differ.

pub mod area;
pub mod bilinear;
pub mod nearest;
