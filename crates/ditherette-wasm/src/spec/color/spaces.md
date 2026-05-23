# color-space image formats

Color-space spec modules use packed AoS f32 formats in `image::formats`:

- `Srgb32`: gamma-encoded sRGB, normalized 0..=1.
- `LinearRgb32`: linear-light sRGB, normalized 0..=1 for in-gamut SDR input.
- `Oklab32`: OKLab L, a, b.
- `Oklch32`: OKLab converted to cylindrical L, chroma, hue radians.
- `Cielab32`: CIELAB L*, a*, b* using D65 white.
- `Cielch32`: CIELAB converted to cylindrical L*, chroma, hue radians.
- `YCbCr32`: full-range BT.601 Y, Cb, Cr over gamma-encoded sRGB, neutral chroma at 0.5.

CIEDE2000 is a distance metric over CIELAB coordinates, so `lab_ciede2000` reuses `Cielab32` conversion and adds the distance oracle.
