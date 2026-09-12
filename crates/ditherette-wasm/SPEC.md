# ditherette-wasm fresh-start spec

## Purpose

`ditherette-wasm` is the clean Rust/Wasm image-processing core for Ditherette.

The old prototype is preserved in `crates/ditherette-wasm-old`. This crate starts from a deliberately small surface so the API, correctness model, optimization boundaries, tiling model, and benchmark discipline can be specified before production code is ported or rewritten.

This document first specifies the resize architecture, but the same rules are intended to apply to color conversion, palette creation, quantization, dithering, and later pipeline execution.

## Current architectural phase

We are currently building only the executable specification layer plus the minimal image representation layer:

```text
src/
  image/  # representation/layout structs and validation
  spec/   # executable specification / oracle implementations
```

Do not add `src/prod/`, public resize barrels, or production dispatch layers during the spec phase. Benches and tests may deep-import spec modules directly. Production modules come later, after the spec contract for the operation is clear.

`spec/` is not a loose reference folder. It is the executable definition of what each operation means.

Future production code may reorganize computation, precompute plans, reuse scratch buffers, specialize kernels, and add tiled adapters, but it must preserve the behavior defined by `spec/` unless a public API explicitly declares a different tolerance/fast-mode contract.

## Directory structure

Current spec-phase target shape:

```text
src/
  lib.rs
  wasm.rs
  error.rs

  image/
    spec.md
    mod.rs
    dimensions.rs
    stride.rs
    view.rs
    owned.rs
    pixel.rs
    formats.rs
    validate.rs

  spec/
    mod.rs

    resize/
      spec.md
      mod.rs
      common/
        mod.rs
        alignment.rs
        alignment.md
        coordinates.rs
        coordinates.md
        sample.rs
        sample.md
      scalar/
        mod.rs
        nearest.rs
        nearest.md
        area.rs
        area.md
        bilinear.rs
        bilinear.md
        convolution.rs
        convolution.md
        bicubic.rs
        bicubic.md
        lanczos.rs
        lanczos.md
        trilinear.rs
        trilinear.md

    color/
      spec.md
      mod.rs
      spaces.rs
      spaces.md
      srgb.rs
      srgb.md
      linear.rs
      linear.md
      oklab.rs
      oklab.md
      oklch.rs
      oklch.md

    palette/
      spec.md
      mod.rs
      options.rs
      options.md
      sampling.rs
      sampling.md
      histogram.rs
      histogram.md
      select.rs
      select.md
      create.rs
      create.md

    quantize/
      spec.md
      mod.rs
      options.rs
      options.md
      nearest_color.rs
      nearest_color.md
      indexed.rs
      indexed.md

    dither/
      spec.md
      mod.rs
      options.rs
      options.md
      ordered.rs
      ordered.md
      error_diffusion.rs
      error_diffusion.md
      blue_noise.rs
      blue_noise.md

    tiling/
      spec.md
      mod.rs
      contract.rs
      contract.md

  pipeline/
    mod.rs
    stage.rs
    graph.rs
    cache.rs
    memo.rs
    executor.rs
```

### Future production boundary

The hard future boundary is not “resize vs color”; the hard future boundary is “semantic oracle vs optimized implementation.” We are not adding `src/prod/` during the spec phase, but when production code exists it should be top-level and unable to leak optimized helpers into spec code.

This avoids confusing patterns like:

```text
resize/shared/
resize/reference/shared/
resize/scalar/shared/
```

Future shape:

```text
spec/resize/scalar/convolution.rs      # readable oracle helper for spec bicubic/lanczos
prod/resize/scalar/convolution.rs      # optimized production convolution engine
prod/resize/common/*.rs               # production-only shared optimization helpers
```

## Import policies

These are hard rules.

### `spec/` may import

- `crate::image`
- `crate::error`
- sibling modules under `crate::spec`
- Rust standard library

### `spec/` must not import

- `crate::prod`
- `crate::tiling`
- production scratch/planning/common helpers
- benchmark helpers
- external image-processing crates as algorithm providers

### `prod/` may import

- `crate::image`
- `crate::error`
- its own `prod::<domain>::common` helpers
- generic `crate::tiling` infrastructure for tiled adapters

### `prod/` should avoid importing `spec/`

Production code should not depend on spec control flow. If a semantic constant/type is needed by both, prefer putting it in a neutral API/options module only if it is not algorithmic. When in doubt, duplicate the tiny semantic formula rather than weakening oracle independence.

### `tiling/` may import

- image dimensions / shape types
- `crate::error`

### `tiling/` must not import

- resize/color/palette/quantize/dither algorithms
- pixel-format-specific code
- domain-specific kernels

Top-level `tiling/` is scheduling infrastructure only. Domain-specific adapters live under `prod/<domain>/tiling/`.

## Image buffer abstraction

The detailed image module design lives in `src/image/spec.md`.

There is one minimal image-shaped buffer abstraction. It is generic over packed format markers and keeps storage as flat channel arrays.

```rust
pub struct ImageDimensions {
    pub width: u32,
    pub height: u32,
}

pub trait ImageFormat {
    type Storage;
    const CHANNEL_COUNT: usize;
    const NAME: &'static str;
}

pub struct ImageView<'a, F: ImageFormat> {
    pub data: &'a [F::Storage],
    pub dimensions: ImageDimensions,
    pub stride: RowStride, // storage elements, not pixels or bytes
}

pub struct ImageViewMut<'a, F: ImageFormat> {
    pub data: &'a mut [F::Storage],
    pub dimensions: ImageDimensions,
    pub stride: RowStride,
}

pub struct ImageBuf<F: ImageFormat> {
    pub data: Vec<F::Storage>,
    pub dimensions: ImageDimensions,
    pub stride: RowStride,
}
```

Format markers define flat-array layout contracts:

```rust
pub enum Rgba8 {}        // u8, 4 channels: R G B A
pub enum Rgb8 {}         // u8, 3 channels: R G B
pub enum LinearRgb32 {}  // f32, 3 channels: R G B
pub enum LinearRgba32 {} // f32, 4 channels: R G B A
pub enum Oklab32 {}      // f32, 3 channels: L a b
pub enum Oklaba32 {}     // f32, 4 channels: L a b alpha
pub enum PaletteIndex8 {}// u8, 1 channel: index
```

Canonical RGBA storage remains:

```text
[r, g, b, a, r, g, b, a, ...]
```

as:

```rust
ImageView<'_, Rgba8>
```

not `&[Rgba8PixelStruct]`. Internal Rust APIs should prefer typed format views like `ImageView<'_, Oklab32>` over runtime color-space enums. Runtime enums are acceptable at Wasm/API boundaries.

## Spec documentation policy

Spec documentation has two layers:

1. Every `src/spec/<domain>/` directory must include a module-level `spec.md`.
2. Every individual Rust module file under `src/spec/<domain>/` must have a sibling Markdown document with the same stem.

Examples:

```text
src/spec/resize/spec.md      # high-level resize design
src/spec/resize/scalar/nearest.rs   # executable nearest scalar spec
src/spec/resize/scalar/nearest.md   # nearest-specific design/correctness notes

src/spec/resize/scalar/convolution.rs
src/spec/resize/scalar/convolution.md

src/spec/palette/spec.md     # high-level palette design
src/spec/palette/histogram.rs
src/spec/palette/histogram.md
```

`mod.rs` is the only `.rs` exception. A directory may optionally include a `README.md` for navigation, but domain design belongs in `spec.md` and file-specific correctness/design documentation belongs beside each concrete `.rs` file.

### Domain-level `spec.md`

A domain-level `spec.md` explains how the whole spec module fits together. It should answer:

1. What the domain solves and what it does not solve.
2. What inputs and outputs the domain owns.
3. How the domain composes with other domains.
4. Which spec files exist and how they relate.
5. Domain-wide correctness philosophy.
6. Domain-wide edge cases and invariants.
7. Production obligations for the domain.
8. Deferred choices and non-goals.

Suggested template:

```markdown
# <domain> spec

## Purpose and scope

## Inputs and outputs

## Module map

## Domain model

## Correctness philosophy

## Edge cases and invariants

## Production obligations

## Non-goals / deferred choices
```

### File-level sibling `.md`

A file-level sibling `.md` explains one executable spec module before readers inspect code. It should answer:

1. What problem this file solves.
2. What inputs and outputs it owns.
3. Which algorithm or semantic rule it defines.
4. Why the spec implementation works the way it works.
5. Which edge cases define correctness.
6. Which invariants production must preserve.
7. Which choices are intentionally not optimized here.
8. How production implementations are expected to differ while remaining exact.

Suggested template:

```markdown
# <module> spec

## Purpose

## Inputs and outputs

## Algorithm / semantic rule

## Why this works this way

## Correctness invariants

## Edge cases

## Production obligations

## Non-goals
```

Markdown files are design documents. Sibling `.rs` files are executable documentation. They must stay aligned: if behavior changes in `src/spec/<domain>/*.rs`, update the matching file-level `.md`; if a domain-level assumption changes, update `src/spec/<domain>/spec.md` in the same change.

## Spec implementation policy

`spec/` implementations are executable documentation and correctness oracles.

They optimize for:

1. readability
2. auditability
3. stable semantics
4. byte-for-byte expected output

They do not optimize for speed.

### Spec implementations must be boring

Allowed:

- direct loops
- clear coordinate math
- explicit rounding
- explicit tie-breaking
- small local helpers
- comments explaining semantics

Forbidden:

- scratch reuse for speed
- thread-local buffers
- contribution-plan caches
- SIMD-oriented layouts
- unsafe code
- tiling
- Rayon or worker scheduling
- perf TODOs
- benchmark-driven experiments
- importing production helpers

### Spec code is self-documenting

Each operation should begin with a module doc comment explaining what behavior it defines.

Example:

```rust
//! Spec bilinear resize.
//!
//! This module defines the exact coordinate mapping, triangle weights,
//! normalization, accumulation order, and u8 rounding behavior that production
//! implementations must match byte-for-byte.
```

### Spec is deterministic

All spec algorithms must be deterministic. If an algorithm is normally randomized, the spec version requires an explicit seed and deterministic tie-breaking.

This matters for palette creation and quantization:

- same input + options => same palette
- same palette + input => same indexed output
- ties are resolved by documented order

### Spec may share spec-local helpers

Spec modules may share helpers inside `spec/<domain>/`.

Example:

```text
spec/resize/scalar/convolution.rs
spec/resize/scalar/bicubic.rs
spec/resize/scalar/lanczos.rs
```

This is acceptable because it stays inside the spec world. The helper must remain readable and semantic, not optimized.

### Spec and prod exactness

For exact modes:

```text
prod output must equal spec output byte-for-byte.
prod tiled output must equal spec output byte-for-byte.
```

If a future fast/tolerance mode intentionally differs, it must be a distinct public mode, option, or function name. It cannot silently replace an exact mode.

## Production implementation policy

Production implementations live under `prod/`.

They may:

- precompute weights/contributions
- reuse scratch buffers
- reorder computations if exact output is preserved
- specialize hot paths
- add scalar fast paths
- add tiled adapters
- use production-only common helpers

They must:

- pass spec conformance tests
- keep public wrappers thin
- isolate algorithm-specific optimization in the owning domain/filter
- record rejected performance ideas when useful

### Production common helpers

Production common helpers are allowed under:

```text
prod/<domain>/common/
```

Examples:

```text
prod/resize/common/contributions.rs
prod/resize/common/weights.rs
prod/resize/common/scratch.rs
```

Spec must never import these.

### Resize production sharing

Bicubic and Lanczos should share production separable resampling logic:

```text
prod/resize/scalar/convolution.rs
prod/resize/scalar/bicubic.rs
prod/resize/scalar/lanczos.rs
prod/resize/common/contributions.rs
```

Spec mirrors the concept independently:

```text
spec/resize/scalar/convolution.rs
spec/resize/scalar/bicubic.rs
spec/resize/scalar/lanczos.rs
```

The two separable implementations are separate code.

## Tiling model

Tiling partitions the output domain only.

It does not crop input images, resize cropped tiles, and paste them back together. That approach risks seams unless halos are handled perfectly.

Instead, each worker receives:

```text
full immutable input(s)
full output metadata
exclusive mutable access to one output row band/region
global coordinates for that output region
```

Sampling always uses global coordinates.

### Generic tiling types

```rust
pub struct ImageShape {
    pub width: u32,
    pub height: u32,
}

pub struct RowBand {
    pub y_start: u32,
    pub y_end: u32, // exclusive
}

pub struct RowBandPlan {
    pub output_shape: ImageShape,
    pub bands: Vec<RowBand>,
}
```

Top-level tiling depends only on dimensions/shape, total pixels, and cost hints. It does not know about RGBA, Oklab, palettes, resize kernels, or dither modes.

### Domain adapters

Domain adapters live under `prod/<domain>/tiling/` and answer:

```text
Given operation options + dimensions, what tiling plan should we use?
Given a row band, how do we process exactly those output rows?
```

Examples:

```text
prod/resize/tiling/bilinear.rs
prod/color/tiling/convert.rs
prod/dither/tiling/ordered.rs
```

### Source extents / halos

Some operations need to read outside the output band’s corresponding source rows. That is a source extent issue, not output tile stitching.

For resize:

```text
output rows 200..300
may read source rows 180..325
writes only output rows 200..300
```

The adapter computes any required source extent internally while still receiving the full source image.

### Tiling limitations

Some algorithms are not trivially independent by row band.

Examples:

- ordered dither: row-band safe
- color conversion: row-band safe
- resize: row-band safe if global coordinates are used
- error-diffusion dither: not naively row-band safe because rows depend on prior-row error
- palette histogram: requires per-band partials + merge

Adapters must encode these realities. Top-level tiling should not pretend every operation is embarrassingly parallel.

## Resize spec

Initial resize families:

```text
nearest     direct point sampler
area        exact coverage/integration resampler
bilinear    exact triangle-filter resize
bicubic     separable cubic preset
lanczos2    separable Lanczos radius=2 preset
lanczos3    separable Lanczos radius=3 preset
trilinear   policy/mipmap resampler
```

Open decision: whether `box` remains a public alias for exact `area`.

### Resize file roles

```text
spec/resize/scalar/nearest.rs       simplest direct loop
spec/resize/scalar/area.rs          simplest coverage integration
spec/resize/scalar/bilinear.rs      direct per-output coordinate/weight calculation
spec/resize/scalar/convolution.rs   readable finite-support convolution helper for bicubic/lanczos
spec/resize/scalar/bicubic.rs       bicubic kernel/preset semantics
spec/resize/scalar/lanczos.rs       lanczos radius/support semantics
spec/resize/scalar/trilinear.rs     mip policy semantics
```

```text
prod/resize/scalar/nearest.rs       optimized scalar nearest
prod/resize/scalar/area.rs          optimized scalar area
prod/resize/scalar/bilinear.rs      optimized scalar bilinear
prod/resize/scalar/convolution.rs   optimized scalar convolution engine
prod/resize/scalar/bicubic.rs       bicubic production preset/specialization
prod/resize/scalar/lanczos.rs       lanczos production preset/specialization
prod/resize/scalar/trilinear.rs     trilinear production policy
```

Scale-aware variants are support policies, not duplicated resize engines.

```rust
pub enum SupportPolicy {
    Fixed,
    ScaleAware,
}
```

## Palette creation direction

Palette creation should be its own domain because it feeds quantization and dithering.

Recommended default pipeline:

```text
input image
  -> color-space conversion
  -> weighted sample extraction
  -> perceptual histogram / candidates
  -> diversity-aware selection
  -> optional weighted refinement
  -> output palette
```

The public API should be goal-oriented:

```rust
pub struct PaletteOptions {
    pub color_count: usize,
    pub working_space: PaletteWorkingSpace,
    pub strategy: PaletteStrategy,
    pub seed: Option<u64>,
}

pub enum PaletteStrategy {
    Dominant,
    Balanced,
    HighContrast,
    AccentAware,
}
```

Spec implementation should define deterministic candidate generation and tie-breaking. Production can optimize histograms, sampling, and nearest-color lookup.

## Independent stages first, pipeline later

Independent APIs are v1.

Pipeline API is v2.

All v1 operations should have explicit option structs and deterministic outputs so they can later become memoizable pipeline stages.

Future pipeline direction:

```rust
Pipeline::new()
    .resize(...)
    .convert_color(...)
    .create_palette(...)
    .quantize(...)
    .dither(...)
    .run()
```

Memoization key shape:

```text
input fingerprint
+ operation options fingerprint
+ algorithm/version fingerprint
+ crate version
```

Do not build the full pipeline until independent stage contracts are stable.

## Spec-to-prod workflow

Every operation should move through the same lifecycle.

### 1. Write the spec implementation

- Implement the operation in `spec/<domain>/`.
- Prefer direct loops and obvious math.
- Document coordinate mapping, rounding, tie-breaking, and edge handling.
- Add edge-case tests against manually constructed inputs.

### 2. Add conformance tests

Tests compare future production outputs against spec.

```text
spec output == prod scalar output
spec output == prod tiled output
```

Use deterministic cases:

- 1x1
- 2x2
- non-square
- identity
- upsample
- downsample
- alpha edge cases
- boundary/clamping cases
- odd dimensions

### 3. Implement scalar production

- Add `prod/<domain>/scalar/<operation>.rs`.
- Start simple, then optimize.
- Keep output byte-for-byte exact against spec.
- Do not add tiling yet unless scalar semantics are stable.

### 4. Benchmark scalar production

- Add Criterion benchmarks for representative cases.
- Save explicit baselines before optimization loops.
- Only accept optimizations that pass conformance and improve the representative suite enough to justify complexity.

### 5. Add tiled production adapter

- Add `prod/<domain>/tiling/<operation>.rs`.
- Partition output only.
- Use full input and global coordinates.
- Compare tiled output to spec and scalar output.
- Tune row-band plan with tiling sweeps.

### 6. Expose public API

Only after spec + scalar + tests are stable:

- expose Rust public wrapper
- expose Wasm wrapper if needed
- add docs and examples

## Benchmark spec

Benchmarks are first-class design tools, but they must not define correctness. Correctness comes from `spec/`.

### Benchmark namespaces

Use distinct script namespaces:

```text
crit:*    Criterion benchmarks
bench:*   custom harnesses, shootouts, sweeps, quick compares
wasm:*    wasm-pack builds/tests
```

Optional later:

```text
brunch:*  Brunch microbenchmarks if they prove useful
```

### Benchmark directories

```text
benches/
  support/
    fixtures.rs
    cases.rs
    report.rs
    compare.rs

  crit_resize.rs
  crit_color.rs
  crit_palette.rs
  crit_quantize.rs
  crit_dither.rs

examples/
  resize_quick_bench.rs
  tiling_sweep.rs
  resize_shootout.rs
```

`benches/support` is benchmark-only infrastructure. It must not be imported by `src/`.

### Fixture policy

No hard-coded stale fixture paths.

Benchmark fixture discovery should:

1. read explicit CLI/env fixture selections
2. otherwise scan `benchmark-fixtures/`
3. support PNG and JPEG where decode is needed
4. print selected fixtures before running

Default fixture set should be small enough for routine runs. Full fixture sweeps require an explicit flag.

### Benchmark ID policy

Benchmark IDs should encode domain, operation, implementation, fixture, and case.

Example:

```text
resize/bilinear/prod-scalar/Celeste_box_art/0.5x
resize/bilinear/prod-tiling/Celeste_box_art/0.5x
resize/bilinear/spec/Celeste_box_art/0.5x
```

Script-level comparison names may use compact triples:

```text
resize:bilinear:prod-scalar
resize:bilinear:prod-tiling
resize:bilinear:spec
```

### Criterion policy

Criterion is for stable statistical comparisons.

Rules:

- use explicit baselines
- do not delete baselines to force comparisons
- save new baselines only after accepted changes
- keep default runs short enough for iteration
- provide full-suite flags for deeper validation

Recommended defaults:

```text
--warm-up-time 1
--measurement-time 5
--sample-size 50
```

### Quick compare policy

`bench:cmp:quick` is for fast local directional checks.

It should:

- run in-process Rust examples, not Wasm/CLI loops
- compare two implementation triples
- verify checksums or exact output equality when practical
- print ratio and percent delta
- be treated as directional, not final proof

Example:

```sh
pnpm bench:cmp:quick --compare resize:bilinear:prod-scalar --to resize:bilinear:prod-tiling
```

### Tiling sweep policy

Tiling sweeps are for deriving per-operation tiling plans.

They should:

- sweep row-band sizes / worker counts / minimum pixels per band
- report correctness first
- preserve selected logs/results
- compare scalar vs tiled for the same operation
- produce machine-readable JSON
- optionally plot results

Tiling plan decisions belong in the domain adapter, not the generic scheduler.

### Shootout policy

Shootouts compare Ditherette implementations to external libraries.

Rules:

- external libraries are allowed in benches/examples only
- external libraries must not become production correctness oracles
- report exactness/tolerance differences separately from timing
- do not compare a tolerance-based external implementation as if it were byte-exact

### Benchmark acceptance policy

An optimization is accepted only if:

1. conformance passes
2. benchmark is representative for the intended path
3. improvement justifies complexity
4. code remains maintainable

Rejected ideas may be recorded in production code with `REJECT(perf)` when useful. `REJECT(perf)` means closed unless material conditions change.

Do not put perf TODOs or REJECT notes in `spec/`.

## Testing policy

Use Rust's idiomatic split between inline unit tests and `tests/` integration tests.

### Inline unit tests

Inline `#[cfg(test)]` tests inside implementation files are for local behavior only:

```text
private helpers
small formula checks
local invariants that explain the file
```

They should not become the main operation behavior suite. If inline tests make the implementation harder to read, move them out.

Current example:

```text
src/spec/resize/common/alignment.rs  # axis mapping helper tests
```

### Integration tests

Integration tests live in `crates/ditherette-wasm/tests/` and exercise module/public behavior from the consumer side:

```text
tests/image_layout.rs
tests/spec_resize_nearest.rs
tests/spec_tiling_contract.rs
```

Use integration tests for:

```text
public/module behavior
cross-module behavior
full operation examples
future spec-vs-prod conformance
fixture/golden cases
Wasm boundary behavior
```

### Test layers

```text
unit tests       inline private/local semantic cases
integration      module/API behavior through public imports
conformance      future spec vs prod exactness
property-ish     generated deterministic shape/value cases
benchmark sanity selected benches verify exactness before timing
```

Conformance tests should be easy to add per operation and should exercise scalar and tiled implementations where available.

## Open decisions

- Final public resize filter set for v1.
- Whether `box` remains a public alias for `area`.
- Whether experimental alternatives like `bilinear_2` exist in the fresh crate or stay only in `ditherette-wasm-old`.
- Exact Wasm API shape: many named exports vs enum/config-driven entrypoint.
- Initial palette creation strategy and output format.
- How much of `prod -> spec` type reuse is acceptable without weakening oracle independence.
