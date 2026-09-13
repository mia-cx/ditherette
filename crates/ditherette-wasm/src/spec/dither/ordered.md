# Bayer reference

`bayer_noise_at(x,y,size)` takes global image coordinates and one of the matrix widths 2,4,8,16.
The recursive Bayer ranks follow the website's [matrix orientation](../../../../../src/lib/processing/bayer.ts): width two is `[[0,2],[3,1]]`.
For width n and rank r, the centered threshold is `(r+0.5)/(n*n)-0.5`.
Coordinates repeat modulo n. Every rank appears once; each tile has mean threshold zero.
The f32 arithmetic is exact for these power-of-two denominators.

The older `dither_bayer_*` adapters accept precomputed f32 working coordinates and palette coordinates, then return indices.
Their direct coordinate offset remains unchanged. The palette-free composition has its own explicit RGBA8 reconstruction boundary.
