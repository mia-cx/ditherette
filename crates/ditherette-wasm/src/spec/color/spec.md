# color spec

## Purpose and scope

`spec/color` defines the readable, executable semantics for color-space conversion and color representation decisions used by Ditherette.

It owns the correctness contract for conversions such as:

- sRGB bytes to linear RGB
- linear RGB to sRGB bytes
- linear RGB to Oklab/Oklch
- Oklab/Oklch back to displayable RGB where needed

It does not own resize, palette selection, quantization, dithering, tiling schedules, or production SIMD/layout decisions.

## Storage layout caveat: AoS now, SoA later

The shared `image` abstraction currently represents packed interleaved channel buffers:

```text
Rgba8:   [r, g, b, a, r, g, b, a, ...]
Oklab32: [l, a, b, l, a, b, ...]
```

This is a good default for browser interop, resize, and simple stage composition. It keeps raw buffers flat and avoids per-pixel structs/destructuring.

However, color conversion, palette creation, quantization, and some dithering operations may eventually benefit from a structure-of-arrays layout:

```text
L plane: [l, l, l, ...]
a plane: [a, a, a, ...]
b plane: [b, b, b, ...]
```

SoA can be better for SIMD because each vector lane can process one channel without deinterleaving interleaved pixels first.

Do not forget this caveat when optimizing color-heavy paths.

## Policy

The spec implementation should stay layout-simple and readable. It may use packed `ImageView<'_, Rgba8>` / `ImageView<'_, Oklab32>` because spec prioritizes clarity over SIMD shape.

Production color code may introduce SoA intermediates later if benchmarks justify it. That should happen under `prod/color`, not by complicating `image` or `spec/color` prematurely.

Acceptable future production shape:

```rust
struct OklabPlanes<'a> {
    l: ImageView<'a, PlaneF32>,
    a: ImageView<'a, PlaneF32>,
    b: ImageView<'a, PlaneF32>,
}
```

or an owned scratch/intermediate equivalent inside `prod/color/common`.

## Correctness invariant

AoS and SoA production layouts must produce the same semantic color values as `spec/color` for exact modes. Layout is an optimization detail, not a behavior change.

## Non-goal

Do not build a general planar image framework until a measured production color/palette/quantize path needs it.
