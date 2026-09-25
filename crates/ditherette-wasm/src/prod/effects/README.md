# Production effects

This directory starts as a copy of the frozen [`spec/effects`](../../spec/effects/spec.md) reference.
Semantics, argument domains, and error paths are documented there. `tests/prod_effects.rs` checks exact output against the reference.

## Optimizations

### Per-channel table folding (selected)

A leading run of enabled per-channel effects (`Effect::per_channel`) is tabulated once per chain: 256 `f32` entries per channel.
Every pixel with channel byte `k` starts at `k / 255` and passes through the same scalar maps, so the table is exact.
When the run is the whole chain, the tables are rounded to bytes and the image is rewritten in place with three lookups per pixel. No continuous carrier is allocated.
Otherwise the tables seed the carrier and the remaining effects run normally.

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

### Tabulated linear decode for hue-saturation (selected)

Hue-saturation mixes channels, so it runs on the carrier: per pixel, three sRGB decodes, three cube roots, and three encodes.
When it directly follows byte input, or a tabulated run, each channel has at most 256 carrier values.
`Effect::apply_tabulated` lets it decode through a per-channel linear table while building the carrier, which removes three `powf` per pixel.
It also hoists the turn's sine and cosine out of the pixel loop (about 2% on its own).

| Chain | Fixture | Copy | Selected | Change |
| --- | --- | ---: | ---: | ---: |
| `hue-saturation` | Celeste_Insta_selfie 800×800 | 42.2 ms | 28.6 ms | −32% |
| `grade+hue` | Celeste_Insta_selfie 800×800 | 50.5 ms | 37.4 ms | −26% |
| `hue-saturation` | Picking_at_thread 3462×2309 | 560.0 ms | 385.8 ms | −31% |
| `grade+hue` | Picking_at_thread 3462×2309 | 670.2 ms | 494.1 ms | −26% |

### Colour memo for byte-input carrier effects (selected)

After a tabulated run, a pixel's carrier value depends only on its three bytes, so a carrier effect's output does too.
`memo::try_memoized` keeps a direct-mapped table of 16,384 colours (256 KiB, charged as scratch) and checks the full key on every hit.
Hue-saturation and recolour use it when they read byte input. Photos repeat colours locally; illustrations repeat them everywhere.

| Chain | Fixture | Copy | Selected | Change |
| --- | --- | ---: | ---: | ---: |
| `hue-saturation` | Celeste_Insta_selfie 800×800 | 42.2 ms | 6.1 ms | −86% |
| `grade+hue` | Celeste_Insta_selfie 800×800 | 50.5 ms | 15.6 ms | −69% |
| `hue-saturation` | Picking_at_thread 3462×2309 | 560.0 ms | 124.7 ms | −78% |
| `grade+hue` | Picking_at_thread 3462×2309 | 670.2 ms | 254.2 ms | −62% |
| `recolour-apply` | Celeste_Insta_selfie 800×800 | 86.5 ms | 8.2 ms | −90% |
| `recolour-apply` | Picking_at_thread 3462×2309 | 1061 ms | 188.6 ms | −82% |

`recolour-apply` applies the fixture's own analysed recipe (8-colour palette, Oklab); it includes the tabulated linear decode.

### Recolour analysis (selected)

Each sample's hue, chroma, and neutral ramp are computed once instead of once per sector, keeping the reference's multiplication and summation order.

| Fixture | Copy | Selected | Change |
| --- | ---: | ---: | ---: |
| Celeste_Insta_selfie 800×800 | 26.3 ms | 19.1 ms | −27% |
| Picking_at_thread 3462×2309 | 57.4 ms | 43.8 ms | −24% |

Analysis reads at most 2¹⁸ samples, so its cost flattens for large images. The processor also caches analyses by exactly what they read; a repeated analysis costs one SHA-256 pass over the samples.

### Considered, not taken

- Encoding through a byte-threshold search instead of `powf` when hue-saturation is last. Exact only if `byte(linear_to_srgb_unit(v))` is monotone for every `f32` on every target, which the libm contract does not promise.
- Parallel carrier rows under `threads`. Exact and embarrassingly parallel, but the threaded execution policy has no effect budget yet.

### Public package, scalar Wasm

Node 24 (V8), 3462×2309 synthetic source, `process` to 480×320 area resize, Oklab matching, no dither. Median of five, milliseconds:

| Chain | `applyEffects` cold | warm | `process` cold | warm |
| --- | ---: | ---: | ---: | ---: |
| none (recipe v1) | | | 54.5 | 4.9 |
| `levels` | 43.3 | 16.2 | 61.0 | 16.7 |
| `levels-x3` | 36.5 | 12.9 | 59.0 | 16.5 |

Cold calls use a fresh processor. Warm `applyEffects` returns the retained result after verifying the source.
Warm recipe-v2 `process` reapplies the chain to verify its snapshot, then hits the downstream caches.
That verification costs about 11 ms here. Keeping a raw-source snapshot would turn it into one comparison, at the price of one more retained source-sized buffer.

Memory: `applyEffects` holds the source snapshot and one output buffer, like `perturb`.
Recipe-v2 `process` adds one source-sized comparison buffer to the v1 budget.
Neither allocates the continuous carrier while every enabled step tabulates.
