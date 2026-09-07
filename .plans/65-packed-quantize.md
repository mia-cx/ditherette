# S24 native packed-color quantization

Base: `467542f49ce3f600e5b03aeef574a97be554ae15`. Issue #65.

## Existing implementation

`prod/color.rs` already owns the five ordinary forward formulas, byte lookup tables, and legacy f32x4 writers.
Keep those APIs, formulas, tables, and callers intact. The packed writer reuses their conversion calls.
Palette preparation, indexed alpha handling, and direct matching are missing production modules.
Their frozen counterparts provide the baseline. No production algorithm or shared helper is replaced.

## TODOs

- [x] Add the missing baseline with copied palette/alpha/direct-scan behavior and packed color wiring; verify exact outputs.
- [x] Add fallible preparation, capacity accounting, and allocation-free execution for later public integration.
- [x] Validate native/Wasm independence and hand off the committed native interface.

Public integration resumes after joining `ebbc1c5a32a77f68c2e59595e84a7b227c25c7ee`.
This worktree owns quantize-specific Processor, private Wasm, and package methods. S23 owns separate trilinear method sections.
The coordinator owns benchmark registration and measurements. No benchmark, optimization experiment, PR, or rollout runs here.

## Public integration TODOs

- [x] Add native Processor quantize ownership with complete allocation preflight and focused failure fixtures.
- [x] Add borrowed private quantize ABI and caught complete indexed-result construction; verify handle cleanup.
- [x] Expose strict five-space quantize requests and durable indexed results in the package.
- [x] Validate both Wasm builds, interface/private fixtures, and installed tarball in three engines; push evidence.

## Native interface

Prepared palette and Euclidean matcher own only retained palette metadata and working-coordinate triplets.
The packed color writer borrows RGBA8 alpha unchanged; it never writes a fourth float channel.
An allocation-free indexed writer consumes validated inputs and caller-owned output storage.
Ordinary spaces are sRGB, linear RGB, Oklab, CIELAB, and YCbCr. Other matching recipes remain unavailable here.

## Baseline validation

`cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_quantize` passes four tests.
They compare packed float bits, preserve the legacy four-channel API, and compare complete indexed results for all five spaces.
Fixtures include duplicates, ties, transparent-only/fallback/truncation warnings, matte/premultiplied rounding, and the f64 threshold witness.
`prod/palette/mod.rs` is a literal frozen copy. Matcher arithmetic and scan order are copied unchanged, with ordinary-space registration and existing converter wiring.
The old color module changes only by adding its packed child module declaration.

Baseline commit: `a23260edec0452fd17c13073636f548b07804230`.

## Integration handoff

Use `PreparedQuantizer::required_capacity_bytes(entries, alpha, matching)` before reserving call-owned buffers.
`try_new(entries, alpha, matching, memory_limit)` checks settings and fallibly reserves every prepared allocation.
`capacity_bytes()` includes the owned record, existing conversion tables, Vec capacities, and warning-string capacities.
`quantize_into(ImageView<Rgba8>, &mut [u8])` allocates nothing and retains no source buffer.
`into_indexed(ImageBuf<PaletteIndex8>)` moves metadata into the complete native result without allocation.

The native convenience `quantize(request, memory_limit)` also owns/reserves output indices and returns `IndexedImage`.
It uses existing typed request validation. Its `QuantizeError::Request` diagnostics contain strings.
The public adapter should perform its allocation-free boundary validation, then use `PreparedQuantizer` directly.
`PreparationError` contains only a stable `ErrorCode` and static path, including allocation failures.
The adapter must count its owned source copy and control separately; native source views are borrowed.
Palette/matcher state can be reused privately. This subtask does not implement cache identity, eviction, or publication.

## Final native evidence

- Eight focused tests pass. Seven injected allocation positions cover palette bytes, visible entries, warnings, both warning strings, matcher coordinates, and output indices.
- Exact budget succeeds; one byte less rejects before any allocation. Prepared execution performs zero allocations.
- `cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --features bench-subjects --quiet` passes 288 tests, summed from actual result groups.
- `cargo fmt --check` and `cargo check --locked --target wasm32-unknown-unknown` pass for the core manifest.
- The separately trusted freeze guard passes against the S20 coordinator checkout. Spec, image, and policy bytes remain unchanged.

At this native checkpoint, public integration and browser conformance were still outstanding. The checkpoints below complete that work.
No new optimization was attempted. Existing landed color math and lookup tables are reused unchanged.

## Public native checkpoint

`Processor::quantize` reuses `PreparedQuantizer` and preflights prepared ownership plus source and index capacities before importing source bytes.
It follows the shared Ready/Running/Disposed lifecycle and restores Ready after ordinary failures.
The boundary constructs a complete durable result while Rust retains every temporary; no result is published before completion.
Three new Processor tests and four existing direct-quantize tests pass. They cover all five spaces/three alpha policies,
257-entry metadata, the f64 threshold witness, exact/one-under budgets, both image reservation failures, caught copy/completion failures, recovery, and disposal.
An initial fixture used the wrong owned-image constructor name; it now uses existing `ImageBuf::from_vec_packed`.

## Private Wasm checkpoint

`privateQuantize(input, width, height, paletteCodes, matching, alphaMode, threshold, matteRgb, resultSink)` returns a numeric status.
Matching codes 0..4 select sRGB, linear RGB, Oklab, CIELAB, and YCbCr Euclidean. Alpha codes 0..2 select preserve, premultiplied, and matte.
Palette codes are RGB integers or 16777216 for transparent. At most 257 normalized entries preserve truncation semantics.
The fixed Rust palette temporary is counted in quantize-specific boundary capacity; no original palette/source buffer is retained.
Caught scalar palette reads and borrowed input/result slices avoid generated owned input allocations and caught owned-return handles.
The caught void completion helper constructs durable indices, palette, warnings, and the complete sink from authoritative Rust metadata.

Scalar build and two release-Wasm private fixtures pass. Generated privateQuantize contains no malloc/passArray/slice/owned-handle insertion.
512 cycles each exercise success and input/index/palette copy failures, frozen-sink failure, and throwing palette getters.
Externref table capacity, live handles, and Wasm pages remain unchanged after warmup; later calls recover.

## Public package checkpoint

Implementation commit: `2e1b1d8f324828ea5b99adab9de9c063b9cb5993`.

`quantize({ version: 1, source, palette, alpha, matching })` returns the direct indexed result.
Its fields are `width`, `height`, `indices`, `palette: { rgba, transparentIndex }`, and `warnings: { code, message }[]`.
Both byte arrays are durable JS-owned copies. No prepared palette or source survives the call.
The active-call guard precedes request property reads. All original palette entries are validated, including the discarded tail.
Normalization retains at most 257 codes; the last code signals truncation without retaining the original palette.
Preserve thresholds remain f64. Future matching tags fail explicitly as unsupported; malformed tags fail validation.
Progress remains explicitly unsupported until S33. Resize requests and landed color formulas remain unchanged.

The indexed completion helper uses captured intrinsic lengths through the existing input-length helper.
This prevents a replaced typed-array length getter from padding output copies.
All imported copies and complete-result publication remain caught void calls.

## Final public validation

- `pnpm package:build` passes scalar and threaded builds, private factory staging, and package TypeScript compilation.
- `cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --features bench-subjects --quiet` passes 307 tests, summed from actual result groups.
- `node --test crates/ditherette-wasm/tests/private_processor.mjs crates/ditherette-wasm/tests/private_quantize.mjs` passes nine tests.
- `pnpm --filter ditherette test:interface` passes 20 tests and the public type fixture.
- `DITHERETTE_TEST_WEBKIT_EXECUTABLE=/tmp/ditherette-webkit-libs.2dS6Yu/webkit pnpm --filter ditherette test:browser` passes all three engines and its parent test.

Installed-tarball engines are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
The package fixture installs offline into a temporary consumer and imports the packed artifact.
WebKit uses the unchanged private launcher and extracted libraries documented in `.plans/60-browser-runtime.md`.
No host package installation or system changes were needed.

Fixtures cover all five spaces, exact indices/palette/warnings, offset and detached inputs, alpha rounding, and first-index ties.
They also cover 257-entry truncation, invalid discarded entries, exact/one-under memory budgets, and getter reentry.
Input, index-copy, palette-copy, and result-publication failures recover without exposing partial output.
Earlier indexed results remain unchanged after later calls, Wasm memory growth, and disposal.

`cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` and `git diff --check` pass.
The separately trusted freeze guard passes against `.worktrees/v1-s18-freeze`.
It reports frozen revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and artifact digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

No benchmarks or new optimization ran. The coordinator owns benchmark integration, measurements, and the eventual S24 PR.
S23 trilinear remains a separate join; its package edits concern resize sections, not the new quantize method.
