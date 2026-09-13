# Supplied palette and alpha reference

`PreparedPalette` prepares an ordered palette after `Request::validate` succeeds.
It owns its normalized metadata. It never borrows mutable caller storage.

## Ordered entries

Retain the first 256 entries, including duplicates and every transparent entry.
Indices always refer to these original positions. Visible entries contain their original index and byte RGB.
Matching visits visible entries in this order and replaces a winner only for a strictly smaller distance.
Transparent entries never participate in visible matching.

Normalized visible entries are `[r, g, b, 255]`; transparent entries are `[0, 0, 0, 0]`.
The transparent index identifies the first transparent entry, including index 255.
Truncation happens before transparency discovery and fallback selection.

An all-transparent palette maps every pixel to its first entry, regardless of alpha policy.
With visible entries but no transparent entry, preserve mode uses the darkest visible entry for thresholded pixels.
Darkest means the smallest integer `r + g + b`, with the first exact tie winning.
This fallback is not a perceptual nearest-color operation.

## Byte alpha before indexed matching

For source `[r, g, b, a]`, opacity is `a / 255`.
All arithmetic in this section uses f64, matching TypeScript numbers.
The resulting RGB bytes enter the selected f32 color conversion only afterward.

- Preserve uses the inclusive comparison `a <= threshold`. Matching otherwise receives the unchanged source RGB.
- Premultiplied uses `round(channel * opacity)` for each RGB channel.
- Matte uses `round(channel * opacity + matteChannel * (1 - opacity))`, clipped to byte range.

All rounded values are nonnegative, so Rust's half-away-from-zero rounding matches JavaScript's half-up rounding here.
The threshold retains f64 precision. With threshold `127.9999999`, alpha 128 survives; rounding that threshold to f32 changes the result.

For example, premultiplying `[200, 100, 50, 128]` yields `[100, 50, 25]`.
Compositing `[255, 0, 0, 128]` over blue yields `[128, 0, 127]`.
With alpha zero, premultiplied mode matches black and matte mode matches the matte RGB.
Neither mode selects transparency from source alpha.

`PalettePixel::Index` bypasses matching and dithering.
Diffusion must discard incoming error at that pixel and emit no outgoing error.
This includes darkest-visible fallback indices, even though their palette alpha is opaque.

Standalone `perturb` does not apply this indexed policy.
It processes source RGB, including hidden RGB, and reconstructs RGBA8 with the original byte alpha.
Thus a zero-strength sRGB perturb of `[200, 100, 50, 0]` retains all four bytes.
Later quantization applies its alpha policy to that reconstructed RGBA8 image.
The color inverse adapters carry byte alpha; the perturb slices exercise their field composition.

## Warnings and ownership

Warnings appear in this order. The transparent-only case returns before fallback handling.

| Condition | Code | Exact message |
|---|---|---|
| More than 256 supplied entries | `palette-truncated` | Palette was truncated to 256 entries for indexed PNG export. |
| No retained visible entry | `transparent-only` | Only Transparent is enabled; every output pixel is transparent. |
| Preserve mode without retained Transparent | `transparent-fallback` | Transparent is disabled; alpha-thresholded pixels use the darkest enabled visible color. |

Fallback warnings describe preparation, even when every source pixel happens to be opaque.
The package receives concrete matte RGB. Frontend matte-key selection and its disabled/unavailable warnings remain website responsibilities.
The package does not replace an explicitly supplied matte with an enabled palette color.

`into_indexed` moves the normalized palette and warnings into the owned index result after matching succeeds.
It does not expose a partial result during request validation or preparation.

## Authority and fixtures

The approved behavior comes from [decision 35](https://github.com/mia-cx/ditherette/issues/35)
and [decision 40](https://github.com/mia-cx/ditherette/issues/40).
The website's `src/lib/processing/quantize-shared.ts` defines truncation, fallback selection, and warning wording.
`src/lib/processing/compositing.ts` and `color.ts` define compositing and byte rounding.
`src/lib/processing/quantize-algorithms/direct-runner.ts` and `error-diffusion-runner.ts` define threshold bypass behavior.
`src/lib/processing/png.ts` defines normalized transparent palette bytes.

`tests/spec_palette.rs` uses tiny explicit byte fixtures and checks invalid request forms and durable metadata.
It does not invoke production kernels or compare the reference to itself.
