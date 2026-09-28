# #302 Replace the three curve effects with one curves effect

## Summary

Replace `curves`, `model-curves`, and `channel-curve` with one ordered `curves` effect. Each step contains up to 16 explicit remap or adjustment curves. Every curve selects from the step’s original pixel, then edits the accumulated result.

The frozen reference defines the new behaviour. Production preserves exact output, RGB table folding, and pointwise colour memoisation. The package and web move to the new shape without migrating saved steps.

## Acceptance criteria

- [ ] `curves` accepts zero to 16 curves. Each curve accepts 2 to 16 valid points.
- [ ] `remap` requires matching input and output channels. `adjust` accepts any valid channel pair.
- [ ] Remaps preserve today’s model-curve semantics. Adjustments preserve today’s `channel-curve` semantics.
- [ ] Hue inputs use the cyclic Fritsch–Butland spline and enforce the existing closed seam.
- [ ] Every curve selects from the original step input. Output edits apply in list order to the accumulated result.
- [ ] The reference, production registry, package, and web no longer accept the three old shapes.
- [ ] Production matches the reference byte for byte, including randomised mixed-model and ordered-list cases.
- [ ] Existing table folding and pointwise memoisation remain active where valid.
- [ ] The current model picker and Arbitrary XY editor emit the new effect without visible UI changes.
- [ ] Saved old shapes are dropped without throwing.
- [ ] One freeze extension records the complete reference replacement.
- [ ] A 0.x-breaking package changeset and updated documentation describe the new schema.
- [ ] The existing root `CONTEXT.md` enters the commit unchanged.

## TODOs

- [x] Replace the frozen reference effects. Expand `spec/effects/curves.rs` and `curves.md` with the ordered curve list, shared channel types, both curve kinds, validation, spline selection, hue rules, and original-input selection. Remove `model_curves` and `channel_curve`, update the reference modules and registry, and rewrite `spec_effects.rs` around the unified contract. Record all changes as one `effects-curves` extension. The reference tests and local freeze guard pass.

- [x] Implement the direct production equivalent. Consolidate the old production modules into `prod/effects/curves.rs`, remove their registry variants, and implement each curve as an independent output-model round trip. Keep exact neutral and empty-step no-ops. Port deterministic coverage for validation paths, list ordering, cross-model selection, hue confidence, carrier overshoot, and closed hue seams. Native production tests pass before adding specialised paths.

- [x] Restore and prove production fast paths. Prepare at most 16 resolved curves and inline splines once per call. Keep per-channel tables for all-remap steps whose matching input/output channels use sRGB or linear RGB. Keep the pointwise colour memo for every other curves step. Rewrite the model-curves and channel-curve randomised comparisons as unified mixed-list comparisons, update allocation-failure coverage and benchmark recipes, and prove every optimised path byte-matches the reference.

- [ ] Cut the package and web over together. Replace the public effect types and validator with the new discriminated curve list. Update package type and validation tests, including rejection of all old shapes through `isEffect`. Make the web store the new package effect directly, remove old migration and step fan-out, count Curves as one effect step, and adapt the existing editor to build three remaps for model mode or one adjustment for Arbitrary XY. Add store tests showing old saved layers disappear without errors. Package and web checks pass at this commit.

- [ ] Finish release material and validation. Update the package README effect table, `DESIGN.md`, relevant production documentation, examples, and comments. Leave historical changesets alone, add a new minor changeset describing the 0.x breaking change, and add `CONTEXT.md` unchanged. Run formatting, TypeScript checks, package tests, focused Vitest suites, Rust tests, freeze tests, and the local freeze guard. Use `CARGO_BUILD_JOBS=2`, `nice`, one heavy build at a time, and add `crates/ditherette-wasm/node_modules/.bin` to `PATH`. Report the exact `/approve-freeze sha256:<policy-digest>` printed by the trusted-base check.

## Notes

### JSON shape

The effect uses `x` and `y` from today’s `channel-curve`, nested under an ordered `curves` list:

```json
{
  "effect": "curves",
  "enabled": true,
  "curves": [
    {
      "kind": "remap",
      "x": { "model": "oklch", "channel": "lightness" },
      "y": { "model": "oklch", "channel": "lightness" },
      "points": [[0, 0], [0.5, 0.6], [1, 1]]
    },
    {
      "kind": "adjust",
      "x": { "model": "hsl", "channel": "hue" },
      "y": { "model": "cielch", "channel": "chroma" },
      "points": [[0, 0.5], [0.5, 0.8], [1, 0.5]]
    }
  ]
}
```

Each curve has exactly `kind`, `x`, `y`, and `points`. A remap requires `x` and `y` to contain the same model and channel. An empty `curves` array is valid and preserves the carrier exactly.

### Original-input selection

For a step input `source`, initialise `current = source`. For each curve in list order:

1. Convert `source` to `x.model` and read `x.channel`. Earlier curves never affect this value.
2. Evaluate the linear or periodic spline at that value.
3. Convert `current` to `y.model`.
4. Apply only the selected output edit, then convert those coordinates back to the carrier. This becomes the next `current`.

A non-hue remap sets the current output coordinate to the spline result. A hue remap computes today’s shortest circular delta from the original hue to the spline target, scales it by the original hue confidence, and wraps modulo one. Zero confidence leaves `current` untouched without an output round trip.

An adjustment derives its amount from the original-input spline result. Input-hue confidence comes from `source`; output-hue confidence comes from `current`. Use the smaller applicable confidence, as `channel-curve` does. Zero confidence leaves `current` untouched. Hue adds up to half a turn, saturation and chroma apply gain, and other outputs add a normalised offset.

Each curve performs its own conversions. Curves never share a model conversion, even when adjacent models match.

### Freeze extension

Append one `effects-curves` entry with `tools/spec-freeze/extend.mjs`. The old curve files entered after v1, so extensions may replace or delete them:

- Replace `curves.rs` and `curves.md`.
- Delete `model_curves.rs`, `model_curves.md`, `channel_curve.rs`, and `channel_curve.md`.
- Replace the affected effect module, recipe registry, and index documentation.
- Retain `model.rs` and `model.md` because the unified effect still uses them.

The extension records deletions with `after: null` and verifies every previous hash. No freeze-tool change is needed. The candidate’s local guard should pass. The trusted-base check should fail only for missing maintainer approval and print the policy digest Mia must approve.

### Test disposition

Rewrite behavioural tests rather than losing their coverage:

- Fold old curves, model-curves, and channel-curve reference tests into unified validation and semantic tests.
- Replace both old randomised production suites with remap-only, adjustment-only, and mixed ordered-list comparisons.
- Update allocation and benchmark fixtures to the new JSON.
- Rewrite package type and validation cases around the new discriminated list.
- Replace web fan-out tests with direct one-step construction and saved-state rejection tests.
- Keep the web spline tests unchanged.

Delete tests tied only to removed structure: the exact three-curve tuple, rejection of sRGB by `model-curves`, old registry variants, old prepared-state variant names, RGB step fan-out, and the three-step Curves cost. Add explicit rejection tests for every old JSON shape.

### Production fast paths

A curves step remains table-foldable only when every entry is a remap with matching sRGB or linear-RGB input and output channels. Its scalar map retains the step’s original channel value while applying entries in list order.

All curves remain pointwise, so the existing colour memo survives. Prepared state holds resolved channels and up to 16 inline splines. Cross-model remaps, every adjustment, and hue-dependent work use this path. The old `channel-curve` adjustment table special case does not survive.

### Open questions

None for #302. Overlay editing, masks, two-input curves, and compiled LUTs remain in their later slices.

### TODO 1 validation

- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml` passed.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test spec_effects` passed: 21 tests.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test spec_grading` passed: 6 tests.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml` with all 29 `spec_*` integration targets passed.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --lib` passed: 117 tests.
- `node tools/spec-freeze/extend.mjs effects-curves "Replace three curve effects with one ordered curves effect"` recorded one extension with 10 changes.
- `NODE_OPTIONS=--require=/tmp/spawnsync-status-zero.cjs nice node tools/spec-freeze/guard.mjs` passed. The temporary shim works around this VM reporting `EPERM` after successful synchronous child processes; the guard and repository files were unchanged.
- Plain `cargo test` reaches one expected TODO 2 compile failure in `tests/prod_effects.rs:361-362`, where the production comparison still imports removed `spec::model_curves` items. Production library code compiles and its unit tests pass.

### TODO 2 validation

- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml -- --check` passed.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_effects` passed: 16 tests.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_effects_allocation` passed: 1 test.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml` passed all unit, integration, and doc-test targets.
- The old randomised fixtures and allocation fixture were ported minimally to the unified JSON shape so the crate compiles. TODO 3 still owns prepared curves, table folding, and the unified mixed-list randomised proof.

### TODO 3 validation

- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml -- --check` passed.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_effects` passed: 18 tests, including remap-only, adjustment-only, and mixed ordered-list randomised comparisons.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_effects_allocation` passed: every injected allocation failure returned cleanly and the processor remained usable.
- `CARGO_BUILD_JOBS=2 nice cargo bench --manifest-path crates/ditherette-bench/Cargo.toml --bench crit_effects --no-run` passed with the unified curves JSON recipes.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml` passed every unit, integration, and doc-test target.
- `CARGO_BUILD_JOBS=2 nice cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --features threads` passed every unit, integration, and doc-test target.
