# Coverage-weighted resize

## Purpose

Filtering kernels average neighbouring pixels. Averaged independently, a transparent pixel's hidden RGB, usually black, bleeds into visible edges. Kernels with negative lobes also ring in alpha, giving fully transparent areas small alpha values that later alpha thresholds treat as opaque. This resize weights colour by coverage and keeps alpha from ringing into empty space.

## Inputs and outputs

The v1 `ResizeRequest`: RGBA8 source, output size, and resize policy. Returns owned RGBA8 at the output size. Validation and error paths are v1's.

## Algorithm / semantic rule

1. Validate the request exactly like v1 `resize`.
2. If the policy is nearest, the output size equals the source size, or every source alpha is 255, return v1 `resize`, which is an exact copy at equal size.
3. Premultiply into four `f32` channels: `c * (a / 255)` for each colour channel, computed in `f64` and stored as `f32`; alpha stays `a`.
4. Run the policy's v1 kernel, unchanged, on that carrier with the same anchor and support. `f32` storage keeps every intermediate unrounded, including trilinear mips.
5. For bicubic and Lanczos, find each output pixel's main lobe: source taps whose distance from the mapped position is below one kernel unit on both axes (scaled like the kernel's support, clamped at edges). Clamp the filtered alpha to the lowest and highest source alpha among those taps.
6. Output alpha is the clamped alpha, rounded to a byte like v1. When that byte is 0, or the unclamped filtered alpha is not positive, colour is black. Otherwise each colour channel is `filtered_c * 255 / filtered_a` using the unclamped alpha, rounded to a byte like v1.

## Why this works this way

Dividing premultiplied colour by filtered alpha is the same as weighting each tap's colour by its alpha, so fully transparent taps contribute no colour. Running the unchanged v1 kernels on a premultiplied carrier keeps every kernel's coordinate, support, and mip rules identical.
The main-lobe clamp is the usual anti-ringing rule applied to coverage only: a transparent neighbourhood stays transparent, while edges keep their full range. Colour keeps the kernel's sharpening.
Area, bilinear, and trilinear have no negative weights, so their alpha cannot leave the tap range and needs no clamp.
Opaque sources skip the carrier, so their bytes stay identical to v1.

## Correctness invariants

- Nearest, equal-size, and fully opaque requests equal v1 `resize` byte for byte.
- A fully transparent source pixel's RGB never affects any output byte.
- Where every main-lobe tap is fully transparent, bicubic and Lanczos output `[0, 0, 0, 0]`.
- Output alpha never exceeds the largest source alpha in the main lobe.

## Edge cases

- At any other size, a fully transparent source resizes to all `[0, 0, 0, 0]`.
- A single translucent pixel sends the whole image through the carrier; opaque regions may then differ from v1 by rounding only.

## Production obligations

Production must match these bytes. It may keep its fast path for nearest and fully opaque sources, and route other sources to an exact copy of this carrier path.

## Non-goals

Filtering in linear light, alpha-aware nearest, and matte or threshold decisions, which stay in quantization.
