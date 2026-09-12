# area resize spec

## Purpose

`area.rs` defines exact coverage/integration resize semantics.

## Inputs and outputs

Input and output are validated image views with the same packed format:

```rust
ImageView<'_, F>
ImageViewMut<'_, F>
```

`F::Storage` must implement `ResizeSample` so channels can accumulate as `f64` and convert back to storage.

## Algorithm / semantic rule

Each output pixel covers a rectangle in source pixel space:

```text
x: output_x * source_width / output_width .. (output_x + 1) * source_width / output_width
y: output_y * source_height / output_height .. (output_y + 1) * source_height / output_height
```

For every overlapped source pixel, compute x/y interval overlap, multiply them into area weight, accumulate channels, and divide by the output rectangle's source-space area.

## Why this works this way

Area resize is a coverage problem, not a reconstruction-kernel problem. Direct interval overlap is the clearest oracle for preserving average coverage during downscales and for defining exact behavior during arbitrary ratios.

## Correctness invariants

- Weights are source-area coverage fractions.
- All channels of a source pixel use the same area weight.
- Output channels are normalized by the output source-space area.
- Sampling uses edge extension for any numerical boundary spill.

## Edge cases

- identity resize
- exact integer downscale
- non-integer downscale
- upscale
- one-pixel source axes
- odd dimensions
- byte and float storage formats

## Production obligations

Production area implementations may precompute spans, use integer arithmetic, reuse accumulators, or tile output rows. Exact modes must match this coverage result.

## Non-goals

- No separable convolution abstraction.
- No approximate box filter shortcut.
- No alpha/premultiplication policy yet.
