# resize coordinate mapping spec

## Purpose

`coordinates.rs` maps output coordinates into continuous source pixel-index space for filters that sample neighborhoods instead of selecting one integer source coordinate directly.

## Inputs and outputs

Input:

```rust
output_coordinate
source_len
output_len
AxisAlignment
```

Output:

```rust
f64 source_position
```

Source pixel centers are at integer coordinates:

```text
0, 1, 2, ... source_len - 1
```

## Algorithm / semantic rule

Start alignment:

```text
source_position = output * source_len / output_len
```

Center alignment:

```text
source_position = (output + 0.5) * source_len / output_len - 0.5
```

End alignment:

```text
source_position = (output + 1) * source_len / output_len - 1
```

## Why this works this way

Continuous filters need positions, not just integer samples. Keeping the mapping in one spec file ensures bilinear, convolution-based bicubic/Lanczos, and trilinear delegation agree on the meaning of anchors.

## Correctness invariants

- Source positions are expressed in source pixel-center coordinates.
- Axis alignment is independent for x and y.
- Center alignment preserves old Ditherette center-based behavior.

## Edge cases

- Positions may be outside the source image for start/end alignment during upscales.
- Individual filters decide how to handle out-of-bounds sampling. Current specs use edge extension by clamping source indices.

## Production obligations

Production coordinate maps must match these formulas for exact modes.

## Non-goals

- No arbitrary subpixel offset API yet.
- No per-filter coordinate exceptions.
