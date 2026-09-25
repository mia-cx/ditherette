# #202 Add an ordered effect/plugin pipeline to the Rust/Wasm crate

## Summary

Callers pass an ordered array of effect instances. Wasm applies them in caller order on one continuous
f32 image, then either returns RGBA8 (`applyEffects`) or continues into the existing terminal
resize → dither/quantize stage (`process` with recipe version 2). Effects get a readable reference in
`spec/effects/`, a frozen extension record, and an independent copy in `prod/effects/` that is
optimized against benchmarks. The first built-in effect is `levels`; #106 and #201 add the rest on
top of this stack.

## Acceptance criteria

- [x] Public Rust/Wasm API accepts an ordered effect array; a noncommutative fixture verifies order; repeated instances keep independent arguments.
- [x] Standalone effects and pipeline composition agree; empty chain is identity; recipes replay deterministically.
- [x] Extension contract adds an effect without touching sequencing; plugin mechanism documented and exercised.
- [ ] (#201, stacked; deferred to the #201 PR) Recolouring composes with preceding adjustments and respects the enabled palette.
- [ ] Complete pipeline output uses only enabled palette colours, with and without dithering; no API shape allows colour effects after quantization; time/memory costs recorded.

## TODOs

- [x] Freeze policy: ordered reference extensions that add files (plus `spec/mod.rs` registration) without changing the v1 identity.
- [x] Spec: continuous effect image, `Effect` trait, ordered executor, recipe decoding/validation, `levels`, standalone `apply_effects`, `process` v2 composition, docs.
- [x] Spec tests: order, repetition, bypass, identity, custom-effect extension, levels vectors, validation paths, v2 composition equality.
- [x] Record the effects reference extension in the freeze checkpoint.
- [x] Prod: copy the reference into `prod/effects/`, prod-vs-spec conformance tests.
- [x] Prod: benchmark, then optimize (per-channel LUT folding) with evidence.
- [x] Wasm + package: `privateApplyEffects`, v2 `privateProcess` effects input, `applyEffects`, `RecipeV2`, validation, progress stage, docs, package tests.
- [ ] Final validation: cargo tests, freeze guard, wasm check, package tests.

## Notes

- Design: carrier is straight, unclipped, encoded-sRGB f32 RGB plus untouched alpha bytes. One RGBA8
  boundary at the end of the chain (clip, ×255, round half up). Effects run before resize.
- Plugin boundary: statically compiled built-ins decoded from JSON, plus the Rust `Effect` trait for
  crate consumers. No JS callbacks or runtime-loaded modules.
- Paths use `effects.<index>` to match the package's `palette.<index>` convention.
- Prod folds leading per-channel runs into byte tables: 24-37x faster than the reference (see `prod/effects/README.md`).
- process v2 applies effects inside the source snapshot (`EffectedInput`), so downstream caches stay warm across calls.
- The `Frozen reference` CI check fails on purpose: the checkpoint edit needs maintainer approval.
