# resize spec

## Purpose and scope

`spec/resize` defines readable, executable resize semantics for Ditherette.

It owns the oracle behavior for coordinate mapping, edge handling, sampling, accumulation, and rounding. Future production resize implementations may precompute plans, specialize kernels, tile output rows, or use SIMD, but exact modes must produce the same bytes as these specs.

## Inputs and outputs

Resize specs operate on validated `image` views:

```rust
ImageView<'_, F>
ImageViewMut<'_, F>
```

where `F` is a packed format marker such as `Rgba8`, `Oklab32`, or `PaletteIndex8`.

Specs stay generic so they can document the math independently of storage choices. Ditherette production resize is narrower: all production resize filters operate on normalized packed `Rgba8`. HDR or non-RGBA inputs must be converted at the boundary before resize.

Specs should not accept raw `(data, width, height)` triples. Boundary wrappers validate raw buffers and construct views before calling resize specs or future production code.

## Module map

```text
common/
  alignment.rs   resize axis/anchor mapping semantics

scalar/
  nearest.rs     direct nearest-neighbor sampling
```

Planned scalar specs:

```text
scalar/area.rs        exact coverage/integration resize
scalar/bilinear.rs    triangle-filter resize
scalar/convolution.rs readable finite-support convolution helper for bicubic/lanczos
scalar/bicubic.rs     cubic resize preset
scalar/lanczos.rs     Lanczos2/Lanczos3 semantics
scalar/trilinear.rs   mip/policy resize semantics
```

## Domain model

Resize maps every output pixel to source image content using global image coordinates. The output domain is never treated as a cropped tile in spec code.

For nearest, each output coordinate maps to exactly one source coordinate. For filters like bilinear, bicubic, and Lanczos, each output coordinate maps to a weighted source neighborhood.

## Correctness philosophy

Spec resize code should make coordinate behavior obvious. It should prefer direct loops and local calculations over contribution caches or optimized layouts.

Future production obligations:

- match spec output byte-for-byte for exact modes
- use the same coordinate mapping and tie-breaking
- keep any fast/tolerance mode as a distinct public contract
- use global coordinates when tiled

## Alignment policy

Resize alignment is expressed as explicit in-axis anchors, not as vague whole-image endpoint modes.

Nearest currently supports a 3x3 `ResizeAnchor` model:

```text
TopLeft      Top      TopRight
Left         Center   Right
BottomLeft   Bottom   BottomRight
```

Each anchor maps to independent x/y `AxisAlignment`s:

```text
Start
Center
End
```

`ResizeAnchor::Center` preserves the old Ditherette nearest behavior and remains the default.

## Edge cases and invariants

Resize specs should cover:

- identity resize
- 1x1 input
- 1-pixel output axes
- upsample
- downsample
- non-square dimensions
- odd dimensions
- alignment differences
- packed formats with different storage/channel counts
- strided inputs/outputs where relevant

## Future production obligations

Production resize may:

- precompute x/y maps
- reuse row scratch
- specialize exact ratios
- copy rows/spans
- tile output row bands
- use SIMD

Production resize must not:

- change exact coordinate semantics
- clamp to tile-local input coordinates
- import spec control flow as the production implementation
- silently replace exact modes with approximate/tolerance modes

## Non-goals / deferred choices

- No production optimization in spec code.
- No generic interpolation framework until multiple filters prove the shape.
- No source-cropping tile model.
- No tolerance-based nearest mode until a real use case exists.
