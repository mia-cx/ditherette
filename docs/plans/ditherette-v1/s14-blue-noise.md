# S14 blue-noise reference

Implements [issue55](https://github.com/mia-cx/ditherette/issues/55) on `impl/v1-s14-blue-noise`.
PR base is `impl/v1-s13-base` at `bfa3d42b79dcc51db9a1f99da3be2c02653b3009`.
Prerequisites are S12 `01df66826e532d8fb3b522a1564f1121c96f4d1f` and S09 `d8bcdcdbe8f874eaee65447a640b97483c9cb775`.
Both are verified ancestors. Their combined placement/palette tests pass 19 fixtures in this joined tree.

## Work

- [x] Record the defect and a reproducible naive generator with fixed numerical acceptance criteria.
- [x] Generate and retain the rank tile, digest, and numerical spectral analysis.
- [ ] Connect the reference field, verify fixed RGBA8 compositions, and file the unmerged PR.

Only blue-noise reference code, its offline generator/asset, tests, and documentation belong to this slice.
S13 separately owns the shared perturbation loop; S17 joins and certifies complete-method composition.
No production optimization or benchmark measurement belongs here.

## Pre-generation criteria

The [preregistered construction plan](https://github.com/mia-cx/ditherette/issues/55#issuecomment-5570367697) predates any generated replacement.
The inherited 64-entry tile is exactly transposed Bayer8. A direct DFT at half occupancy places all non-DC power in one frequency.
It therefore fails the replacement's maximum peak-fraction gate of 0.1.

Use periodic 32x32 Ulichney void-and-cluster ranking, Gaussian sigma 1.5 pixels, 128 initial occupied sites, seed `0xd17ee77e`.
Fisher-Yates visits positions 1023 down to 1. Each draw advances the u32 LCG by `state = 1664525*state + 1013904223` modulo 2^32.
Its swap index is the high 32 bits of `state*(i+1)`. Equal densities choose the first row-major site.
All densities and DFT coefficients are direct sums. The generator neither maintains convolution caches nor uses FFTs.

Ranks must contain every integer 0..1023 once. At occupancies 1/8, 1/4, 1/2, 3/4, and 7/8:

- Mean non-DC power at frequency radius <=3.2 must be <=0.2 times the Bernoulli white-noise expectation `1024*p*(1-p)`.
- Mean power at radius 8..16 must be at least four times the low-band mean.
- A single non-DC frequency contains at most 10% of total non-DC power.
- Four equal angular sectors modulo pi have normalized mean-power coefficient of variation <=0.5.
- A Parseval check verifies the direct DFT's total power against `1024^2*p*(1-p)` within relative error 1e-10.

These construction checks do not claim universal visual quality or approve any non-exact production candidate.

## Construction result

Generator checkpoint `c70bedc368ec9e6023c89931f961ccfde8c0766c` predates the asset.
The original parameters converge after 45 moves and pass every gate on the first generation.
Repeated generation returns the exact retained rank array.
Its little-endian u16 digest is `bcd93746b99ef8ad678ad425f21e1890b4248050b1ea1b382800d7da977e5943`.

Across the five tested occupancies, low-band power is 0.0172..0.0491 of white-noise power.
High/low mean-power ratios are 25.9..65.3. Peak fractions stay below 0.01; angular coefficients stay below 0.092.
The raw per-pattern numbers, parameters, and toolchain are retained in `spec/dither/blue_noise/analysis.json`.
Six focused tests pass, including complete regeneration and negative Bayer/stripe controls; the existing 12 dither fixtures also pass.
The fixed RGBA8 fixture proves quarter-range scaling and byte rounding independently, including hidden RGB with zero alpha.
