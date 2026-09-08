# S13 palette-free Bayer and random perturbation

Implements [issue 54](https://github.com/mia-cx/ditherette/issues/54).
Branch `impl/v1-s13-perturb` uses PR base `impl/v1-s13-base`.
Validated join `bfa3d42b79dcc51db9a1f99da3be2c02653b3009` contains both prerequisite commits:

- S12 placement `01df66826e532d8fb3b522a1564f1121c96f4d1f`.
- S09 palette `d8bcdcdbe8f874eaee65447a640b97483c9cb775`.

Both ancestry checks pass. All 19 prerequisite placement/palette tests compile and pass.

## Work

- [x] Define Bayer thresholds and global-index random draws with exact fixtures.
- [x] Reconstruct field-generated f64 coordinates without overflowing valid strengths, with seven-space fixtures.
- [x] Compose palette-free RGBA8 perturbation and quantize-after-perturb through the byte boundary, with focused tests.

## Ownership

This slice owns ordered/random fields, shared perturb composition, the dither barrel, and focused tests/docs.
S14 owns blue-noise generation, asset, and its module. S10 owns the shared color dispatcher on a separate branch.
Local color dispatch keeps this slice buildable from its declared prerequisites.
The coordinator also assigns the new `spec/color/reconstruct.rs` and its single barrel declaration to this slice.
This per-pixel f64 arithmetic retains the finite nonnegative strength contract without introducing f64 image planes.
No production or benchmark changes belong here.

## Evidence

Three field fixtures and all 12 inherited dither tests pass.
Independent JavaScript `Math.imul` fixtures cover three seeds, row-adjacent indices, and the 32-bit sequence boundary.
The one-draw-per-global-pixel assignment is independent of alpha, strength, placement, and traversal order.
Four reconstruction fixtures pass, covering known vectors, sampled seven-space byte round trips, hue/neutral conventions, and maximum legal strengths.
The numeric proof bounds valid inverse intermediates below `1e125`; no new public strength ceiling or coordinate clipping is needed.

Final validation:

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked` passes all 168 native tests.
- Focused `spec_dither_perturb` passes 11 tests and `spec_color_reconstruct` passes four.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown` passes.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml -- --check` and `git diff --check` pass.

No benchmark measurement ran. Inherited native benchmark-export smoke tests provide no performance evidence.

## Handoff

Read [perturb.md](../../../crates/ditherette-wasm/src/spec/dither/perturb.md) before integrating S14 or the S17 pipeline.
It defines the shared callback signature, global draw assignment, reconstruction boundary, and temporary blue-noise registration.
The current `BlueNoise` branch is explicitly uncertified inherited data; S14 replaces its lookup at the validated join.
S17 supplies the completed S10 quantizer to `quantize_after_perturb` and reconciles local forward dispatch.
S38 uses the current shared TS RGB strength constant, 96, divided by 63.75; normalized color-space strength needs no correction.
The historical 64 value in frozen reference notes is not the current website scale.
