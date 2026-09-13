# Oklab round-trip recipe

`rgb8_to_oklab` decodes non-HDR sRGB bytes and applies the inherited f32 Oklab matrix and signed cube roots.
`oklab_to_rgb8` applies the inverse matrix, cubes the cone coordinates, and encodes sRGB.
The matrices follow [Ottosson's 2021-01-25 linear-sRGB recipe](https://bottosson.github.io/posts/oklab/#converting-from-linear-srgb-to-oklab), with constants rounded to f32.
No D50 adaptation occurs; Oklab uses D65.

The image adapters write packed f32 `[L,a,b]` triples and reconstruct packed RGBA8.
The inverse receives a corresponding RGBA8 image solely for byte alpha. It copies alpha unchanged, including zero and partial alpha.
Hidden RGB remains color data; conversion does not premultiply, composite, or erase it.
Source, alpha, and output views must have matching dimensions. Their row strides may differ.

Lightness has nominal black/white coordinates 0 and 1. Opponent coordinates are signed, unitless values with neutral at zero.
Forward f32 matrix rounding can leave tiny neutral opponent residuals; this cartesian recipe retains them.
OKLCH canonicalizes exact byte grays separately, as documented in [oklch.md](oklch.md).

Inverse coordinates may extend beyond the sRGB gamut. Inputs and intermediate arithmetic must remain finite.
The inverse does not clip L, a, b, cone responses, or linear RGB.
After sRGB encoding, each channel clips to `[0,1]`, multiplies by 255, and rounds to the nearest byte with half ties upward.
This is channel clipping, not perceptual gamut mapping. Out-of-gamut round trips therefore lose information.

For domain derivation, nonnegative forward cone weights sum to approximately one, so byte-input cone roots lie within `[0,1.000001]`.
Applying coefficient signs independently gives conservative f32 enclosures `L∈[-0.01,1.01]`, `a∈[-2.5,2.5]`, `b∈[-1,1]`.
These are enclosing boxes, not tight sRGB extrema or recommended placement ranges. S12 derives its declared normalization ranges before freeze.

Tests use independently evaluated primary-color coordinates, every byte gray, a stratified RGB cube, and inverse clipping cases.
