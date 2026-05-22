//! Scalar production resize kernels.
//!
//! Scalar kernels are the first production target for benchmark-driven work.

// Cross-filter perf-search dependency map, current-code pass only:
// represented shared-layout benchmarks -> narrow common packed-RGBA8 APIs ->
// exact integer/identity path sharing -> shared word-copy primitives.
// NOTE(perf): Cross-filter `accepted` baselines exist for area, nearest, and
// bilinear, so shared packed-RGBA8 experiments can be judged against the three
// production filters instead of one local benchmark.
// REJECT(perf): A shared same-size packed RGBA8 copy helper for area and
// bilinear preserved correctness but had no performance upside over local
// `copy_from_slice` identity branches. Full `--scales 1 --baseline accepted`
// runs showed equivalent raw means (~35µs selfie, ~0.96-0.99ms box); keep the
// inline copy shape and do not re-open nearest identity without a represented 1x
// nearest profile.
// TODO(perf:api, rank=3, after perf:harness cross-filter-exact-coverage): Test
// a shared exact-integer scale classifier for area and nearest so exact up/down
// factor detection has one branch shape. This is plan/dispatch-level work, so
// accept only if `run area --scales 0.5,0.25,2,4 --baseline accepted` and `run
// nearest --scales 0.5,0.25,2,4 --baseline accepted` show no codegen loss.
// TODO(perf:kernel, rank=4, after perf:api shared-exact-scale-classifier): Test
// promoting nearest/common unaligned RGBA8 word read/write into shared packed
// primitives. Keep bilinear out of this experiment because packed tap reads are
// already rejected there; judge nearest exact/generic paths plus area exact
// upscales with `run nearest --baseline accepted` and `run area --scales 2,4
// --baseline accepted`.
// NOTE(perf): Do not merge area and bilinear weighted support plans just because
// both cache per-axis source weights. Area deliberately uses f32 bounded drift,
// bilinear is exact f64, and both have local REJECT notes for changed
// normalization/evaluation order.

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
// DEFER(perf): A separable two-pass minify path changes f64 grouping relative
// to the exact direct 2D tap order, and the exact profile has already rejected
// denominator pre-sums for byte drift. Revisit only with a bounded/fast bilinear
// profile or a separable design that demonstrably preserves exact operation
// order before benchmarking minify scales.
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
// DEFER(perf): Pre-rewrite parity needs the old scalar/tiled implementation
// registered as separate benchmark subjects or archived result import support.
// The current harness can compare against spec and accepted baselines, but not
// directly execute `crates/ditherette-wasm-old` subjects in `run bilinear`.
// NOTE(perf): The `bilinear` profile includes the old shootout's 0.875x and
// 1.8x scales through the `bilinear-prior-art` scale group, so prior-art
// comparisons cover the same important ratios.
// DEFER(perf): Native row-band/tiled bilinear should be a separate subject and
// profile, not folded into scalar. Register it only when the production tiling
// module owns row-band execution and can be accepted independently from scalar.
// DEFER(perf): The old normalized `AxisContribution { first, weights }` plan is
// not a drop-in replacement for the current exact unnormalized tap order. Exact
// denominator pre-sums, packed reads, fixed two-tap loops, and channel unrolling
// all regressed or failed, so prototype old contribution layouts only under a
// new candidate subject/profile rather than perturbing accepted scalar.
// DEFER(perf): Vertical-first separable scratch rows depend on the deferred old
// contribution layout and change f64 grouping. Revisit with a candidate subject
// and exactness proof, or move it to bounded/fast bilinear if bytes differ.
// DEFER(perf): Row-band cutoff sweeps require the deferred tiled bilinear
// subject; scalar perf-loop cannot judge dynamic tiling thresholds.
// DEFER(perf): Old unrolled separable RGBA loops depend on a chosen separable
// scratch layout. The direct-kernel unroll was exact but slower, so do not port
// old unrolls until the surrounding separable kernel exists.
// DEFER(perf): Old f32 contribution math belongs in a bounded/fast bilinear
// profile. The exact `bilinear` profile should keep f64 tap math unless a future
// experiment proves byte-identical output.

pub mod area;
pub mod bilinear;
pub mod nearest;
