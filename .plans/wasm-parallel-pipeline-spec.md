# Wasm pipeline, tiling, and parallelization spec

## Status

Drafted from the design grilling session. This supersedes the earlier one-shot tiling notes and should be treated as the working spec for the Wasm pipeline + future parallelization work.

## Core direction

Ditherette's production image processing target is browser Wasm only. Native Rust is useful for unit tests and exploratory benches, but production behavior must be designed around browser constraints.

The architecture should land in two phases:

1. **Scalar Wasm pipeline MVP**
   - Wire prod scalar kernels through wasm-bindgen.
   - Establish staged internal exports and a full end-to-end `processRgba8` export.
   - Accept `parallelizationPolicy: boolean`, but initially both true/false may execute scalar.

2. **Wasm-thread parallelization PoC**
   - Prototype a Wasm thread pool, likely bootstrapped from JS via wasm-bindgen/Rayon-style tooling.
   - Rust/Wasm owns scheduling, policy, plans, kernels, and row-band execution after pool initialization.
   - First parallel kernel: color-space materialization.
   - Then nearest resize as overhead floor.
   - Then convolution-backed resize as the compute-heavy resize proof.

No SharedArrayBuffer / no threaded Wasm support means scalar-only production behavior.

## Public API shape

### External control

JS should not choose worker counts, band heights, plan scopes, or filter-specific concurrency. It should only indicate whether internal parallelization policy is allowed.

Use a simple boolean:

```ts
parallelizationPolicy: boolean
```

Meaning:

- `false`: force scalar/no parallel policy.
- `true`: allow Wasm to use its internal policy.

Important: `parallelizationPolicy: true` does **not** force tiling. If policy says nearest is faster scalar, it runs scalar.

### Staged exports

Expose generic staged functions for internal UI/prototype/bench use. These are not the final product surface, but they support lazy materialization and memoization.

First staged export should be color-space materialization:

```ts
convertColorSpace(
  input: Uint8Array,
  width: number,
  height: number,
  from: string,
  to: string,
  parallelizationPolicy: boolean,
): Float32Array | Uint8Array
```

Rules:

- Generic string enums are preferred for JS ergonomics.
- Output format is defined by the target color space.
- Float color spaces return `Float32Array`.
- U8 color spaces return `Uint8Array`.
- This is not a visual preview API and does not need roundtrip conversion to RGBA8.
- Use prod kernels, not spec oracle kernels.

Later staged exports should cover resize and other stages:

```ts
resizeRgba8(...): Uint8Array
quantize(...): Uint8Array | { indices: Uint8Array, rgba: Uint8Array }
dither(...): Uint8Array
```

Exact signatures can be designed when each stage is wired.

### Full pipeline export

Production should eventually use one end-to-end Wasm call to avoid repeatedly crossing the JS/Wasm boundary:

```ts
processRgba8(
  input: Uint8Array,
  width: number,
  height: number,
  settings: ProcessSettings,
  parallelizationPolicy: boolean,
): Uint8Array
```

`settings` includes resize, color, perturbation, quantization, dithering, and output settings.

The final return is cleanest as a buffer for canvas/UI preview. Export workflows may later need richer outputs, such as actual indexed PNG generation.

### Error handling and validation

This is an internal app boundary, not a public SDK.

- Use tests to enforce API contracts.
- Validate enough at the boundary to make internal UI bugs debuggable, especially dimensions, enum strings, and buffer lengths.
- Throw for invalid JS-facing configuration.
- Do not defensive-program impossible internal states.
- Browser owns image decode; if a blob is not an image, decode fails before Wasm processing begins.

## Pipeline model

### Operation order

The pipeline is sequential by dependency. Different domains do not run concurrently.

Example:

```text
browser decode -> source RGBA8 -> resize -> color/cache -> perturb/dither/quantize -> final output
```

You cannot quantize pixels that have not been resized/materialized yet. One operation domain owns the worker pool at a time.

### Intra-operation parallelism

Parallelization happens inside one operation at a time. Policy can choose different execution strategies per operation/kernel:

- nearest resize may choose scalar
- color conversion may choose row bands
- quantization may choose row bands
- error diffusion initially chooses scalar/special path

Cancellation and progress reporting are future concerns, not part of the first threading prototype.

## Memory model

### Browser/Wasm boundary

- Browser owns decode.
- JS passes decoded RGBA8 bytes into Wasm memory.
- Wasm owns processing buffers and caches.
- JS receives final output bytes for canvas/export.
- Zero-copy browser decode into Wasm memory is not required for the first version.

### Source and working buffers

The source image must remain materialized as RGBA8/sRGB while the image is open. The current final output must also remain materialized while the image/app is open.

Use a predictable ping-pong model for operation execution:

```text
immutable source RGBA8
working buffer A
working buffer B
side caches / memoized materializations
```

Rules:

- No in-place transforms in v1.
- Source remains immutable because the UI needs to render the original/source preview.
- A/B buffers are execution scratch/working buffers, not cache identities by themselves.
- If an intermediate must be cached, promote it into cache ownership under a stage key.
- Do not keep every intermediate full buffer by default.

### Cache and memoization

Cache keys are hashes of the canonical serialized inputs/settings that produced a stage.

Do not rehash large pixel buffers for every slider tick. The original source gets a content/import key once. Downstream keys compose from parent keys plus stage settings.

Example:

```text
stage_key = hash({
  operation: "resize",
  input_key: source_key,
  source_dimensions,
  output_dimensions,
  filter,
  filter_params,
  version,
})
```

Rules:

- All relevant settings are part of cache keys.
- Cache lives in Wasm memory.
- JS holds opaque handles/IDs where needed.
- Wasm owns cache invalidation.
- Cache entries become valid only after a full stage completes.
- No partial tiled-output cache validity in the MVP.
- Never drop original source or current final output while the image/app is open.
- Under memory pressure, drop derived caches first:
  1. dither/quantize final intermediates/checkpoints
  2. color-space caches
  3. resized checkpoints
  4. retained worker scratch
  5. original source only when image/document closes

Memoization can branch. For example, after resize, multiple color-space materializations may coexist. If the user tweaks dither settings, the resize cache should remain valid.

## Color-space buffer format

The first staged/prototype export is color-space materialization.

Rules:

- RGBA8/sRGB is always materialized for source/current display.
- Resize operates on RGBA8 before color-space work.
- Other color-space buffers are materialized only on demand.
- Output format follows target color space.
- Float color spaces use normalized alpha.
- U8 color spaces use U8 alpha.
- First prototype format: AoS, 4 channels per pixel.

Channel conventions:

```text
linear-srgb-f32: r, g, b, alpha
oklab-f32:       l, a, b, alpha
oklch-f32:       l, c, h, alpha
cielab-f32:      l, a, b, alpha
cielch-f32:      l, c, h, alpha
ycbcr-f32:       y, cb, cr, alpha
```

AoS vs SoA is a later performance bench. SoA may win for SIMD-heavy color metrics, but AoS is simplest for the prototype.

## Tileability model

CPU parallelization uses full-width output row bands.

Generic tiling applies to kernels where output regions are independent:

- resize: yes; output pixels are independent, though source halos/taps may be needed
- color conversion: yes; 1:1 input/output pixels
- quantization: yes; usually 1:1 source/color-cache to output/index
- threshold/perturbation dither: yes; may need absolute coordinates
- error diffusion: no generic tiling initially

Error diffusion has causal neighboring-output/error-state dependencies. Row-stride padding or overlap does not make it an independent-output problem. Exact parallel error diffusion would require a specialized algorithm such as boundary handoff, wavefront processing, or approximation. Since parallelism must not change output, error diffusion opts out of generic row-band tiling for now.

Generic tiling owns output-region ownership only. Domain kernels own source dependencies, coordinate mapping, source halos, scratch layout, and filter semantics.

## Kernel row-range contract

Tileable kernels expose internal row-range entrypoints conceptually shaped like:

```rust
fn process_rows(
    input: &[Input],
    output_rows: &mut [Output],
    full_geometry: Geometry,
    row_range: Range<u32>,
    plan: &Plan,
    scratch: &mut WorkerScratch,
)
```

Exact Rust types may vary.

Contract:

- `output_rows` covers exactly `row_range` in the full output.
- `row_range` is absolute output coordinates.
- Kernel uses full-image coordinates/plans, not local band coordinates.
- Kernel writes only its assigned output rows.
- Kernel may read the full input buffer.
- Kernel may use private worker scratch.
- Kernel does not allocate/stage/copy output bands in the target path.

Row ranges should be represented as `Range<u32>` rather than `y_start + height` where possible.

Row-range APIs should be internal. The public/app API indicates whether parallelization policy is enabled; it does not expose banding details.

## Plans

A plan is immutable precomputed metadata for one operation shape/configuration. It is not output data, scratch, or worker state.

Examples:

- nearest resize: x source starts, y coordinates, exact scale factors
- convolution resize: x/y tap lists and weights
- quantization: palette converted to target color space, metric constants
- threshold dither: threshold matrix metadata / coordinate periods
- color conversion: maybe no plan, or transform constants

Rules:

- Generic tiling does not own plans.
- Kernel/domain code owns plans.
- Plans are full-image by default.
- Plans own precomputed offsets/coordinate metadata.
- Workers own scratch.
- Full-image plans are the starting point because they preserve exact coordinate semantics and avoid duplicate x metadata.

Plan granularity is benchmarkable:

```text
plan_scope = full_image | per_worker | per_band
```

The bench should record:

- plan build time
- execution time
- total time
- estimated plan bytes
- estimated scratch bytes
- correctness vs full scalar output

Default policy starts with full-image plans. Per-worker/per-band plans must prove a win without coordinate drift.

## Scratch memory

Scratch is private mutable temporary memory owned by each hot worker.

Rules:

- Scratch is per worker.
- Scratch persists across jobs to avoid allocation churn.
- Scratch retention is capped.
- Huge scratch buffers are dropped or shrunk after a job.
- Caps should be fixed constants or memory-pressure driven, not image-size driven.
- A giant image must not permanently inflate worker memory.
- Generic tiling does not know scratch types.
- Worker context exposes reusable scratch pads (`u8`, `f32`, `f64`, named slots, etc.).
- Kernels decide what scratch to request.
- Kernels may use multiple scratch buffers at once.
- Scratch is not an access-control mechanism; output ownership and kernel contract enforce correctness.

Benchmarks should report:

- cold allocation time
- warmed steady-state time
- peak scratch bytes
- retained scratch bytes after trim

Browser worker-owned scratch viability must be validated by prototype.

## Scheduling model

Use static row-band planning with a shared assignment queue.

Rules:

- Split output into deterministic contiguous full-width row bands.
- Each assignment is exactly one contiguous row range.
- Workers pull the next assignment from a queue until the operation completes.
- Correctness must not depend on completion order.
- No per-worker private queue or work stealing in v1.
- The shared queue is simpler and naturally handles small row-cost variance.
- Active worker count means pool workers only in browser production.
- The UI/caller thread does not participate.
- The JS/Wasm boundary is async; image processing must not block the UI thread.

A "job" means one submitted image-operation invocation, e.g. one resize of one source image into one output image. A pipeline consists of multiple jobs/stages run sequentially by domain.

## Worker architecture

Prototype wasm-thread/Rayon-style architecture first.

Expected shape:

```text
JS bootstraps Wasm + thread pool
JS calls high-level async Wasm operation
Rust/Wasm policy decides scalar vs row bands
Rust/Wasm schedules row bands internally on hot workers
Workers write directly into shared output memory
```

JS owns bootstrap/lifecycle because browser worker APIs live in JS. Rust/Wasm owns per-operation scheduling if the threading runtime supports it.

Fallback if wasm-thread prototype fails:

```text
JS owns worker pool/coordinator
Workers call Wasm row-range exports
No SharedArrayBuffer => scalar only
```

This fallback is not the preferred path because it complicates memoization/plans across JS boundaries.

## Direct-write vs staged output

Target production mode is direct-write row bands:

```text
shared input read-only
shared output mutable, split into disjoint row ranges
worker writes assigned rows directly
```

Staged/stitch output is not the target. It may exist only as a diagnostic/fallback mode.

Staged/stitch drawbacks:

- per-band/per-worker output allocation
- full output copied again during stitch
- bad for memory-bandwidth-bound kernels
- exactly the overhead that made current tiling sweep pessimistic

Direct-write requires proving disjoint row slices. Use safe slicing where possible; isolate unsafe code in a small helper only if required by Rust/Rayon constraints.

## Tiling policy

Tiling policy is a mathematical function, not a lookup table.

Shape:

```text
policy(operation, kernel, input_dims, output_dims) -> Scalar | RowBands {
  worker_count,
  band_height,
  plan_scope,
  scratch_strategy?,
}
```

Inputs include actual dimensions, not scale alone:

```text
source_w, source_h, output_w, output_h, output_pixels,
scale_x, scale_y, minify/maxify factors, operation/kernel
```

Rules:

- Policy is per operation/kernel/filter/algorithm.
- Policy is global, not hardware-specific.
- Policy is bench-evidenced but may be hand/AI simplified to a smooth formula.
- Prefer smooth simple formulas over hyperlocalized optimization.
- Use gates sparingly: min, max, and maybe one proven inner band such as near-identity if it loses across both ARM64 and AMD64.
- If no policy applies, use scalar.
- Cache/materialization are outside tiling policy.
- Production does not run adaptive calibration or distributed benchmarks. That costs user compute/battery and can look untrustworthy.

Allowed formula families:

- log-linear
- S-curves/logistic
- parabolic
- exponential
- simple piecewise smooth
- splines only if simpler functions fail

Discrete outputs are rounded/clamped:

```text
if profitability_score(dims) <= threshold:
    Scalar
else:
    RowBands {
        worker_count = clamp(round(f_worker(dims)), 1, pool_size)
        band_height = clamp(round(f_band(dims)), min_band_height, output_h)
    }
```

Tie-breaking:

- Prefer fewer workers if performance is effectively identical.
- Prefer smoother functions over local optima.
- Prefer scalar unless tiled clears noise/practical thresholds.

Noise handling:

- Do not accept a tiled win just because median speedup is slightly above 1.
- Use uncertainty from repeated samples.
- A useful acceptance rule should consider lower confidence bound / variance and a practical margin.

Example practical rule for synthesis:

```text
accept tiled if speedup_median > 1.0
and lower_bound_speedup > 1.0
and speedup_margin > max(0.03, 2 * observed_noise)
```

Exact threshold can be tuned after browser/Wasm benchmark data exists.

## Benchmarking

The current tiling sweep is pessimistic because it measures thread spawn + per-band `Vec` allocation + copy-back. The new benchmark must measure the production model.

### Output artifact

Use one complete JSON artifact, not duplicated CSV/JSON outputs.

Recommended file:

```text
tiling-sweep-results.json
```

Contents:

- schema version
- git SHA
- machine metadata
- run configuration
- cases
- scalar samples and summary stats
- candidate samples and summary stats
- uncertainty fields
- policy accept/reject flags

Candidate summary fields:

```text
scalar_median_ns
scalar_stdev_ns
scalar_p05_ns
scalar_p95_ns
tiled_median_ns
tiled_stdev_ns
tiled_p05_ns
tiled_p95_ns
speedup_median
speedup_lower_bound
worker_efficiency
iterations_per_sample
accepted_win
```

Scalar should be periodically remeasured with the same warmup, target sample time, and sample size as tiled candidates.

Machine metadata:

```text
arch
os
cpu-ish if available
available_parallelism
wasm/native
browser/build profile
git_sha
```

Randomization:

- Candidate order can be randomized/blocked to avoid thermal/frequency drift bias.
- Keep deterministic seed in JSON.
- Pure data first; no plots required initially.

### Bench modes

Main policy sweep:

```text
scalar vs pooled_direct
```

Diagnostics:

- `pooled_noop`: executor overhead floor
- `pooled_copy`: memory bandwidth/direct row-write floor
- `plan_scope`: `full_image | per_worker | per_band`
- `spawn_staged`: temporary diagnostic for comparison with old data only

Do not explode the main matrix with every diagnostic axis.

### Cross-machine synthesis

Run browser/Wasm sweeps on:

- Apple Silicon M4
- Ryzen 9 7950X

Policy is synthesized to serve both well enough, not hyperoptimized for either.

An M4-derived policy may land as a PoC/dev policy first. Default production policy should wait for real browser/Wasm evidence across both architectures.

## Correctness requirements

Parallelism must not change output.

Required tests/proofs:

- row-band output coverage is complete and non-overlapping
- direct mutable row slicing rejects overlaps/gaps or makes them impossible
- row-band output equals scalar for exact kernels
- bounded kernels pass bounded row-vs-full checks
- cached plan dimensions/configs match the full operation
- invalid row-band dimensions fail consistently
- cache entries become valid only after complete operation
- no partial/corrupted output is cached on worker failure/cancel

Current resize row-range parity fixes are important precedent:

- nearest row ranges preserve scalar fast paths
- area row ranges preserve identity/exact paths
- convolution row ranges share full-image scale-aware path selection

## Implementation order

### 1. Scalar Wasm integration

- Export generic staged color-space materialization first.
- Use prod kernels.
- Return `Float32Array` or `Uint8Array` according to target color space.
- Accept `parallelizationPolicy: boolean`, but run scalar initially.
- Add wasm-bindgen tests where useful.

### 2. Generic resize staged export

- Expose generic `resizeRgba8` staged export.
- Use prod scalar resize filters.
- Return `Uint8Array` for UI/canvas preview.
- Keep `parallelizationPolicy` accepted but scalar until policy/threading exists.

### 3. Full pipeline export shell

- Add `processRgba8` with settings serialization.
- Initially execute scalar sequential stages.
- Keep buffers/caches inside Wasm.

### 4. Wasm-thread prototype

- Add thread-pool bootstrap through JS/tooling.
- Prototype color-space materialization with row bands.
- Rust/Wasm owns row-band scheduling internally.
- Workers write direct disjoint output rows.
- No per-band `Vec`, no copy-back.

### 5. Benchmark harness v2

- Browser/Wasm benchmark environment.
- JSON-only artifact.
- `scalar`, `pooled_direct`, `pooled_noop`, `pooled_copy`.
- Periodic scalar remeasurement.
- Plan granularity mini-bench.

### 6. Resize threading probes

- Nearest as overhead sentinel; expect scalar or highly gated policy.
- Bicubic/Lanczos convolution as compute-heavy resize proof.

### 7. Additional kernels

- Color conversion policies.
- Quantization policies.
- Threshold/perturbation dither policies.
- Error diffusion remains scalar/specialized.

### 8. Policy PoC and refinement

- Synthesize an M4-derived smooth policy as a PoC/dev flag.
- Run browser/Wasm sweeps on M4 + Ryzen.
- Refine global policy formulas.
- Production defaults use scalar fallback whenever policy evidence is absent or noisy.

## Open prototype questions

These should be answered by implementation spikes rather than speculation:

- Does wasm-bindgen/Rayon threading integrate cleanly with the app toolchain?
- Can the JS/Wasm API remain async and keep UI responsive?
- What is the exact shared-memory initialization story?
- Can worker-owned scratch persist cleanly in browser Wasm workers?
- Is AoS or SoA better for color-space and quantization SIMD paths?
- How expensive is plan construction at full-image vs per-worker vs per-band granularity?
- Which kernels actually clear noise thresholds under browser/Wasm `pooled_direct` execution?
