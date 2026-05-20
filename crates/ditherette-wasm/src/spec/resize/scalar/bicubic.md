# bicubic resize spec

## Purpose

`bicubic.rs` defines Ditherette's bicubic resize oracle as a Catmull-Rom cubic reconstruction filter.

## Inputs and outputs

Input and output are validated image views with the same packed format. Storage channels must implement `ResizeSample`.

The caller chooses:

```rust
ResizeAnchor
SupportPolicy
```

## Algorithm / semantic rule

Bicubic delegates sampling to `convolution.rs` using a cubic kernel with radius `2` and parameter `a = -0.5`.

Kernel:

```text
0 <= |x| < 1:
  (a + 2)|x|^3 - (a + 3)|x|^2 + 1

1 <= |x| < 2:
  a|x|^3 - 5a|x|^2 + 8a|x| - 4a

2 <= |x|:
  0
```

## Why this works this way

Catmull-Rom is a common bicubic preset and gives a precise, maintainable default. Keeping the cubic formula in this file and the sampling loop in `convolution.rs` makes the spec easy to audit.

## Correctness invariants

- Kernel radius is `2`.
- Parameter `a` is `-0.5`.
- Sampling, edge extension, and normalization are inherited from `convolution.rs`.
- Fixed vs scale-aware support is explicit through `SupportPolicy`.

## Edge cases

- kernel support crossing edges
- upscales
- downscales
- byte and float formats

## Production obligations

Production bicubic may specialize four-tap paths, precompute contributions, or use SIMD. Exact modes must match this kernel and convolution contract.

## Non-goals

- No Mitchell/Robidoux variants yet.
- No configurable cubic parameter yet.
- No color-space or alpha policy.
