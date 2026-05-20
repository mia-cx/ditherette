# convolution resize spec

## Purpose

`convolution.rs` defines the shared scalar spec engine for finite-support separable reconstruction kernels.

It is intended for bicubic, Lanczos, and future windowed kernels such as Mitchell, Robidoux, Spline36, or Gaussian.

## Inputs and outputs

Input and output are validated image views with the same packed format. A caller supplies:

```rust
ReconstructionKernel
ResizeAnchor
SupportPolicy
```

## Algorithm / semantic rule

For each output pixel:

1. Map x/y to continuous source positions.
2. Determine kernel support for x/y.
3. Evaluate the 1D kernel for every source sample in support.
4. Multiply x/y weights into a separable 2D weight.
5. Accumulate all channels.
6. Normalize by total weight.

## Why this works this way

Bicubic and Lanczos differ mostly by kernel function and radius. The readable oracle can share the loop that applies finite-support separable kernels while keeping production-style contribution planning out of spec code.

## Correctness invariants

- The kernel is evaluated independently on x and y axes.
- Source edges use edge extension by clamping indices.
- All channels of a source pixel use the same product weight.
- Results are normalized by the actual summed weights.
- Fixed and scale-aware support are explicit semantic choices.

## Edge cases

- one-pixel source axes
- upscales
- downscales
- kernel support crossing image edges
- byte and float formats

## Production obligations

Production convolution code may precompute contributions, split horizontal/vertical passes, reuse scratch, specialize tap counts, or use SIMD. Exact modes must match this direct kernel evaluation.

## Non-goals

- No nearest, area, or trilinear behavior.
- No EWA/radial 2D kernels.
- No production contribution cache.
- No color-space or alpha policy.
