# Sparse nearest browser processing

Small nearest outputs copied or compared the entire source image before resizing.
The Celeste source contains 43,347,200 RGBA bytes. A 5% output contains only 108,160 bytes.

This change gathers only pixels selected by the existing Rust nearest plan.
Rust supplies separate column and row byte offsets. The JS boundary copies their selected RGBA bytes.
The cutoff remains one output pixel per 16 source pixels or fewer.
No frozen specification or existing filter coordinates change.

Standalone resize without progress gathers directly into its independent JS result.
Progress-enabled resize keeps the Wasm output buffer and existing callback ordering.
Fused processing gathers into Wasm for the remaining Rust stages.
Every call reads current source pixels; sparse results do not enter the full-source image cache.
Memory accounting includes final output bytes even when JS owns that allocation.

## Browser comparison

Measured source revision `5a08f2f073d82cb0d0f5a5dc0433546f9b780785`.
Historical JS revision `a895267baea624a6e89bfcef6c5147f170e8a8f7` remains unchanged.
Scalar, size-optimized Wasm; no threads or SIMD changes.
Chromium 147, optimizing Firefox 148, and WebKit 26.4 run serially.

Each cell has six samples with alternating backend order.
Cold means a fresh worker and processor for every sample, excluding module import and initialization.
Browser compilation caches can persist. Warm means recomputing the requested size after priming another size.
These are public-call medians, not image decode or canvas presentation timings.
Raw results also retain dispatch-to-canvas-submission times.

| Browser | Scale | Cold Wasm / JS ms | Warm Wasm / JS ms |
|---|---:|---:|---:|
| Chromium | 5% | 6.55 / 10.70 | 1.35 / 7.85 |
| Chromium | 10% | 9.25 / 14.40 | 3.40 / 11.30 |
| Chromium | 25% | 18.50 / 28.35 | 11.35 / 27.80 |
| Firefox | 5% | 3.00 / 9.00 | 1.00 / 6.00 |
| Firefox | 10% | 6.00 / 14.00 | 3.00 / 12.00 |
| Firefox | 25% | 16.50 / 34.00 | 12.00 / 32.00 |
| WebKit | 5% | 9.00 / 11.50 | 2.00 / 7.50 |
| WebKit | 10% | 11.50 / 17.00 | 4.50 / 10.00 |
| WebKit | 25% | 23.00 / 33.00 | 13.50 / 26.00 |

These rows include nearest resize and sRGB palette quantization without dithering.
All nine meet the 20% lower-time target for both cold and warm calls in this trial.
Six samples do not establish a confidence-bound release gate.

Standalone resize still misses the overall JS target.
At 25%, its Chromium cold/warm times are 4.75/2.85 ms versus JS 4.25/2.80 ms.
Before direct output, those Wasm times were 6.40/4.30 ms.
Firefox and WebKit sub-millisecond samples are timer-limited, not zero-cost wins.

## Correctness and retained evidence

All 18 candidate browser PNGs match the compact-gather predecessor byte for byte.
The earlier fused candidate preserves all 96 Chromium outputs against the pre-sparse baseline.
Native tests compare every anchor, fractional and exact scales, mixed axes, and one-pixel dimensions against frozen nearest.
Boundary tests cover mutations, offset views, budgets, failed copies, callback failures, disposal, and durable output ownership.

The [evidence archive](sparse-nearest-evidence.tar.xz) retains raw timings, manifests, scripts, image equality records, and build/test logs.
Its SHA-256 is `fa21d97069817caa61139f9a295c4fa4e3cb873852f4128dc13058c0ab3870f1`.
Package and script hashes identify each measured artifact.
PNG files and compiled package snapshots remain outside compiler targets in the benchmark worktree.
The full 96-case Chromium comparison still has 46 warm misses, 59 cold misses, and one identity no-op.
This PR completes sparse copying only; it does not complete the broader JS performance goal.
