# Rust/Wasm port design notes

## Code rules

- Keep every Rust source file single-concern. A file/module owns one piece of work: one data model, one transform, one adapter, one algorithm, or one test fixture family.
- Componentize shared logic before duplicating it. Shared code must describe a real concept, not just hide two lines of code.
- Use clear, domain-specific names. Prefer names that say what data represents in the image pipeline: `source_rgba`, `palette_rgba`, `output_indices`, `row_stride`, `transparent_index`.
- Document exported types and functions with docstrings. Use code comments for concepts, invariants, and non-obvious tradeoffs; avoid comments that restate syntax.
- Optimize for readable, maintainable code first. Hot loops can be specialized later after benchmarks prove the target.
- Add `TODO(perf):` comments on functions or code blocks that could become faster through SIMD, lookup tables, memory layout changes, loop unrolling, tiling, or other readability-costing optimizations.
- Keep trust-boundary validation at Wasm entry points. Internal functions can assume typed, already-validated inputs.
- Keep deterministic behavior testable in Rust before wiring algorithms into Svelte or workers.

## Boundary decisions

### Image decoding

Use the browser for source image decoding.

Browser-owned decode keeps format support, color/profile handling, EXIF orientation behavior, security hardening, and streaming/file API integration in the platform instead of making the Rust/Wasm core responsible for image codecs. The Rust/Wasm core should receive already-decoded, normalized pixel buffers from JavaScript.

Initial boundary:

1. Browser accepts `File`/`Blob` input.
2. Browser decodes via `createImageBitmap`, `HTMLImageElement`, or Canvas fallback.
3. Browser normalizes every decode path into one canonical source image shape before Wasm.
4. Browser applies crop/resize if the active frontend pipeline still owns those stages.
5. JavaScript transfers a contiguous RGBA buffer plus dimensions into Wasm.
6. Wasm returns palette indices, optional preview RGBA, and metadata needed for export.

Canonical Rust input contract:

- `source_rgba: &[u8]` — tightly packed RGBA bytes in row-major order.
- `width: u32` — source/output width for this processing call.
- `height: u32` — source/output height for this processing call.
- Invariant: `source_rgba.len() == width * height * 4`.
- Invariant: channels are 8-bit RGBA in browser `ImageData` channel order.
- Invariant: rows are tightly packed; no stride/padding parameter at the Rust boundary until a measured use case needs it.

JavaScript may represent the same bytes as `ImageData`, `Uint8ClampedArray`, or `Uint8Array`, but the Wasm adapter owns that normalization. Rust should not expose overloads for browser-specific source types.

Open question: whether resize eventually moves into Rust/Wasm. Keep decode out either way; resize is a numeric transform and is a plausible Wasm stage, while decode is an external format/platform concern.

## First implementation slice: nearest-neighbor resize

### Goal

Implement the smallest useful Rust/Wasm image transform: resize a canonical RGBA image buffer with nearest-neighbor sampling.

This slice proves the Rust boundary, module layout, validation style, test style, and JS/Wasm package shape before porting color-space or dithering work.

### Non-goals

- No browser decode work in Rust.
- No crop/fit policy in Rust; caller passes the already-selected source buffer and requested output dimensions.
- No aspect-ratio decisions in Rust; UI/domain code owns dimension selection.
- No color-space conversion, palette matching, dithering, alpha policy, PNG export, or preview rendering.
- No SIMD, tiling, threading, or lookup-table optimization in the first pass.

### Public Wasm contract

Expose one Wasm adapter function for this slice:

```rust
resize_rgba_nearest(
    source_rgba: &[u8],
    source_width: u32,
    source_height: u32,
    output_width: u32,
    output_height: u32,
) -> Result<Vec<u8>, JsValue>
```

Input invariants validated at the Wasm boundary:

- `source_rgba.len() == source_width * source_height * 4`.
- `source_width`, `source_height`, `output_width`, and `output_height` are greater than zero.
- Size arithmetic cannot overflow `usize`.
- Output allocation size is exactly `output_width * output_height * 4` bytes.

Rust-internal resize code should not know about `JsValue`; it returns a Rust-native result/error and is called by the Wasm adapter.

### Sampling rule

Use integer proportional mapping for deterministic, readable nearest-neighbor behavior:

```rust
source_x = output_x * source_width / output_width
source_y = output_y * source_height / output_height
```

Then copy the four RGBA bytes from the selected source pixel to the output pixel.

This rule intentionally avoids floating-point rounding and center-offset ambiguity for the first implementation. It preserves identity resizes exactly, maps 2× upscales as repeated source pixels, and makes tests easy to reason about.

Add a `TODO(perf):` comment near the row loop noting future optimization options: precomputed source-x lookup table, row-copy fast paths for same-width rows, `u32` word-copy pixel paths, SIMD/vectorized copies, and tiled/chunked processing.

Keep these performance ideas out of the first slice unless benchmarks force them:

- Precompute `source_x` byte offsets once per output width instead of recomputing proportional x mapping for every row.
- Precompute `source_y` row offsets once per output height.
- Add an identity fast path that returns or clones the input when dimensions match.
- Add same-width row-copy fast paths for vertical-only resizes.
- Add exact integer-scale fast paths, e.g. 2×/3× upscales or clean divisors for downscales.
- Copy each RGBA pixel as a `u32` word when alignment and Wasm memory assumptions are proven safe.
- Use unsafe unchecked indexing in the hot loop after validation proves bounds.
- Process rows in tiles/chunks to improve cache locality and future worker progress reporting.
- Add SIMD/vectorized copies where the algorithm shape and browser support make it worthwhile.
- Reuse caller-provided output buffers or Wasm-owned scratch buffers to reduce repeated allocations.

### Directory and file structure

Keep the Rust crate organized by pipeline stage and shared primitives:

```txt
crates/ditherette-wasm/
  Cargo.toml
  README.md
  src/
    lib.rs                  # Crate exports and module declarations only.
    wasm.rs                 # wasm-bindgen boundary functions and JsValue error mapping.
    error.rs                # Shared Rust error type for validation/processing failures.
    image/
      mod.rs                # Image module declarations/re-exports.
      rgba.rs               # Canonical RGBA buffer constants/helpers: channel count, byte length math.
      dimensions.rs         # Width/height value helpers and checked pixel/byte counts.
    resize/
      mod.rs                # Resize module declarations/re-exports.
      nearest.rs            # Nearest-neighbor RGBA resize implementation only.
    pipeline/
      mod.rs                # Reserved stage organization; no broad orchestration yet.
      stages.rs             # Stage naming/types only when needed by integration.
    palette/
      mod.rs                # Reserved for palette data/compilation later.
    color/
      mod.rs                # Reserved for color spaces/conversions later.
    dither/
      mod.rs                # Reserved for dithering algorithms later.
    quantize/
      mod.rs                # Reserved for palette matching/quantization later.
  tests/
    resize_nearest.rs       # Rust integration tests for resize behavior.
    wasm_resize.rs          # wasm-bindgen tests for the exported boundary.
```

Only create files when the slice needs them. For the nearest-neighbor slice, create `wasm.rs`, `error.rs`, `image/mod.rs`, `image/rgba.rs`, `image/dimensions.rs`, `resize/mod.rs`, and `resize/nearest.rs`. Leave future stage directories out until their first real code lands, unless an empty `mod.rs` directly improves navigation.

### Planned stage boundaries, not yet specified

The long-term shape should keep each stage isolated without committing to detailed APIs yet:

1. Decode normalization in TypeScript/browser code.
2. Resize/resample in Rust/Wasm once this slice lands.
3. Color preparation/conversion in Rust/Wasm, potentially with non-RGBA internal buffers such as Oklab.
4. Palette compilation and search structures in Rust/Wasm.
5. Quantization and optional dithering in Rust/Wasm.
6. Indexed output assembly in Rust/Wasm or TypeScript, to be decided when export work starts.
7. Browser display/download orchestration in TypeScript.

Each stage should receive one canonical input shape for that stage. RGBA is the first boundary shape, not a permanent requirement for later color-space-specific stages.

### Browser smoke benchmark plan

Yes: each Rust/Wasm processing stage should get a small browser smoke benchmark alongside correctness tests. Playwright is the right harness because it measures the generated Wasm package in a real browser instead of only testing native Rust or Node Wasm behavior.

Keep smoke benches separate from correctness tests:

- Correctness tests are deterministic and fail on any mismatch.
- Smoke benchmarks measure representative runtime and should fail only on broken loading, broken output shape, or extreme regressions.
- Detailed `TODO(perf)` studies should reuse the same harness but compare explicit variants and write JSON artifacts instead of relying on one noisy threshold.

Initial resize benchmark shape:

1. Build `crates/ditherette-wasm/pkg` before the Playwright bench.
2. Load a minimal benchmark page/module that imports the generated Wasm package.
3. Decode `benchmark-fixtures/Celeste_box_art_full.png` in the browser, normalize it to canonical RGBA bytes, and benchmark these scales: 0.95x, 0.75x, 0.5x, 0.25x, and 0.125x.
4. Run two benchmark lanes for each scale:
   - **Browser decode lane:** reports browser decode + RGBA normalization costs alongside Wasm resize timing. This measures the actual product boundary and catches browser decode/normalization integration failures.
   - **Decoded RGBA lane:** reuses the already-decoded RGBA bytes, then calls Wasm directly. This isolates Rust/Wasm stage performance from decode noise and is the right lane for `TODO(perf)` studies.
5. Warm up each case before measuring.
6. Run several iterations and report median, min, max, decode time, normalization time, Wasm time, and output byte length.
7. Assert only invariants in CI: Wasm loads, output length is correct, checksum is stable, runtime is finite and below a generous smoke ceiling.
8. Save machine-readable results under `benchmark-results/wasm-resize-nearest.json` for perf passes.

Implemented harness file:

```txt
scripts/
  benchmark-wasm-resize.mjs     # Builds on Playwright + a tiny static server for real-browser Wasm resize timing.
```

Future app-integration files can still be added when the Svelte UI imports Wasm directly:

```txt
src/lib/wasm/
  load-ditherette-wasm.ts       # Browser-side Wasm package loader adapter.

e2e/
  wasm-resize-smoke.e2e.ts      # Product-level smoke test once Wasm is wired into the app.
```

Do not add fine-grained optimization variants to production code just for the smoke bench. For `TODO(perf)` studies, put experimental variants behind separate benchmark-only branches or clearly named internal functions until the benchmark proves they should replace the readable implementation.

### Test plan

- Unit-test dimension byte-count validation, including zero dimensions and overflow-shaped inputs.
- Unit-test identity resize returns the same bytes.
- Unit-test 1×1 → N×M fills all output pixels with the source pixel.
- Unit-test 2×2 → 4×4 repeats each source pixel into a 2×2 block.
- Unit-test 4×4 → 2×2 samples the expected top-left proportional pixels.
- Unit-test non-square resize, e.g. 3×2 → 2×3, to catch x/y stride mistakes.
- Wasm-test that invalid input length returns an error instead of panicking.
- Wasm-test that valid input returns a `Vec<u8>` with expected length and bytes.

### Acceptance criteria

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml` passes.
- `cargo clippy --manifest-path crates/ditherette-wasm/Cargo.toml -- -D warnings` passes.
- `pnpm wasm:build` produces `crates/ditherette-wasm/pkg`.
- `pnpm wasm:test` passes boundary tests under Node; browser-specific Wasm tests can live under `pnpm wasm:test:browser` when a slice uses browser APIs.
- Public exported Rust/Wasm docs explain the RGBA boundary and nearest-neighbor sampling rule.
- Every created Rust file has one clear responsibility.
