# S16 adaptive Yliluoma mixing

Implements [issue 57](https://github.com/mia-cx/ditherette/issues/57).
Branch `impl/v1-s16-yliluoma` uses PR base `impl/v1-s15-base`.
Validated join `578d677822d5daa8d4b63e7f2cb709c12fd608d0` contains S10 `47712a500c4079293a06fae4a07ae105a643af8f` and S12 `01df66826e532d8fb3b522a1564f1121c96f4d1f`.
Both ancestry checks and all 23 prerequisite matching/request/placement tests pass.

## Work

- [x] Preserve exhaustive pair/ratio search while adding typed matching and tie/hue fixtures.
- [x] Compose validated adaptive Yliluoma requests with alpha, metadata, Bayer recipes, and all matching policies.

## Ownership

Only the Yliluoma reference module, its documentation, focused tests, and this evidence file belong to S16.
S15 owns diffusion and its request correction. S13 reconstruction is outside this base and unnecessary for palette mixing.
No production changes, new public mixing modes, or benchmark measurements belong here.

## Recipe witness

Zero placement changes the target to the nearest palette coordinate, not the pair-search tie order.
For sRGB palette `[black,gray128,gray64]` and source gray64, pair `(0,1)` at ratio 1/2 exactly represents the target.
It wins before pure entry 2. A zero mask can therefore still emit the earlier pair's Bayer pattern.
Preserving the approved target formula and inherited enumeration requires retaining this result, not a nearest-only shortcut.

## Evidence

Four focused pair/ratio/hue fixtures and all 12 inherited dither tests pass after sharing the search.
The coordinator confirmed the zero-placement witness retains the inherited search result.

Final validation:

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --quiet` passes all 179 native tests.
- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked -- --list` lists 179 test cases; this is a counted total.
- `spec_dither_yiluoma` passes 12 focused fixtures covering every matching policy and Bayer width.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown` passes.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml -- --check` and `git diff --check` pass.

No benchmark measurement ran. Inherited benchmark-export smoke tests do not provide performance evidence.

## Handoff

S17 can dispatch Yliluoma requests to `spec::dither::yiluoma::dither_yiluoma`.
It returns `Result<IndexedImage,DitheretteError>` after complete request validation, alpha preparation, target adaptation, and pair selection.
Read [yiluoma.md](../../../crates/ditherette-wasm/src/spec/dither/yiluoma.md) before changing target or mixture composition.
The documented zero-mask witness and componentwise hue interpolation are intentional recipe behavior.
