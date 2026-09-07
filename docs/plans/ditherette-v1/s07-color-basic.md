# S07 ordinary color references

Issue [#48](https://github.com/mia-cx/ditherette/issues/48). Base and dependency: S03 at `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`, branch `impl/v1-s03-contracts`, PR #89.

Branch: `impl/v1-s07-color-basic`. Ownership is limited to the sRGB, linear RGB, and YCbCr reference modules/docs and their tests. Production code, common helpers, and shared module exports remain unchanged.

1. [x] Define and test per-pixel forward/inverse equations and RGBA8 reconstruction with unchanged alpha.
2. [ ] Validate inherited color tests, full native tests, Wasm-target compilation, and formatting; record exact evidence.
3. [ ] File an unmerged stacked PR against S03.

The per-space pixel helpers accept RGB byte triplets or f32 working triplets. Image inverse adapters accept the working image, corresponding RGBA8 alpha source, and mutable RGBA8 output. Final encoded RGB clips to `[0,1]`; byte ties round upward. Canonical coordinate domains remain unchanged. S08 uses the same reconstruction convention for perceptual spaces; S10 can join dispatch after both dependencies are available.

The [space references](../../../crates/ditherette-wasm/src/spec/color/) record primary formula sources and numerical conventions. This slice runs correctness checks only; no benchmarks or optimization candidates.

The ten new ordinary-color tests and six inherited color tests pass. Full validation and PR evidence follow.
