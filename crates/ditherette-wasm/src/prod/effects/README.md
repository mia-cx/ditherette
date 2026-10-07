# Production effects

This directory starts as a copy of the frozen [`spec/effects`](../../spec/effects/spec.md) reference.
Semantics, argument domains, and error paths are documented there. `tests/prod_effects.rs` checks exact output against the reference.

## Optimizations

### Per-channel table folding (selected)

A leading run of enabled per-channel effects (`Effect::per_channel`) is tabulated once per chain: 256 `f32` entries per channel.
Every pixel with channel byte `k` starts at `k / 255` and passes through the same scalar maps, so the table is exact.
When the run is the whole chain, the tables are rounded to bytes and the image is rewritten in place with three lookups per pixel. No continuous carrier is allocated.
Otherwise the tables seed the carrier and the remaining effects run normally.

`curves` joins this fold only when every entry is a one-input remap of one encoded or linear RGB channel to itself. Any two-input curve makes the step pointwise. Production prepares all other curve lists once per call and runs them through the colour memo.

Preparation removes neutral grids before any colour conversion. A grid evaluates `x` across its columns, then `x2` across its rows. It borrows grid values by curve index and uses fixed `[f32; 16]` row scratch, so maximum grids do not enlarge every prepared entry or allocate per pixel. One-input hue remaps use the ordinary open spline. One-input hue adjustments use their duplicate-seam cyclic spline. Each hue-valued grid axis uses its own closed sequence without a duplicate seam.

Criterion, `crit_effects`, native x86-64 release, quiet host, 2026-09-25:

| Chain | Fixture | Reference | Production | Speedup |
| --- | --- | ---: | ---: | ---: |
| `levels` | Celeste_Insta_selfie 800×800 | 14.0 ms | 0.50 ms | 28× |
| `levels-x3` | Celeste_Insta_selfie 800×800 | 18.6 ms | 0.51 ms | 37× |
| `levels` | Picking_at_thread 3462×2309 | 177.7 ms | 7.34 ms | 24× |
| `levels-x3` | Picking_at_thread 3462×2309 | 238.0 ms | 7.32 ms | 33× |

Production time includes copying the source. It no longer grows with the number of per-channel steps.

The grading effects from #106 join the same fold. `grade` is exposure, white balance, curves, and brightness-contrast:

| Chain | Fixture | Reference | Production | Speedup |
| --- | --- | ---: | ---: | ---: |
| `grade` | Celeste_Insta_selfie 800×800 | 52.0 ms | 0.55 ms | 95× |
| `grade` | Picking_at_thread 3462×2309 | 683.5 ms | 7.38 ms | 93× |

### Pointwise colour memo (selected)

Every built-in is pointwise once its arguments are known: each output pixel depends only on the same input pixel.
After the tabulated run, a pixel's carrier value is a pure function of its three input bytes, so the rest of the chain is too.
Terminal application now keeps final RGB bytes in a direct-mapped table and writes them into the existing RGBA buffer.
Its size is the next power of two at `pixels / 32`, clamped from 16,384 through 262,144 entries.
Each eight-byte entry checks the full 24-bit key. The large photo therefore uses a 2 MiB table instead of a 99.35 MiB carrier and float memo, reducing peak scratch by 97.35 MiB.
The table is allocated before any pixel changes, so allocation failure leaves the input clean.

Each miss runs the unchanged `f32` chain, including the clamp after every effect, then converts the final channels to bytes once.
Fixed-size prepared state computes hue sine, cosine, and chroma scale once per call.
When hue-saturation leads the remaining chain, three 256-entry tables decode its exact linear inputs once.
The continuous `carrier_after` analysis path keeps its 16,384-entry float memo because recolour analysis needs the unrounded values.
Nothing is boxed or allocated per miss; `tests/prod_effects_allocation.rs` still fails every allocation in turn.
Photos repeat colours locally; illustrations repeat them everywhere. Effects without a pixel map, such as future spatial effects, keep the carrier path.

A step [mask](../../spec/effects/mask.md) keeps the chain pointwise: its strength depends only on the colour entering the step. A masked step never joins the leading tables, since its mask reads all three channels. Inside the memo, each masked step evaluates a fixed-size `PreparedMask` on the colour reaching it, then calls `map_prepared_masked`. Mask curves whose values are all 1 are dropped while preparing, which is exact because they multiply by 1. Resolved chains borrow each caller's mask, so resolution copies no curve data. On the continuous-carrier path, a masked step that is not pointwise reserves a copy of its input and its strengths fallibly, and the memory estimate charges both.

A recipe-less `recolour` step is global, since it analyses the whole image reaching it. `resolve_analyses` first replaces each one (and each `curves`-less `palette-fit` step) with the recipe or curve list it would derive: it builds the carrier up to that step (memoized too), analyses it through the cache, and substitutes the result. The resolved chain is pointwise end to end.

Criterion `crit_effects`, same host and fixtures, 8-colour palette, Oklab:

| Chain | Fixture | Reference | Production | Speedup |
| --- | --- | ---: | ---: | ---: |
| `hue-saturation` | Celeste_Insta_selfie 800×800 | 42.3 ms | 6.7 ms | 6.3× |
| `grade+hue` | Celeste_Insta_selfie 800×800 | 59.5 ms | 7.4 ms | 8.0× |
| `recolour-apply` | Celeste_Insta_selfie 800×800 | 86.9 ms | 9.5 ms | 9.1× |
| `recolour+grade` | Celeste_Insta_selfie 800×800 | 139.0 ms | 29.7 ms | 4.7× |
| `hue-saturation` | Picking_at_thread 3462×2309 | 547.0 ms | 141.1 ms | 3.9× |
| `grade+hue` | Picking_at_thread 3462×2309 | 752.6 ms | 164.8 ms | 4.6× |
| `recolour-apply` | Picking_at_thread 3462×2309 | 1078 ms | 225.7 ms | 4.8× |
| `recolour+grade` | Picking_at_thread 3462×2309 | 1453 ms | 359.3 ms | 4.0× |

`recolour-apply` applies the fixture's own analysed recipe. `recolour+grade` is an automatic recolour step followed by exposure and curves, analysis included and uncached.

The adaptive cache sizing evidence for `Picking_at_thread` is:

| Cache | Hit rate | Misses |
| --- | ---: | ---: |
| Direct 16K | 78.48% | 1,720,149 |
| Direct 64K | 86.68% | 1,064,671 |
| Direct 262K | 95.37% | 369,976 |
| 2-way 262K | 95.98% | 321,028 |

The extra associativity saves only 0.61 percentage points, so the selected table stays direct-mapped.

Criterion `crit_effects`, native x86-64 release, quiet host. Values are medians and IQRs from 20 flat samples:

| Fixture | Baseline median (IQR) | Candidate median (IQR) | Speedup |
| --- | ---: | ---: | ---: |
| Celeste_Insta_selfie 800×800 | 6.652 ms (0.066) | 2.400 ms (0.031) | 2.77× |
| Picking_at_thread 3462×2309 | 140.811 ms (1.477) | 20.023 ms (0.593) | 7.03× |

The baseline samples are the 2026-09-25 direct-16K implementation. The candidate samples are from 2026-09-27.
Both runs compare production bytes with the frozen reference before timing.

`crit_effects_browser.mjs` supplies the corresponding scalar-Chromium recipe-v2 comparison.
It decodes both photos before timing, uses a fresh processor per sample, alternates artifact order, checks indexed-output hashes, and reports medians plus IQRs for no effects, curves, large hue-saturation, and 800×800 hue-saturation.
`DITHERETTE_BENCH_PHOTO` swaps the large fixture for a local photo. Scalar Chromium, 2026-09-28, with a 6000×4000 camera JPEG (not committed) as the large fixture; indexed output matched in every case:

| Case | Baseline median (IQR) | Candidate median (IQR) | Speedup |
| --- | ---: | ---: | ---: |
| 6000×4000 no effects | 136.6 ms (2.7) | 136.7 ms (4.0) | 1.00× |
| 6000×4000 curves | 167.3 ms (4.3) | 166.3 ms (3.8) | 1.01× |
| 6000×4000 hue-saturation | 2100.6 ms (29.5) | 393.4 ms (9.0) | 5.34× |
| Celeste_Insta_selfie 800×800 hue-saturation | 32.4 ms (1.2) | 19.6 ms (0.7) | 1.65× |

### Recolour analysis (selected)

Each sample's hue, chroma, and neutral ramp are computed once instead of once per sector, keeping the reference's multiplication and summation order.
Resolution builds the analysed carrier through the tables and memo as well.

| Fixture | Reference | Production | Change |
| --- | ---: | ---: | ---: |
| Celeste_Insta_selfie 800×800 | 25.7 ms | 17.8 ms | −31% |
| Picking_at_thread 3462×2309 | 55.3 ms | 44.8 ms | −19% |

Analysis reads at most 2¹⁸ samples, so its cost flattens for large images. The processor also caches analyses by exactly what they read; a repeat costs one SHA-256 pass over the samples.

### Considered, not taken

- Encoding through a byte-threshold search instead of `powf` when hue-saturation is last. Exact only if `byte(linear_to_srgb_unit(v))` is monotone for every `f32` on every target, which the libm contract does not promise.
- Parallel carrier rows under `threads`. Exact and embarrassingly parallel, but the threaded execution policy has no effect budget yet.

### Public package, scalar Wasm

Node 24 (V8), 3462×2309 synthetic source, `process` to 480×320 area resize, Oklab matching, no dither. Median of five, milliseconds:

| Chain | `applyEffects` cold | warm | `process` cold | warm |
| --- | ---: | ---: | ---: | ---: |
| none (recipe v1) | | | 53.0 | 5.1 |
| `levels` | 41.6 | 10.5 | 73.7 | 5.0 |
| `levels-x3` | 37.7 | 11.0 | 76.4 | 4.9 |
| automatic `recolour` | 475.4 | 19.4 | 515.0 | 4.9 |

Cold calls use a fresh processor. Warm `applyEffects` returns the retained result after verifying the source.
Warm recipe-v2 `process` keeps the raw source from its last successful call. When the source and the enabled chain repeat, it skips the effects and hands `process` the same effected snapshot, so every downstream cache hits.
Any other call drops that raw snapshot at its start, so it is charged only by the recipe-v2 call that owns it.

Memory: `applyEffects` holds the source snapshot and one output buffer, like `perturb`.
Recipe-v2 `process` adds the retained raw snapshot to the v1 budget.
A terminal pointwise chain adds its adaptive byte memo. Analysis adds the 13-byte-per-pixel carrier and 256 KiB float memo; automatic recolouring also adds its bounded analysis samples.
The analysis cache has a fixed bound in the processor bookkeeping.
