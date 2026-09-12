# dither spec

Dithering is separate from color-space conversion and quantization metrics.

The dither specs operate in a caller-selected working color space: the source image and palette are already converted into matching f32 coordinates. The algorithms emit palette indices. Error diffusion propagates quantization error in the dither working space; ordered/noise algorithms perturb the working-space color before palette lookup.

Current spec algorithms:

- Bayer ordered thresholding: 2x2, 4x4, 8x8, 16x16.
- Floyd-Steinberg error diffusion.
- Sierra error diffusion.
- Sierra Lite error diffusion.
- Atkinson error diffusion.
- Yliluoma-style ordered two-color palette mixing.
- Bundled-tile blue-noise thresholding.
- Seeded random-noise thresholding with Mulberry32.
