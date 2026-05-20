# bilinear resize spec

## Purpose

`bilinear.rs` defines triangle-filter resize semantics.

## Inputs and outputs

Input and output are validated image views with the same packed format. Storage channels must implement `ResizeSample`.

## Algorithm / semantic rule

For every output pixel:

1. Map x/y output coordinates to continuous source positions using `ResizeAnchor`.
2. Use a separable triangle filter:
   ```text
   weight(d) = max(0, 1 - |d|)
   ```
3. During minification, widen the support by the source/output scale.
4. Accumulate `x_weight * y_weight` for every covered source pixel.
5. Normalize by the total weight and convert channels back to storage.

## Why this works this way

This is the clearest mathematical statement of bilinear/triangle resampling. It is not a production two-tap shortcut: the widened support gives a stable oracle for downscales as well as upscales.

## Correctness invariants

- X and Y filtering are separable.
- All channels for a source pixel use the same product weight.
- Weights are normalized by the actual summed weight.
- Source edges use edge extension via clamping.
- `ResizeAnchor::Center` uses the same coordinate convention as nearest center alignment.

## Edge cases

- identity resize
- upscales
- downscales
- one-pixel source axes
- odd ratios
- byte and float formats

## Production obligations

Production may precompute contributions, use two-pass accumulation, specialize upscales, or use SIMD. Exact modes must match the normalized triangle result.

## Non-goals

- No production scratch reuse.
- No convolution helper dependency yet.
- No color-space or alpha policy.
