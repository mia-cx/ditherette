# D65 CIELAB round-trip recipe

`rgb8_to_cielab` decodes non-HDR sRGB bytes, applies the inherited linear-sRGB-to-XYZ matrix, then computes CIELAB.
The reference white is exactly the inherited f32 triple `[0.95047,1,1.08883]`.
The [W3C conversion equations](https://www.w3.org/TR/css-color-4/#color-conversion-code) specify the piecewise Lab transform and its inverse.
This recipe substitutes its D65 white for the sample's D50 white and performs no chromatic adaptation.

Forward `f(t)` uses the cube root above `216/24389` and `(24389/27*t+16)/116` otherwise.
Inverse `f⁻¹(t)` uses `t³` above `6/29` and `(116*t-16)/(24389/27)` otherwise.
The linear branch includes equality. These rational thresholds avoid an independently rounded discontinuity.

The inverse XYZ-to-linear-sRGB matrix is the mathematical inverse of the existing decimal forward matrix, rounded to f32:

```text
 3.240455   -1.5371388  -0.49853155
-0.9692664   1.8760109   0.041556083
 0.05564342 -0.20402585  1.0572251
```

`cielab_to_rgb8` preserves out-of-gamut coordinates until final encoded sRGB clipping to `[0,1]`.
It then multiplies by 255 and rounds nearest with half ties upward. Inputs and intermediate arithmetic must remain finite.
There is no perceptual gamut mapping or intermediate Lab/XYZ clipping.

L* has nominal black/white coordinates 0 and 100. The a*/b* opponent axes use CIELAB units and have no universal gamut-independent bounds.
For byte sRGB, normalized XYZ lies near `[0,1]`. Applying the monotone `f` interval gives conservative bounds `L*∈[0,101]`, `a*∈[-500,500]`, and `b*∈[-200,200]`.
These outward bounds include the inherited matrix's f32 white residual; they are not tight gamut extrema.
The cartesian recipe retains tiny neutral residuals. [cielch.md](cielch.md) defines cylindrical neutral normalization.

Image adapters use packed f32 triples, honor independent row strides, and copy byte alpha from the corresponding RGBA8 source during reconstruction.
Conversion does not composite or premultiply. Tests include independently evaluated D65 primary vectors and the inverse threshold at L*=8.
