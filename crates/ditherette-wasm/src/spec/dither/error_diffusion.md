# Error-diffusion reference

`error_diffusion::diffuse(DitherQuantizeRequest)` validates and executes a complete indexed diffusion call.
It supports all four kernels, both scan orders, both feedback recipes, and all 15 matching policies.
Other dither families receive `UnsupportedOperation` at `dither.family`; the later pipeline dispatcher selects this composition only for diffusion.

## Feedback and scan order

The explicit `feedback` tag selects one of two recipes.

- `srgb-bytes` stores f32 byte-domain RGB. Before matching, round/clamp each current channel to a byte.
  Convert those bytes into the selected matching space. Subtract the selected palette's original RGB bytes from the rounded current bytes.
- `matching` stores the selected space's f32 coordinates. Match the unrounded current coordinates directly.
  Subtract the selected palette coordinates componentwise, including cylindrical hue. Do not clip or reconstruct RGB between pixels.

The former diffusion `space` field could not distinguish these paths when the matcher also used sRGB.
With red palette bytes 0/2 and two source red bytes 1/1, the first pixel ties and emits error 1.
The second receives `7/16`, making red `1.4375`. Byte feedback rounds back to 1 and keeps the first tie.
Matching feedback retains the fractional coordinate and selects red 2. The index sequences are `[0,0]` and `[0,1]`.

Source alpha preparation happens before initializing work. S09 owns threshold, matte, premultiplication, and warning semantics.
Each preserved transparent/fallback pixel has a fixed index. It discards incoming error and emits none, regardless of hidden RGB.
Fixed-index pixels are error sinks, including an opaque darkest fallback. Discarded error never becomes useful work.

Rows progress top to bottom. Raster visits every row left to right.
Serpentine visits odd rows right to left and negates every tap's horizontal offset on those rows.
All taps target later pixels. Discard out-of-image taps without redistributing or renormalizing their weights.

## Tap sets

Each triple is `(dx, dy, numerator)`. Divide weights by the listed denominator.

| Kernel | Denominator | Ordered taps |
|---|---|---|
| Floyd-Steinberg | 16 | `(1,0,7), (-1,1,3), (0,1,5), (1,1,1)` |
| Sierra | 32 | `(1,0,5), (2,0,3), (-2,1,2), (-1,1,4), (0,1,5), (1,1,4), (2,1,2), (-1,2,2), (0,2,3), (1,2,2)` |
| Sierra Lite | 4 | `(1,0,2), (-1,1,1), (0,1,1)` |
| Atkinson | 8 | `(1,0,1), (2,0,1), (-1,1,1), (0,1,1), (1,1,1), (0,2,1)` |

The first three kernels distribute total weight 1. Atkinson distributes 6/8 and deliberately discards the remaining quarter.
These are the inherited kernel definitions; the website's `quantize-algorithms/error-kernels.ts` supplies its three existing families.

## Placement and arithmetic

Placement reads the unchanged source RGBA8 in the selected matching space, even during byte feedback.
It never reads composited/error-modified work or palette-derived ranges.
The [placement reference](placement.md) supplies its eight-neighbor mask, including minimum-chroma hue arcs for cylindrical spaces.
Multiply normalized strength by the source pixel's mask, then multiply each tap weight by that value.
The mask scales outgoing error, not current matching or incoming error.

The reference stores a full-image f32 work plane and a per-pixel alpha-preparation vector.
The work plane starts with source colors; each incoming contribution rounds back to f32 immediately.
Residual subtraction, strength/mask multiplication, and each scalar scatter addition use f64, like JavaScript arithmetic around Float32Array writes.
No f64 image plane, ring buffer, tiled schedule, cache, or search optimization is involved.
Matching scores retain S10's f32 formulas and strict first-entry tie rule.

Finite settings can exceed this representation during feedback growth.
Check work and every matching score before selecting a winner; check every useful scatter update before storing it.
Nonfinite work or scores return `Runtime` at `dither.arithmetic`. No partial indexed result escapes.
The reference imposes no new strength ceiling and never clamps matching coordinates to hide overflow.
For example, large strength can overflow byte work directly, or leave coordinate work finite while its squared distances overflow.
Exact palette colors and error sinks can still succeed at `f32::MAX` strength because they produce no useful growing error.

The website's `quantize-algorithms/error-diffusion-runner.ts` defines the two feedback paths, per-scatter Float32Array rounding, and transparent bypass.
The [approved architecture decision](https://github.com/mia-cx/ditherette/issues/40) permits full-image reference scratch and requires bounded production scratch later.

## Existing coordinate adapters

`dither_error_diffusion_into` and `dither_error_diffusion_by_nearest_into` remain available with their original unrounded coordinate semantics.
They accept precomputed f32 coordinates and caller-provided output storage, without request/alpha/placement handling.
Their low-level caller supplies valid palette, image, and finite-arithmetic inputs; use `diffuse` for the structured public-request reference.
`ErrorDiffusionKernel::taps` remains the common readable tap inventory.
