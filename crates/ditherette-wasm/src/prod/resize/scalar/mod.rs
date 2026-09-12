//! Scalar production resize kernels.
//!
//! Scalar kernels are the first production target for benchmark-driven work.

// Cross-filter perf-search dependency map, current-code pass only:
// represented shared-layout benchmarks -> narrow common packed-RGBA8 APIs ->
// exact integer/identity path sharing -> shared word-copy primitives.
// NOTE(perf): The manifest profiles include identity plus partial exact-integer
// down/up scales, so shared packed-RGBA8 experiments can be judged with
// `ditherette-bench run area`, `ditherette-bench run nearest`, and
// `ditherette-bench run bilinear` as configured in the manifest.
// REJECT(perf): A shared same-size packed RGBA8 copy helper for area, nearest,
// and bilinear preserved correctness, but even with forced inlining it regressed
// represented area cases by roughly -3% to -10% in `ditherette-bench run area`.
// Keep identity copy branches local to filters until a caller-level pass-through
// avoids output allocation entirely.
// REJECT(perf): A shared exact-integer scale classifier for area and nearest
// preserved correctness, but the area profile showed broad represented
// regressions around -3% to -12% even after forced inlining and single-dispatch
// classifier use. Keep exact scale classification local to each filter's branch
// shape.
// CLOSE(perf): Shared unaligned RGBA8 word read/write primitives depended on a
// shared exact-scale classifier to justify another common dispatch surface. That
// parent classifier regressed area and was rejected, while bilinear packed tap
// reads were already rejected, so leave nearest/common word helpers local.
// NOTE(perf): Do not merge area and bilinear weighted support plans just because
// both cache per-axis source weights. Area deliberately uses f32 bounded drift,
// bilinear is exact f64, and both have local REJECT notes for changed
// normalization/evaluation order.

// Bilinear perf-search dependency map, current-code pass only:
// harness baseline/profile coverage -> reusable plan/API -> scale-class path
// split -> packed kernels -> arithmetic micro-tuning.
// NOTE(perf): The `bilinear` profile includes an explicit identity/same-size
// scale via the `identity` scale group, so later copy fast paths are measured by
// the default profile. Bounded oracle checks pass for the expanded matrix.
// NOTE(perf): `BilinearResizePlan` with cached direct-evaluated x/y source
// positions and thread-local bench reuse was accepted in `ditherette-bench run
// bilinear`; it improved most default cases by ~5-14% while
// preserving exact oracle output.
// NOTE(perf): Cached nonzero support taps in `BilinearResizePlan` were accepted
// in `ditherette-bench run bilinear`; preserving duplicate clamped edge taps
// kept the bounded contract stable and improved default cases by ~50-160%.
// NOTE(perf): The identity/same-size class copies packed RGBA8 bytes directly;
// `ditherette-bench run bilinear` improved represented identity cases by >99%.
// REJECT(perf): Splitting the remaining upscale/two-tap support class into a
// fixed 2x2 kernel preserved exactness but regressed represented upscale cases
// by about -52% to -55% in `ditherette-bench run bilinear`; keep the generic tap
// iterator until a different kernel shape is available.
// NOTE(perf): The single production bilinear path now uses vertical-first
// separable f32 scratch rows under bounded correctness. This closes most of the
// old scalar gap without adding a second bilinear implementation.
// REJECT(perf): Rewriting the packed kernel to index `source.data()` directly
// with row byte offsets preserved exactness but regressed representative
// large/upscale cases by roughly -5% to -19% in `ditherette-bench run
// bilinear`; keep `ImageView::row` in the hot pixel writer.
// REJECT(perf): Specializing the upscale/two-tap class into a fixed 2x2
// contribution loop preserved exactness but regressed represented upscale cases
// by about -52% to -55% in `ditherette-bench run bilinear`.
// REJECT(perf): Unrolling RGBA accumulation and final rounding preserved exact
// output but broadly regressed non-identity cases in `ditherette-bench run
// bilinear`; keep the small channel loops for optimizer/code
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
// CLOSE(perf): Do not register or tune `crates/ditherette-wasm-old` as a
// benchmark subject. It is only a temporary rewrite source and will be removed
// after current prod parity.
// NOTE(perf): The `bilinear` profile includes the old shootout's 0.875x and
// 1.8x scales through the `bilinear-prior-art` scale group, so prior-art
// comparisons cover the same important ratios.
// DEFER(perf): Native row-band/tiled bilinear should be a separate subject and
// profile, not folded into scalar. Register it only when the production tiling
// module owns row-band execution and can be accepted independently from scalar.
// NOTE(perf): The old normalized `AxisContribution { first, weights }` plan was
// previously closed as a standalone exact-profile task. The bounded/separable
// contract materially changes that condition; current retest TODOs live with the
// bilinear plan/kernel code.
// DEFER(perf): Row-band cutoff sweeps require the deferred tiled bilinear
// subject; scalar perf-loop cannot judge dynamic tiling thresholds.
// REJECT(perf): Do not port old unrolled separable RGBA loops as-is. The
// current separable kernel keeps compact plan taps and measured better with the
// existing small RGBA statements than broader unroll rewrites.

pub mod area;
pub mod bicubic;
pub mod bilinear;
pub mod convolution;
pub mod lanczos;
pub mod nearest;
