# S15 full-image diffusion reference

Issue [56](https://github.com/mia-cx/ditherette/issues/56).
Branch `impl/v1-s15-diffusion`, PR base `impl/v1-s15-base`.
Review [PR 101](https://github.com/mia-cx/ditherette/pull/101), open and non-draft with auto-merge disabled.
Join `578d677822d5daa8d4b63e7f2cb709c12fd608d0` contains S10 `47712a500c4079293a06fae4a07ae105a643af8f`
and S12 `01df66826e532d8fb3b522a1564f1121c96f4d1f`.

## TODOs

- [x] Audit all four tap sets and define explicit byte/working-coordinate feedback behavior.
- [x] Compose validated diffusion requests with alpha, placement, ordered matching, and finite-arithmetic handling.
- [x] Verify tiny independent images, full native tests, Wasm compilation, and formatting.
- [x] Rebase onto the latest join and file the unmerged stacked PR.

## Prerequisite checks

Both exact dependency SHAs pass ancestry checks.
`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_quantize_request --test spec_quantize_dispatch --test spec_dither_placement`
passes 23 tests. Wasm compilation with the locked manifest also passes.

No production, ring-buffer optimization, benchmark code, aggregate ledger, or root user-file changes belong to this slice.

## Approved findings

The website uses rounded/clipped byte RGB feedback when `useColorSpace` is false.
Its matching-space path leaves working coordinates unrounded before matching and error emission.
For sRGB matching, the existing `Diffusion.space` field cannot distinguish those behaviors.
With palette red bytes 0 and 2, source red bytes 1 and 1, and Floyd strength 1,
byte feedback emits `[0,0]`; matching-coordinate feedback emits `[0,1]`.
The coordinator approved `DiffusionFeedback::{SrgbBytes, Matching}` in place of the ambiguous `space` field.
Serialized diffusion recipes now require `feedback: "srgb-bytes" | "matching"` and reject the old field.
The inherited generic coordinate kernels keep their unrounded behavior.

Finite strengths near f32 maximum can overflow distance scores before they overflow work storage.
The reference must report that arithmetic failure rather than silently treating infinite/NaN scores as first-entry ties.
The coordinator approved `Runtime` at `dither.arithmetic`, without a strength ceiling or coordinate saturation.

The tap audit verifies every offset and fractional weight. Floyd, Sierra, and Sierra Lite sum to 1.
Atkinson deliberately distributes 6/8; missing edge taps are discarded rather than renormalized.
Focused tap and contract checks pass ten tests.

## Complete composition

`error_diffusion::diffuse(DitherQuantizeRequest)` returns owned indexed output or a structured error.
It uses full-image f32 work, unmodified-source placement, and per-scalar f64 scatter arithmetic rounded back to f32.
Matching retains all 15 f32 metric recipes. Fixed-index alpha pixels discard error before it can enter useful work.
Existing low-level coordinate adapters keep their signatures and unrounded behavior.

Six diffusion tests pass, covering tap definitions, the two-pixel feedback distinction, transparent sinks,
separate work/score overflow errors, legal maximum-strength success, and zero-strength composition across all kernels and metrics.
The normative recipe and TypeScript source pointers live in `crates/ditherette-wasm/src/spec/dither/error_diffusion.md`.

## Final correctness evidence

The completed diffusion suite has 13 tests. Hand-calculated 2x2 outputs distinguish raster and serpentine for every kernel.
A one-column `[64,0,120]` fixture distinguishes two-row taps and verifies discarded edge weights.
Positive-strength neutral fixtures cover every metric and both feedbacks.
Adaptive fixtures distinguish unchanged source from error-modified work, and matching-space contrast from byte-feedback coordinates.
They also verify hidden RGB influences placement while transparent pixels emit no diffusion error.

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --quiet`: passes every test group, zero failures.
- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked -- --list | rg -c ': test$'`: **181 actual native tests**.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown`: passes.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --all -- --check`: passes.
- `git diff --check`: passes.

The count comes from the executable test listing, not inherited slice prose.
No benchmark process or browser timing run was started.

The final fetch/rebase found the join unchanged at `578d677822d5daa8d4b63e7f2cb709c12fd608d0`.
All 31 focused diffusion, contract, and placement tests pass again after rebase.
Validated code and evidence head was `ee963fb48ef945ee9fad3affe4d8a5771768f1ef`.
The final bookkeeping commit changes only this PR record; the coordinator records its exact SHA in the stack ledger.
