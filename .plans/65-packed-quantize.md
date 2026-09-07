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

Public Processor, Wasm bindings, package methods, and benchmark registration belong to the coordinator and later joins.
No benchmark, optimization experiment, PR, or rollout runs in this subtask.

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

Native prerequisites are ready. Public quantize registration, durable JS indexed-result construction, browser conformance, benchmark subjects/measurements, and the S24 PR remain outstanding.
No new optimization was attempted. Existing landed color math and lookup tables are reused unchanged.
