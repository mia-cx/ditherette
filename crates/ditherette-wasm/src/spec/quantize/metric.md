# quantization metrics

Metrics answer: given a source color coordinate and a palette color coordinate, how far apart are they?

Supported spec metrics:

- Euclidean 3-channel distance for ordinary Cartesian spaces: sRGB, linear RGB, OKLab, CIELAB, YCbCr.
- Circular hue distance for cylindrical spaces: OKLCH and CIELCH. Hue wraps at 2π and the hue term scales by chroma.
- Weighted RGB for gamma-encoded sRGB coordinates:
  - CompuPhase red-mean weighted RGB.
  - Rec.601 fixed RGB channel weights.
  - Rec.709 fixed RGB channel weights.
- CIEDE2000 over CIELAB coordinates.

The weighted RGB metrics are RGB-specific matching heuristics, not independent color spaces.
