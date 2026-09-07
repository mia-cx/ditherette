# S24 native packed-color quantization

Base: `467542f49ce3f600e5b03aeef574a97be554ae15`. Issue #65.

## Existing implementation

`prod/color.rs` already owns the five ordinary forward formulas, byte lookup tables, and legacy f32x4 writers.
Keep those APIs, formulas, tables, and callers intact. The packed writer reuses their conversion calls.
Palette preparation, indexed alpha handling, and direct matching are missing production modules.
Their frozen counterparts provide the baseline. No production algorithm or shared helper is replaced.

## TODOs

- [x] Add the missing baseline with copied palette/alpha/direct-scan behavior and packed color wiring; verify exact outputs.
- [ ] Add fallible preparation, capacity accounting, and allocation-free execution for later public integration.
- [ ] Validate native/Wasm independence and hand off the committed native interface.

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
