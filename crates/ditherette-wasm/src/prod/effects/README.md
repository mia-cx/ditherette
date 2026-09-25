# Production effects

This directory starts as a copy of the frozen [`spec/effects`](../../spec/effects/spec.md) reference.
Semantics, argument domains, and error paths are documented there. `tests/prod_effects.rs` checks exact output against the reference.

## Optimizations

### Per-channel table folding (selected)

A leading run of enabled per-channel effects (`Effect::channel_map`) is tabulated once per chain: 256 `f32` entries per channel.
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
