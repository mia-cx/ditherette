# Dither modes reference

## Purpose

Add the error-diffusion kernels and rectangular ordered tiles that the v1 reference lacks, without changing a v1 byte.

## Contents

| File                         | Role                                                                          |
| ---------------------------- | ----------------------------------------------------------------------------- |
| [diffusion.rs](diffusion.md) | The full kernel vocabulary, its tap tables, and `diffuse`                     |
| [ordered.rs](ordered.md)     | Rectangular rank tiles, `tile_noise_at`, `perturb`, and `dither_and_quantize` |

## Composition

Each operation is its v1 counterpart with one widened input.
`diffusion::diffuse` is v1 `error_diffusion::diffuse` with a `Kernel` that also names eight later kernels.
For the four v1 kernels it returns exactly the v1 result.
`ordered::dither_and_quantize` is a v1 separable field whose threshold comes from a rectangular tile.
It uses the shared v1 `perturb_by_field_rows_into` and v1 `quantize` unchanged.

Validation reuses the v1 request validator. The validator never reads the kernel or field tag,
so a v1 stand-in tag checks every other control with the v1 paths and messages.

## Aliases

Two names in other tools are the same kernels as v1 modes. They get no new tag.

| Other name                          | Same taps as  |
| ----------------------------------- | ------------- |
| Sierra3, Sierra (three-row)         | `sierra`      |
| Sierra24A, Sierra-2-4A, Filter Lite | `sierra-lite` |

## Not covered

- Dizzy, from [ditherit](https://github.com/alexharris/ditherit/blob/8a8e053fa149160dad32526bfab89a38bba6e6d4/app/utils/dithering.ts).
  It diffuses error in a shuffled pixel order, not a raster scan, so it is a separate family with whole-image state.
- The "25% pattern" seen in a GIF. No source defines its tile, so it has no reference yet.
