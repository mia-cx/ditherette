# nearest resize spec

## Purpose

`nearest.rs` defines the simplest exact nearest-neighbor resize oracle.

It copies one complete source pixel into each output pixel. It is generic over packed image formats, so the same spec applies to RGBA bytes, Oklab float channels, and indexed palette images.

## Inputs and outputs

Input:

```rust
ImageView<'_, F>
```

Output:

```rust
ImageViewMut<'_, F>
```

where `F: ImageFormat`.

The source and output must use the same packed format. This keeps nearest resize a storage-preserving operation: it changes dimensions, not color representation.

## Algorithm / semantic rule

For each output pixel:

1. Resolve the selected `ResizeAnchor` into x/y `AxisAlignment`s.
2. Map output `y` to source `y` using the y-axis alignment.
3. Map output `x` to source `x` using the x-axis alignment.
4. Copy `F::CHANNEL_COUNT` flat storage elements from the source pixel to the output pixel.

No interpolation, weighting, rounding, or color conversion occurs.

## Alignment modes

Nearest uses the 3x3 anchor model from `alignment.rs`:

```text
TopLeft      Top      TopRight
Left         Center   Right
BottomLeft   Bottom   BottomRight
```

Each anchor is just x/y axis alignment:

```text
Start   Center   End
```

`Center` is the default and matches the old Ditherette nearest path.

There is intentionally no `Corners` mode. Endpoint-aligned whole-image resizing and in-pixel anchor selection are different concepts; nearest uses explicit pixel/axis anchors.

## Why this works this way

Nearest resize is a coordinate selection problem. The spec keeps it as a direct output loop so readers can see exactly which source pixel is selected for every output pixel.

Runtime anchors are part of the semantic contract because different workflows may want different sampling anchors:

- `Center` is image-library-like and preserves old Ditherette behavior.
- `TopLeft` is useful for start-edge anchoring.
- `BottomRight` is useful for end-edge anchoring.
- centered-edge anchors (`Top`, `Right`, `Bottom`, `Left`) are available without adding special cases.

The spec intentionally does not precompute x/y maps. A future production scalar implementation can do that later while preserving the same formulas.

## Correctness invariants

- Output pixels are copied from exactly one source pixel.
- All channels for a pixel come from the same source coordinate.
- Source and output formats are identical.
- No channel values are changed.
- `ResizeAnchor::Center` matches the documented old nearest coordinate mapping.
- X and Y alignment are independent.
- Tiled production must use global output coordinates, not tile-local coordinates.

## Edge cases

Tests should cover:

- 1x1 source
- 1-pixel output axis
- identity resize
- upsample
- downsample
- non-square resize
- start/center/end axis differences
- all 3x3 anchors, at least through mapping tests
- packed u8 formats
- packed f32 formats
- strided buffers when row-band production is added

## Future production obligations

Future production nearest may:

- copy identity outputs directly
- copy same-width rows
- precompute x/y source indices
- precompute byte/storage offsets
- specialize exact integer ratios
- use span copies
- tile output row bands

Future production nearest must:

- match this spec byte-for-byte for exact modes
- implement anchor modes with identical formulas and tie-breaking
- copy whole pixels, not individual channels from different coordinates
- avoid tile-local clamping or source cropping

## Non-goals

- No interpolation.
- No antialiasing.
- No tolerance mode.
- No production fast paths in spec code.
- No precomputed mapping tables in spec code.
