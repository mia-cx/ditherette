# D65 CIELCH round-trip recipe

`rgb8_to_cielch` composes [D65 CIELAB](cielab.md) with `C=sqrt(a²+b²)` and `h=atan2(b,a)`.
The [W3C Lab/LCH equations](https://www.w3.org/TR/css-color-4/#color-conversion-code) define the polar transform and its inverse.
This recipe retains D65 and stores hue in radians, normalized to `[0,TAU)`, rather than the sample's degrees and D50 white.
L* and C* retain CIELAB units.

Exact byte grays, where R=G=B, use their computed L* with exactly C*=0 and h=0.
Near-grays retain their computed chroma and hue; there is no epsilon threshold or NaN hue representation.
Positive and negative zero hue normalize to positive zero. An f32 remainder rounded up to TAU also becomes zero.

`cielch_to_rgb8` treats negative C* as zero and ignores hue at nonpositive C*.
It wraps positive-chroma hue modulo TAU, computes `a*=C*cos(h), b*=C*sin(h)`, then applies the CIELAB inverse and final byte clipping/rounding.
No shortest-arc interpolation or perceptual gamut mapping occurs.

The [cartesian enclosing box](cielab.md) implies conservative byte-input bounds `L*∈[0,101]`, `C*∈[0,sqrt(500²+200²)]`, and `h∈[0,TAU)`.
The inverse also handles finite out-of-gamut coordinates with finite intermediates.

Image adapters emit packed triples and reconstruct RGBA8 while copying corresponding byte alpha unchanged.
They honor independent source, alpha, and output row strides. Hidden RGB does not disappear when alpha is zero.
