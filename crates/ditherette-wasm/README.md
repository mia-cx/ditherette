# ditherette-wasm

Fresh Rust/Wasm image-processing core for Ditherette.

The previous prototype has been preserved as `crates/ditherette-wasm-old`. This crate is intentionally minimal while we spec the new design before porting behavior back in.

## Initial design goals

- Keep public Wasm/API wrappers thin and stable.
- Separate correctness references from optimized production code.
- Make resize/filter choices explicit presets, not accidental module coupling.
- Keep shared code limited to invariants, data shapes, and neutral math helpers.
- Let each optimized filter own its hot path and tiling plan.
- Add benchmarks only after the API and correctness oracle are clear.

## Proposed future layout

```text
src/
  lib.rs
  wasm.rs
  error.rs
  image/
    dimensions.rs
    rgba.rs
  resize/
    mod.rs
    nearest.rs
    area.rs
    bilinear.rs
    bicubic.rs
    lanczos.rs
    trilinear.rs
    reference/
    scalar/
    shared/
    tiling/
```
