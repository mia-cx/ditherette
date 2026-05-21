//! Scalar production resize kernels.
//!
//! Scalar kernels are the first production target for benchmark-driven work.

// Bilinear perf-search dependency map, current-code pass only:
// harness baseline/profile coverage -> reusable plan/API -> scale-class path
// split -> packed kernels -> arithmetic micro-tuning.
// TODO(perf:harness, rank=1): Capture a clean `resize-bilinear` accepted
// baseline with exact oracle before optimizing so every later perf-loop can
// compare against a stable production copy. Verify with `ditherette-bench run
// resize-bilinear --oracle spec:resize:bilinear:scalar --baseline accepted` and
// replace only with the existing `accepted` baseline when accepted.
// TODO(perf:harness, rank=2): Add an explicit identity/same-size bilinear case
// to the benchmark matrix before adding copy fast paths, because the default
// partial scale group has near-identity 0.99/1.05 cases but no exact 1.0 case.
// Judge with `ditherette-bench run resize-bilinear --scales 1 --baseline
// accepted` plus exact oracle output.
// TODO(perf:api, rank=3): Introduce `BilinearResizePlan` and a
// `resize_bilinear_rgba8_with_plan_into` entry point so bench subjects and
// repeated callers can own source/output/anchor metadata instead of rebuilding
// coordinate state per invocation. Benchmark with `ditherette-bench run
// resize-bilinear --baseline accepted`.
// TODO(perf:layout, rank=4, after perf:api bilinear-resize-plan): Cache x/y
// source positions, support ranges, clamped source indices, and nonzero
// triangle weights in the plan so the kernel stops recomputing identical x work
// for every row and identical y work for every column. Verify exactness with
// `--oracle spec:resize:bilinear:scalar`, then benchmark `run resize-bilinear`.
// TODO(perf:layout, rank=5, after perf:api bilinear-resize-plan): Reuse a
// thread-local prod bilinear plan in `bench_subjects` like nearest/area so the
// harness measures kernel cost rather than setup cost. Benchmark with
// `ditherette-bench run resize-bilinear --baseline accepted`.
// TODO(perf:path, rank=6, after perf:layout bilinear-plan-weights): Split
// identity, upscale/two-tap, near-identity, and minify support-width classes so
// each class can use simpler loops without per-pixel support branching. Verify
// exact oracle checks across the default partial scale group, then benchmark
// `ditherette-bench run resize-bilinear --baseline accepted`.
// TODO(perf:path, rank=7, after perf:path bilinear-scale-classes): Add a packed
// identity/same-size row-copy path if the explicit 1.0 profile proves
// user-representative. Verify exactness with `ditherette-bench run
// resize-bilinear --scales 1 --oracle spec:resize:bilinear:scalar`.
// TODO(perf:path, rank=8, after perf:path bilinear-scale-classes): Evaluate a
// separable two-pass minify path with f64 scratch rows for large downscales;
// accept only if it remains byte-exact against `spec:resize:bilinear:scalar`
// and wins `ditherette-bench run resize-bilinear --scales 0.1,0.125,0.25,0.5
// --baseline accepted`.
// TODO(perf:kernel, rank=9, after perf:layout bilinear-plan-weights): Rewrite
// the packed kernel to index `source.data()` directly with row byte offsets,
// avoiding per-tap `ImageView::row` lookups after the packed RGBA8 boundary has
// already validated layout. Benchmark `ditherette-bench run resize-bilinear
// --baseline accepted` with exact oracle checks.
// TODO(perf:kernel, rank=10, after perf:path bilinear-scale-classes): Specialize
// the upscale/two-tap class into a fixed 2x2 contribution loop with no support
// iterator, zero-weight branch, or dynamic normalization branch. Benchmark
// `ditherette-bench run resize-bilinear --scales 1.05,1.5,2 --baseline
// accepted` against the exact oracle.
// TODO(perf:kernel, rank=11, after perf:layout bilinear-plan-weights): Remove
// zero-weight endpoint visits by storing only nonzero support entries in the
// plan; this should delete the hot `if weight == 0.0` branches. Benchmark the
// default `resize-bilinear` profile with exact oracle verification.
// TODO(perf:kernel, rank=12, after perf:kernel direct-packed-indexing): Unroll
// RGBA accumulation and final rounding in the packed kernel to remove the inner
// channel loop while preserving f64 exactness. Benchmark `ditherette-bench run
// resize-bilinear --baseline accepted`.
// TODO(perf:micro, rank=13, after perf:layout bilinear-plan-weights): Test
// precomputed f64 reciprocals for coordinate and weight scaling only after plan
// exactness is locked, because changed f64 operation order may break the exact
// oracle. Benchmark `ditherette-bench run resize-bilinear --baseline accepted`
// and reject on any byte mismatch.

pub mod area;
pub mod nearest;
