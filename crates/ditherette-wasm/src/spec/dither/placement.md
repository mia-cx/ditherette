# Palette-independent placement reference

## Mask recipe

`placement_mask_at` takes source RGBA8, pixel coordinates, `WorkingSpace`, and the validated `Placement` request.
There is no palette parameter or prepared-palette import. Palette selection cannot influence this mask.
`Everywhere` returns 1. Adaptive placement reads source RGB, including hidden RGB, and ignores alpha.

For radius r, sample offsets are `(-r,0),(r,0),(0,-r),(0,r),(-r,-r),(r,-r),(-r,r),(r,r)` in that order.
Clamp each sample coordinate independently to the nearest image edge. Count duplicate samples and always divide by eight.
Radius is the validated integer pixel distance, from 1 through 32768. It is not a box-filter radius.
Typed image views exclude row padding. The caller supplies an in-bounds center pixel.

Convert the center and each neighbor through the selected reference forward conversion.
Ordinary spaces use Euclidean distance. OKLCH and CIELCH use:

```text
angle = abs(h1 - h2)
wrapped = min(angle, 2*pi - angle)
dh = min(C1, C2) * wrapped
distance = sqrt((L1-L2)^2 + (C1-C2)^2 + dh^2)
```

Thus either neutral chroma suppresses the hue term. Forward conversions supply normalized hue and nonnegative chroma.
This minimum-chroma arc distance is the website's placement metric, not the circular chord metric used for palette matching.
It extends the existing OKLCH placement rule to CIELCH without mixing either space's units.

Let `ranges` be the three fixed coordinate widths documented below.
Contrast is `sum(distance)/8/sqrt(ranges[0]^2+ranges[1]^2+ranges[2]^2)*100`.
Threshold and softness use those normalized contrast percentage points.
Set `lower=threshold-softness` and `upper=threshold+softness`.
When lower equals upper, return 1 if contrast is at least upper, otherwise 0.
Otherwise set `t=clamp((contrast-lower)/(upper-lower),0,1)` and return `t*t*(3-2*t)`.
Contrast is not clipped separately. The smoothstep alone bounds the mask to `[0,1]`.

Color coordinates and fixed boxes are f32. Distances, accumulation, and smoothstep use f64; the final mask rounds to f32.
This preserves the website's scalar arithmetic and avoids overflow when valid finite f32 controls add beyond f32's range.

The source is [quantize-shared.ts at the validated join](https://github.com/mia-cx/ditherette/blob/bdf79a606c94e8b44ee2c6fa11ea97af490b4b2e/src/lib/processing/quantize-shared.ts).
`placementMask` defines sampling, edge handling, percentage scaling, and smoothstep.
`vectorDistanceValues` defines the minimum-chroma arc metric.
The [approved resolution](https://github.com/mia-cx/ditherette/issues/40) replaces palette-derived ranges with fixed space domains.
Weighted matching uses sRGB working coordinates for placement; wrappers select `WorkingSpace::Srgb` without adding a weighted space.
The [S03 inventory](../contract/inventory.md) records the separate working-space feedback and matching-space roles.

## Fixed domains

`coordinate_domain` defines fixed version-one coordinate boxes. It never reads a source image or palette.
Widths are `maximum-minimum` in the coordinates' own units, not measured palette extrema.
These boxes enclose the SDR byte-input gamut; they do not clip source coordinates or inverse perturbations.

| Space | Minimum | Maximum | Units |
| --- | --- | --- | --- |
| sRGB | `[0,0,0]` | `[1,1,1]` | Encoded channels |
| Linear sRGB | `[0,0,0]` | `[1,1,1]` | Linear-light channels |
| Full-range BT.601 YCbCr | `[0,-0.000001,-0.000001]` | `[1,1.000001,1.000001]` | Normalized luma and offset chroma |
| Oklab | `[0,-0.55,-0.37]` | `[1.01,0.60,0.21]` | L,a,b |
| OKLCH | `[0,0,0]` | `[1.01,0.71,TAU]` | L,C,h radians |
| D65 CIELAB | `[0,-87,-108]` | `[101,99,95]` | L*,a*,b* |
| D65 CIELCH | `[0,0,0]` | `[101,147,TAU]` | L*,C*,h radians |

## Ordinary-space derivation

Encoded RGB bytes divide by 255. The monotone sRGB transfer function fixes the same `[0,1]` endpoints in linear light.
For YCbCr, Y is a convex combination of encoded RGB.
`B-Y=0.886B-0.299R-0.587G` lies in `[-0.886,0.886]`.
`R-Y=0.701R-0.587G-0.114B` lies in `[-0.701,0.701]`.
Dividing those differences by 1.772/1.402 and adding 0.5 gives `[0,1]` chroma domains.
Each chroma bound includes 0.000001 outward allowance for the existing f32 operations.
For example, cyan produces Cr near -0.000000060. The conversion stays unchanged; the fixed box encloses that residual.
These equations are the completed [sRGB](../color/srgb.md), [linear](../color/linear.md), and [YCbCr](../color/ycbcr.md) references.

## Oklab derivation

Use the inherited [forward matrix](../color/oklab.md). Its positive cone weights give `l,m,s` approximately within `[0,1]`.
For two such linear forms `p` and `q`, their ratio lies between the minimum and maximum coefficient ratios.
If `q >= k*p`, then `cbrt(p)-cbrt(q) <= (1-cbrt(k))*cbrt(p)`.
Swap p/q for the lower bound. Their maximum cube roots differ from 1 by less than 0.000001 due to f32 matrix rounding.

The resulting root-difference bounds, rounded here for readability, are:

| Root difference | Lower | Upper |
| --- | --- | --- |
| l′-m′ | -0.217557 | 0.198933 |
| s′-m′ | -0.254777 | 0.445517 |
| l′-s′ | -0.566149 | 0.401662 |
| m′-s′ | -0.445517 | 0.254777 |

The opponent equations can be regrouped as `a≈1.9779985(l′-m′)+0.4505937(s′-m′)` and `b≈0.025904037(l′-s′)+0.78277177(m′-s′)`.
Their tiny f32 coefficient-sum residuals fit inside the outward rounding below.
Substitution bounds a near `[-0.54513,0.59424]` and b near `[-0.36341,0.20984]`.
Round outward to hundredths for the fixed a/b box.
L is nonnegative because the positive m′ term exceeds the negative s′ term even at their minimum coefficient ratio.
Dropping the negative term bounds L below 1.005; 1.01 encloses f32 rounding.

## CIELAB derivation

Use the inherited [D65 CIELAB matrix and white](../color/cielab.md). Let x,y,z denote white-normalized XYZ.
The Lab transfer derivative is positive: constant below epsilon and proportional to `t^(-2/3)` above it.
For `a*=500(f(x)-f(y))`, coefficient ratios x/y range from approximately 0.526 through 2.631.
Bounding `f′(x)/f′(y)` by those ratios proves a* increases with R and B and decreases with G.
Thus green is its minimum and magenta its maximum.
For `b*=200(f(y)-f(z))`, the same argument proves increase with R/G and decrease with B.
Thus blue is its minimum and yellow its maximum. The linear transfer branch includes derivative ratio 1 and preserves these signs.

Independent f64 evaluation gives a* in approximately `[-86.18272,98.23432]` and b* in `[-107.86017,94.47798]`.
Round outward to integer CIELAB units to enclose f32 arithmetic.
L* is monotone with Y and has black/white endpoints approximately 0/100.000004; use `[0,101]`.

## Cylindrical derivation

For `C=sqrt(a²+b²)`, the cartesian boxes imply `C<=sqrt(0.60²+0.37²)<0.71` in OKLCH.
They imply `C*<=sqrt(99²+108²)<147` in CIELCH.
The color references normalize hue into `[0,TAU)` and exact RGB byte grays into C=0,h=0.
The enclosing box uses TAU as the upper hue edge. This retains the existing formula's raw coordinate-width normalization.

These derivations sharpen S08's general conservative boxes without changing any color conversion.
The selected outward rounding is part of recipe version one. Later optimizations cannot replace it with fitted palette or image statistics.
