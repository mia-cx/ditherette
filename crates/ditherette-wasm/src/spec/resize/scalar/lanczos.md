# Lanczos resize spec

## Purpose

`lanczos.rs` defines Lanczos resize semantics as a windowed-sinc reconstruction filter.

## Inputs and outputs

Input and output are validated image views with the same packed format. Storage channels must implement `ResizeSample`.

The caller chooses:

```rust
ResizeAnchor
radius
SupportPolicy
```

Convenience wrappers exist for Lanczos2 and Lanczos3.

## Algorithm / semantic rule

Lanczos delegates sampling to `convolution.rs` with radius `a` and kernel:

```text
L(x) = sinc(x) * sinc(x / a),  if |x| < a
L(0) = 1
L(x) = 0,                     otherwise
```

where:

```text
sinc(x) = sin(pi*x) / (pi*x)
```

## Why this works this way

Lanczos is a family parameterized by support radius. The spec keeps radius explicit so Lanczos2 and Lanczos3 share one small, auditable implementation.

## Correctness invariants

- Radius must be greater than zero.
- Lanczos2 uses radius `2`.
- Lanczos3 uses radius `3`.
- Sampling, edge extension, and normalization are inherited from `convolution.rs`.
- Fixed vs scale-aware support is explicit through `SupportPolicy`.

## Edge cases

- zero distance uses weight `1`
- kernel support crossing edges
- upscales
- downscales
- byte and float formats

## Production obligations

Production Lanczos may precompute contributions, specialize tap counts, or use SIMD. Exact modes must match this sinc/window formula and convolution contract.

## Non-goals

- No EWA/radial Lanczos.
- No Jinc filter.
- No color-space or alpha policy.
