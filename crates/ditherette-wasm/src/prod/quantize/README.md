# Quantization performance

Scalar Wasm, 2026-09-28, release profile `opt-level = "s"`: `process` on a 1500×1000
image of smooth ramps plus small per-pixel noise (photo-like colour counts), a
64-colour palette, identity resize, no dithering, and a fresh processor per run.

| Match policy       |  Before |  After |
| ------------------ | ------: | -----: |
| `oklab-euclidean`  |   93 ms | 100 ms |
| `cielab-euclidean` |   90 ms |  92 ms |
| `cielab-ciede2000` | 3266 ms | 690 ms |

The CIEDE2000 matcher retains the bounded exact-RGB memo used by other metrics.
For each cache miss it precomputes source and palette unprimed chroma, seeds the
scan with the closest Euclidean Lab entry, and applies successively tighter exact
lower bounds before evaluating the full hue weight. The pair bound uses the primed
Lab chord and hue direction to retain safe chroma/hue information without trigonometry.
Palette entries keep their original positions, so equal scores still select the first
entry. The after column is the median of three consecutive runs; the Euclidean rows are unchanged within run-to-run noise.
`opt-level = 3` brought CIEDE2000 to 640 ms but grew the scalar Wasm from 674 KB to 829 KB, so the profile stays `"s"`.
