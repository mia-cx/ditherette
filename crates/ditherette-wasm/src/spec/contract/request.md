# Request validation spec

## Inputs and outputs

`request.rs` owns typed requests for process, resize, perturb, quantize, and ditherAndQuantize.
Sources contain dimensions and a borrowed packed RGBA8 byte slice.
Process and resize specify output dimensions. The other methods preserve source dimensions.
Successful validation returns a borrowed source view and output dimensions, not an allocated output image.

## Validation rules

Each request first validates its version and applicable palette, alpha, perturbation, or dither settings.
Validation then checks source dimensions, exact packed byte length, and output dimensions, in that order.
Version one is the only supported version. Recipe decoding rejects unknown fields and malformed tagged combinations.
Raw JavaScript property types remain the boundary adapter's responsibility.

Source sides must be between 1 and 32,768 pixels. Output sides must be between 1 and 16,384 pixels.
Both layouts have a 67,108,864-pixel limit. Arithmetic checks precede image allocation.
Invalid source dimensions or byte lengths report `InvalidImage` at the offending `source` path.
For perturb, quantize, and ditherAndQuantize, the output limit also applies to the source.
Those failures report `InvalidImage` at `source.width` or `source.height`.
Explicit output limits report `InvalidSettings` at an `output` path.
A source wider than the output limit remains valid for a resize to supported output dimensions.

An empty palette is invalid. Transparent-only and oversized palettes remain valid for S09 normalization.
Alpha thresholds must be finite and within 0..255. Strength, adaptive threshold, and softness must be finite and nonnegative.
Adaptive radius must be between 1 and 32,768 pixels.
Typed enums encode valid resize settings and color/metric combinations.
Diffusion uses sRGB feedback or the matching working space; sRGB feedback with perceptual matching remains valid.

## Correctness and production obligations

Validation preserves input storage and returns the first failure in the stated order.
It rejects unsupported sizes rather than clamping them.
Production adapters must preserve codes, field paths, accepted settings, and borrowed-input ownership.
This module does not resize, normalize palettes, perturb pixels, or select production implementations.
