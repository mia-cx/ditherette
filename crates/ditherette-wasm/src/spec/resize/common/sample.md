# resize sample conversion spec

## Purpose

`sample.rs` defines how scalar storage elements are converted into resize accumulators and back out to storage.

## Inputs and outputs

Input storage types:

```rust
u8
f32
```

Accumulator type:

```rust
f64
```

## Algorithm / semantic rule

- `u8` samples convert to `0.0..=255.0` and convert back with clamp-then-round.
- `f32` samples convert losslessly enough for spec arithmetic through `f64` and convert back with `as f32`.

## Why this works this way

Resize specs should express filter math directly without duplicating rounding behavior in every filter. `f64` keeps the oracle simple and stable. The storage conversion is explicit, small, and semantic rather than optimized.

## Correctness invariants

- `u8` outputs are always clamped into byte range.
- `u8` outputs round to nearest using Rust/f64 `.round()` behavior.
- `f32` outputs are not clamped because f32 image formats may represent linear or perceptual values outside byte range during intermediate processing.

## Edge cases

- negative byte accumulations clamp to `0`
- byte accumulations above `255` clamp to `255`
- `.5` byte values round away from zero via `f64::round`

## Production obligations

Production code may use narrower accumulators or SIMD lanes if exact modes match these conversion results.

## Non-goals

- No gamma/color-space conversion.
- No premultiplied-alpha policy.
- No tolerance-based rounding mode.
