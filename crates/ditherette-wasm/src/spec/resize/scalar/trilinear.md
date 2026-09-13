# trilinear resize spec

## Purpose

`trilinear.rs` defines mipmapped trilinear resize semantics.

## Inputs and outputs

Input and output are validated image views with the same format. Logical rows may contain backing-storage padding; it is never image content. Storage channels must implement `ResizeSample`.

The caller chooses a `ResizeAnchor` used by the bilinear resizes from mip levels.

## Algorithm / semantic rule

1. Compute minification as the larger source/output scale across x and y.
2. If minification is not greater than `1`, delegate directly to bilinear.
3. Copy logical source rows into packed mip storage. Build levels by repeatedly halving dimensions with round-up and resizing with exact area.
4. Compute level of detail:
   ```text
   lod = log2(minification)
   ```
5. Bilinear-resize the lower and upper mip levels to the output dimensions.
6. Blend those two outputs by the fractional part of `lod`.

## Why this works this way

Trilinear is not a kernel like Lanczos or bicubic. It is a policy over mip levels. Area creates the pyramid levels because it gives a clear coverage-preserving definition for each downsample step; bilinear samples between levels because trilinear conventionally interpolates within each selected level.

## Correctness invariants

- Mip dimensions halve with round-up and never reach zero.
- Minification uses the largest axis scale.
- Magnification and same-size cases use bilinear directly.
- Mip-level blending is per channel after the two bilinear outputs are produced.
- Each area mip and bilinear output uses its declared storage rounding before the final blend.
- Bilinear is the scale-aware triangle reference, including when sampling between mip levels.

## Edge cases

- identity resize
- magnification
- exact power-of-two minification
- fractional minification between mip levels
- one-pixel source axes
- byte and float formats

## Production obligations

Production trilinear may cache mip pyramids, reuse scratch outputs, or tile per-level resizes. Exact modes must match the area pyramid, bilinear sampling, and LOD blend defined here.

## Non-goals

- No custom mip filter.
- No anisotropic filtering.
- No production mip cache.
- No color-space or alpha policy.
