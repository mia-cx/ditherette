# OKLCH round-trip recipe

`rgb8_to_oklch` composes the [Oklab recipe](oklab.md) with `C=sqrt(a²+b²)` and `h=atan2(b,a)`.
The [Oklab author's polar equations](https://bottosson.github.io/posts/oklab/#the-oklab-color-space) also define the inverse `a=C*cos(h), b=C*sin(h)`.
This recipe stores hue in radians, normalized to `[0,TAU)`, and lightness/chroma in Oklab units.

Exact byte grays, where R=G=B, use their computed L with exactly C=0 and h=0.
This removes unstable hue caused only by f32 matrix residuals. Near-grays retain their computed chroma and hue; there is no epsilon threshold.
Positive and negative zero hue normalize to positive zero. An f32 remainder rounded up to TAU also becomes zero.

`oklch_to_rgb8` treats negative C as zero, ignores hue at nonpositive C, and wraps positive-chroma hue modulo TAU.
Finite hue may exceed one turn or be negative. Reconstruction then follows the Oklab inverse and its final byte clipping/rounding.
It does not choose a shortest interpolation arc. Yliluoma's componentwise coordinate interpolation remains a separate recipe.

The [cartesian enclosing box](oklab.md) implies the conservative byte-input bounds `L∈[-0.01,1.01]`, `C∈[0,sqrt(2.5²+1²)]`, and `h∈[0,TAU)`.
These mathematical enclosures are not tight gamut limits; inverse perturbation coordinates may extend beyond them while remaining finite.

Forward image conversion emits packed triples. The inverse takes a matching RGBA8 alpha source and copies each alpha byte unchanged.
Both adapters honor image row strides. Tests cover every byte gray, near-grays, the hue seam, multiple turns, and alpha-preserving image round trips.
