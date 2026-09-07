# quantization metrics

Metrics answer: given a source color coordinate and a palette color coordinate, how far apart are they?

Supported spec metrics:

- Euclidean 3-channel distance in all seven spaces, including direct L/C/h coordinate comparison for cylindrical spaces.
- Circular hue chord distance for OKLCH and CIELCH, retaining the inherited Rust formula.
- Minimum-chroma hue arc distance for OKLCH and CIELCH. The website's OKLCH mode uses this formula.
- Weighted RGB for gamma-encoded sRGB coordinates:
  - CompuPhase red-mean weighted RGB.
  - Rec.601 fixed RGB channel weights.
  - Rec.709 fixed RGB channel weights.
- CIEDE2000 over CIELAB coordinates.

The weighted RGB metrics are RGB-specific matching heuristics, not independent color spaces.

## Exact score conventions

`distance_score(a, b, policy)` dispatches every valid tagged pair.
All coordinates and metric arithmetic use f32. Euclidean, hue, and weighted RGB scores are squared distances.
CIEDE2000 returns delta E with unit lightness, chroma, and hue weights. Compare scores within one policy only.

For Euclidean distance, square and sum the three coordinate differences in component order.
Direct LCH Euclidean does not wrap hue. Its existing tag remains distinct from both hue-aware metrics.

For both hue-aware metrics, let `dL = L1-L2`, `dC = C1-C2`, and `dh` be the shortest wrapped angular difference.
Hue is in radians and chroma is nonnegative.

- Circular chord uses `dL² + dC² + (2 * sqrt(C1*C2) * sin(dh/2))²`.
- Website hue arc uses `dL² + dC² + (min(C1,C2) * abs(dh))²`.

Zero chroma removes the hue contribution. Unequal chroma can make these formulas select different palette entries.
The separate `*-hue-arc` tags preserve website behavior without replacing the existing `*-circular-hue` recipes.

Rec.601 uses squared channel-delta weights `[0.299, 0.587, 0.114]`.
Rec.709 uses `[0.2126, 0.7152, 0.0722]`.
These weights apply to gamma-encoded RGB, as in `src/lib/processing/color.ts`; they are not a squared luma difference.

CompuPhase also uses normalized gamma-encoded RGB deltas, but its red mean is `255 * (R1+R2)/2`.
The red and blue coefficients are `2 + redMean/256` and `2 + (255-redMean)/256`; green uses 4.
The common omitted factor `255/256` changes winners, not just score units.
From black, red `[17,0,0]` scores `587.595703125` and blue `[0,0,14]` scores `587.234375` in byte-squared units.
Blue wins. Divide both scores by `255²` for this reference's normalized coordinates.

## Formula evidence

The [CompuPhase primary formula](https://www.compuphase.com/cmetric.htm) and the website's `weighted-rgb` branch agree on those coefficients.
The reference uses the article's real-valued formula, not its later integer-shift approximation.

The existing CIEDE2000 implementation follows
[Sharma, Wu, and Dalal's implementation notes](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/ciede2000noteCRNA.pdf).
`tests/spec_quantize_dispatch.rs` checks 16 of their
[published supplementary pairs](https://hajim.rochester.edu/ece/sites/gsharma/ciede2000/dataNprograms/ciede2000testdata.txt),
including signed blue differences, neutral chroma, both sides of the 180-degree discontinuity, and symmetry.
The published four-decimal distances use absolute tolerance `0.0001`; output indices and metadata remain exact.
