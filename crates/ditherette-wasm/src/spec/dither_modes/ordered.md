# Rectangular ordered tiles

`tile_noise_at(x,y,tile)` takes global image coordinates. The tile origin is the image's top-left pixel.
Coordinates repeat modulo the tile width and height. Ranks are row-major, top row first.
For n cells and rank r, the centred threshold is f32 `(r+0.5)/n-0.5`, the [v1 Bayer](../dither/ordered.md) formula.
Every rank appears once, so each tile's mean threshold is zero.

`perturb` and `dither_and_quantize` are the v1 separable composition with this threshold as the field.
They use [v1 perturbation](../dither/perturb.md), including the 0.25 field scale and the RGBA8 boundary, then v1 `quantize`.

## Tiles

| Tag   | Ranks (rows separated by `/`)            | Source                                           |
| ----- | ---------------------------------------- | ------------------------------------------------ |
| `3x1` | `0 2 1`                                  | Construction below                               |
| `4x1` | `0 2 1 3`                                | [Yliluoma][jy] rectangular generator, `M=2, L=0` |
| `4x2` | `0 4 2 6 / 3 7 1 5`                      | [Yliluoma][jy] printed `X=4, Y=2` table          |
| `5x3` | `0 12 7 3 9 / 14 8 1 5 11 / 6 4 10 13 2` | [Yliluoma][jy] hand-designed `X=5, Y=3` table    |

Joel Yliluoma's [threshold-matrix appendix][jy] gives a bit-interleaving generator for any tile whose sides are powers of two.
Running its published C code with `M=2, L=0` prints `0 2 1 3`; with `M=2, L=1` it prints the `4x2` table above.
The `5x3` tile is printed on the same page as a hand-designed example.

No source prints a `3x1` tile. Yliluoma orders ranks so that close ranks sit far apart in pseudo-toroidal distance.
On a three-cell ring every pair of cells is one step apart, so that rule accepts every order.
All six orders are the same pattern up to a shift or a mirror. The tile takes the first three ranks of the `4x1` order, `0 2 1`.
[dither-me-this][dmt] produces the same `3x1` ranks.

These tiles are distinct from the square v1 Bayer sizes. Yliluoma's square tables use a different orientation from v1 Bayer, so none of them replaces a v1 size.

[jy]: https://bisqwit.iki.fi/story/howto/dither/jy/#Appendix%202ThresholdMatrix
[dmt]: https://github.com/ShadowfaxRodeo/dither-me-this/blob/5812b6fd01f6118152f121a17cf06eb7f80a7e9c/src/functions/bayer-matrix.js
