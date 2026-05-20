# image spec

## Purpose and scope

`image` defines Ditherette's minimal image representation layer.

It owns:

- dimensions
- row stride
- packed format definitions
- borrowed image views
- owned image buffers
- storage/layout validation
- safe row, pixel, and channel access helpers

It does not own:

- resizing algorithms
- color conversion algorithms
- palette creation
- quantization
- dithering
- tiling schedules
- file decoding/encoding
- destructuring flat buffers into per-pixel objects

The image module exists so every processing domain can accept the same image-shaped storage abstraction while still keeping hot-path data as flat packed arrays.

## Design goals

1. **Flat storage first**: canonical buffers stay as `[r, g, b, a, r, g, b, a, ...]`, not `[{r,g,b,a}, ...]`.
2. **Typed layout, not runtime guessing**: internal APIs use `ImageView<'_, Rgba8>` or `ImageView<'_, Oklab32>` instead of runtime color-space enums.
3. **No content transforms**: image code validates and indexes storage; it never changes color, size, palette, or dither state.
4. **Stride-aware**: views support padded rows and output row bands without copies.
5. **Small enough to audit**: this is representation infrastructure, not a full image framework.

## File map

```text
src/image/
  spec.md          # this document
  mod.rs           # exports image representation types
  dimensions.rs    # ImageDimensions and checked size math
  stride.rs        # RowStride in scalar storage elements
  view.rs          # ImageView<F>, ImageViewMut<F>
  owned.rs         # ImageBuf<F>
  pixel.rs         # StorageElement marker trait
  formats.rs       # packed format marker types and channel constants
  validate.rs      # layout errors and checked validation helpers
```

## Core model

The central idea is:

```rust
ImageView<'a, F>
```

where `F` is a packed format marker.

Examples:

```rust
ImageView<'_, Rgba8>        // &[u8] laid out RGBA RGBA RGBA...
ImageView<'_, Rgb8>         // &[u8] laid out RGB RGB RGB...
ImageView<'_, Oklab32>      // &[f32] laid out L a b L a b...
ImageView<'_, PaletteIndex8>// &[u8] laid out index index index...
```

The format marker defines storage type, channel count, and channel order. The view stores a flat slice of that storage type.

## Dimensions

```rust
pub struct ImageDimensions {
    width: u32,
    height: u32,
}
```

Invariants:

- `width > 0`
- `height > 0`
- pixel count must fit in `usize`
- format-specific storage and byte lengths are checked

Recommended API shape:

```rust
impl ImageDimensions {
    pub fn new(width: u32, height: u32) -> Result<Self, ImageLayoutError>;
    pub fn width(self) -> u32;
    pub fn height(self) -> u32;
    pub fn pixel_count(self) -> Result<usize, ImageLayoutError>;
    pub fn storage_len<F: ImageFormat>(self) -> Result<usize, ImageLayoutError>;
    pub fn byte_len<F: ImageFormat>(self) -> Result<usize, ImageLayoutError>;
}
```

## Formats

`formats.rs` defines packed layout contracts, not per-pixel structs.

```rust
pub trait ImageFormat {
    type Storage: StorageElement;
    const CHANNEL_COUNT: usize;
    const NAME: &'static str;
}
```

Initial marker types:

```rust
pub enum Rgba8 {}        // u8, 4 channels: R G B A
pub enum Rgb8 {}         // u8, 3 channels: R G B
pub enum LinearRgb32 {}  // f32, 3 channels: R G B
pub enum LinearRgba32 {} // f32, 4 channels: R G B A
pub enum Oklab32 {}      // f32, 3 channels: L a b
pub enum Oklaba32 {}     // f32, 4 channels: L a b alpha
pub enum PaletteIndex8 {}// u8, 1 channel: index
```

Each marker exposes channel constants:

```rust
Rgba8::R
Rgba8::G
Rgba8::B
Rgba8::A

Oklab32::L
Oklab32::A
Oklab32::B
```

Why marker types instead of pixel structs:

- hot loops can index flat storage directly
- browser `ImageData` can be used as `[u8]` without reinterpretation
- no per-pixel destructuring/copying is implied by the type system
- color-space correctness is still encoded at compile time

## Storage elements

`pixel.rs` defines `StorageElement`:

```rust
pub trait StorageElement: Copy + Default + 'static {}
```

Initial storage elements:

```rust
u8
f32
```

This trait has no color semantics. It only marks scalar storage types that can live in image buffers.

## Stride

Stride is measured in scalar storage elements, not pixels and not bytes.

Examples for width `W`:

```text
Rgba8 packed stride        = W * 4
Rgb8 packed stride         = W * 3
Oklab32 packed stride      = W * 3
PaletteIndex8 packed stride = W
```

```rust
pub struct RowStride {
    elements: usize,
}
```

Invariants:

- `stride >= width * F::CHANNEL_COUNT`
- `stride * (height - 1) + row_len <= data.len()`
- all arithmetic is checked

## Views

Immutable view:

```rust
pub struct ImageView<'a, F: ImageFormat> {
    data: &'a [F::Storage],
    dimensions: ImageDimensions,
    stride: RowStride,
}
```

Mutable view:

```rust
pub struct ImageViewMut<'a, F: ImageFormat> {
    data: &'a mut [F::Storage],
    dimensions: ImageDimensions,
    stride: RowStride,
}
```

Views validate the buffer layout once at construction. After construction, algorithms can trust:

- dimensions are non-zero
- row length fits
- stride covers the logical row
- backing storage contains every logical row

Recommended accessors:

```rust
view.row(y)              // logical row, excluding padding
view.pixel(x, y)         // channel slice for one logical pixel
view.channel(x, y, c)    // copied scalar channel
view_mut.row_mut(y)
view_mut.pixel_mut(x, y)
view_mut.set_channel(x, y, c, value)
```

Rows and pixels are slices into flat storage; no `{ r, g, b, a }` objects are created.

## Owned buffers

```rust
pub struct ImageBuf<F: ImageFormat> {
    data: Vec<F::Storage>,
    dimensions: ImageDimensions,
    stride: RowStride,
}
```

Default owned buffers are packed:

```text
stride == width * F::CHANNEL_COUNT
len == width * height * F::CHANNEL_COUNT
```

Owned buffers provide borrowed views through the same validation semantics:

```rust
buf.as_view()
buf.as_view_mut()
```

## Wasm/browser interop

Browser `ImageData` is tightly packed RGBA bytes. It maps directly to:

```rust
ImageView<'_, Rgba8>
```

with:

```text
data: &[u8]
stride: width * 4
layout: R G B A
```

Wasm wrappers may receive raw `&[u8]`/`&mut [u8]`, validate dimensions and storage length once, then construct typed views. They should not copy or reinterpret the bytes into pixel structs.

## Planar and indexed images

The first abstraction is interleaved packed channel storage.

Planar data can be represented as multiple single-channel views if needed later:

```rust
struct LabPlanes<'a> {
    l: ImageView<'a, PlaneF32>,
    a: ImageView<'a, PlaneF32>,
    b: ImageView<'a, PlaneF32>,
}
```

Do not build a general planar framework until a real algorithm needs it.

Indexed output uses:

```rust
ImageViewMut<'_, PaletteIndex8>
```

with a separate `Palette<T>` type owned by palette/quantize modules, not by `image`.

## Validation and errors

Image validation errors should be precise and reusable across domains.

Example variants:

```rust
pub enum ImageLayoutError {
    ZeroWidth,
    ZeroHeight,
    PixelCountOverflow,
    StorageLengthOverflow,
    ByteLengthOverflow,
    ChannelCountZero,
    ChannelOutOfRange { channel: usize, channel_count: usize },
    StrideTooSmall { stride: usize, row_len: usize },
    BufferTooShort { len: usize, required: usize },
    BufferLengthMismatch { len: usize, expected: usize },
}
```

Public domain errors can wrap image layout errors.

## Correctness invariants

Once an `ImageView` or `ImageViewMut` exists:

- dimensions are non-zero
- format channel count is non-zero
- stride is at least the logical row length
- all rows are addressable
- row slices include logical pixels only, not padding
- packed constructors require exact packed storage length
- no processing module needs to revalidate raw layout

## Edge cases

Image module tests must cover:

- zero width rejected
- zero height rejected
- pixel count overflow rejected
- storage length overflow rejected
- byte length overflow rejected
- packed length too short/long rejected
- strided view accepts valid padding
- stride smaller than row length rejected
- final row bound calculation checked
- row access excludes padding
- pixel/channel access respects channel constants
- mutable channel writes update flat storage without destructuring
- `Rgba8` channel order matches browser `ImageData`

## Tiling relationship

Top-level `tiling` depends on image dimensions/shape, not format markers.

Allowed relationship:

```text
tiling -> image::dimensions
```

Disallowed relationship:

```text
tiling -> image::formats::Rgba8
tiling -> color spaces
tiling -> domain algorithms
```

Domain tiling adapters may use full image views and split mutable outputs into row bands.

Tiling partitions output writes only. It does not crop input images.

## Production obligations

All production domains should accept validated image views instead of raw width/height/data triples where practical.

When a public API receives raw buffers, it should:

1. validate dimensions
2. validate format-specific storage length/layout
3. construct typed image views
4. call the domain implementation

Production modules may add optimized row iteration helpers, but those helpers belong in the domain or top-level tiling layer, not in `image`, unless they are purely about safe image layout.

## Non-goals

The image module is not:

- an image decoder
- an image encoder
- a color management engine
- a metadata container
- a general-purpose ndarray
- a full planar image framework
- a dynamic runtime image format registry
- an algorithm dispatch layer
- a per-pixel struct conversion layer

Keep it small. Its job is to make illegal memory/layout states hard to express and to give every processing domain a common typed flat-buffer vocabulary.
