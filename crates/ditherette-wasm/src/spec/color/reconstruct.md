# Field reconstruction

`reconstruct::coordinates_to_rgb8` evaluates the existing inverse equations with per-pixel f64 arithmetic.
Source forward conversions and any stored working-color images remain packed f32 triples. There are no f64 image planes.
The existing decimal matrix coefficients, D65 white, transfer curves, and negative-chroma neutral convention stay the same.
Hue reduction uses f64 TAU here because the field offset and inverse arithmetic are f64.
The original f32 inverse exports remain unchanged until the complete reference join reconciles shared helpers before S18.

The equations and primary-source provenance are in [Oklab](oklab.md), [CIELAB](cielab.md), [linear sRGB](linear.md), and [YCbCr](ycbcr.md).
This module changes arithmetic width, not the color model. Roundoff need not match the original f32 inverse exports bit for bit.

## Numeric domain

Version-one field strength is any finite nonnegative f32, not a factor clamped to one.
Field threshold magnitude is at most 0.5; placement is at most one; the field scale is 0.25.
The largest fixed coordinate width is below 204.
Thus field-generated coordinate magnitudes remain below `1e40`, including the source coordinate.
The inverse matrices and cubics then remain below `1e125`, comfortably within f64.
No valid field offset can cause inverse `Infinity-Infinity` or overflow its hue reduction.
Arbitrary near-f64-maximum caller coordinates are outside this internal helper's input contract.

`coordinates_to_srgb` exposes the unclipped encoded channels for numeric proof fixtures.
Only the final encoded channels clip to `[0,1]`. Multiply by 255 and round nearest, with ties upward.
The perturbation composition copies byte alpha separately, including zero-alpha hidden RGB.
