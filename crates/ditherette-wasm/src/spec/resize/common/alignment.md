# resize alignment spec

## Purpose

`alignment.rs` defines how output coordinates are anchored when they map back into source coordinates.

It replaces vague terms like `Corners` with explicit axis anchors. Resize alignment is an in-axis/in-pixel sampling contract, not an optimized implementation detail.

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
source_coordinate
```

For 2D resize, `ResizeAnchor` provides a 3x3 set of x/y axis alignments.

## Algorithm / semantic rule

Axis alignments:

```text
Start   align start edge
Center  align pixel centers
End     align end edge
```

2D anchors:

```text
TopLeft      Start,  Start
Top          Center, Start
TopRight     End,    Start
Left         Start,  Center
Center       Center, Center
Right        End,    Center
BottomLeft   Start,  End
Bottom       Center, End
BottomRight  End,    End
```

Start mapping:

```text
source = floor(output * source_len / output_len)
```

Center mapping:

```text
source = floor((output + 0.5) * source_len / output_len)
```

End mapping:

```text
source = floor(((output + 1) * source_len - 1) / output_len)
```

All mappings clamp to `source_len - 1`.

## Why this works this way

Nearest resize selects one source sample per output sample. The chosen sample depends on what part of the output pixel we consider representative.

A 3x3 anchor model is explicit enough to express top-left, centered, bottom-right, and centered-edge behavior without pretending that endpoint alignment and pixel-anchor alignment are the same thing.

`Center` remains the default because it matches the old Ditherette nearest behavior.

## Correctness invariants

- Alignment is separable by axis.
- `ResizeAnchor` is only a convenience wrapper around two `AxisAlignment`s.
- Center mapping matches old Ditherette nearest semantics.
- Start and End are not named as whole-image corner alignment.
- Mapping never returns a coordinate outside `0..source_len`.

## Edge cases

- 1-pixel source axes always map to `0`.
- Upscales may map multiple output coordinates to one source coordinate.
- Downscales may skip source coordinates.
- Start/Center/End may coincide for some ratios; that does not make the modes equivalent.

## Production obligations

Production resize code must use the same formulas and anchor mapping for exact modes. It may precompute axis maps or specialize ratios, but those optimizations must preserve these coordinates.

## Non-goals

- No subpixel floating-point offsets yet.
- No arbitrary numeric anchor parameter yet.
- No endpoint-only `Corners` mode.
- No tiling behavior.
