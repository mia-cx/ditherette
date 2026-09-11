# Cache-aware nearest gather cutoff

Nearest outputs at 50% width and height previously used full-source copying or comparison.
The measured cutoff now allows one output pixel per four source pixels for standalone resize and fused direct/diffusion processing.
Separable and Yliluoma processing keep the existing one-per-16 cutoff.
That preserves reusable perturb stages when palette settings change.

Rust still selects every pixel using the existing nearest plan.
Identity, total upscales, and other filters stay outside this change. Existing mixed-axis eligibility remains.
Frozen specifications and filter coordinates are unchanged.

## Selection

Accepted package `29cfdb8cc6c4e6ed5a17647bd299591a346a6bf1` uses the previous cutoff.
Candidate `520f2008d852eee6e06d8314393272f1cb8b6602` widens it globally.
Selected candidate `b72238d414395c4d1df06d70bbfeb78b7d9c415b` preserves larger separable/Yliluoma cache paths.

The first comparison runs six 50% settings in all three browsers, before and after the global change.
Settings include resize alone, direct sRGB, Oklch, Bayer8, Floyd-Steinberg, and high-entropy sRGB.
All 18 PNGs remain byte-exact.
The global candidate improves resize, but Bayer palette edits regress from 65.55–76 ms to 148–165.5 ms.
It is rejected as a universal cutoff.

The selected policy repeats standalone resize, direct processing, and Bayer8 in every browser.
All nine PNGs match the accepted package exactly. Bayer palette edits recover to 65.6–77 ms.
Every trial uses six cold and warm samples, alternating JS/Wasm order.
Cold uses a fresh worker and processor per sample, excluding import/initialization; compilation caches may persist.
Warm recomputes the requested size after priming a different size. Palette-edit timings prime a rotated palette at the same dimensions.
The image is Celeste at 2600 by 4168; output is 1300 by 2084.
JS remains unchanged at `a895267baea624a6e89bfcef6c5147f170e8a8f7`.

| Browser | Operation | Previous warm Wasm ms | Selected warm Wasm ms | Selected-run JS ms |
|---|---|---:|---:|---:|
| Chromium | Resize | 15.75 | 9.05 | 11.45 |
| Chromium | Direct process | 37.85 | 32.20 | 70.05 |
| Firefox | Resize | 20.00 | 9.50 | 11.00 |
| Firefox | Direct process | 44.00 | 34.00 | 86.50 |
| WebKit | Resize | 15.00 | 7.50 | 6.00 |
| WebKit | Direct process | 41.50 | 35.00 | 57.50 |

The 20% JS target remains unmet in several cells. WebKit standalone resize still loses.
Selected-run cold direct-process times are 37/39.5/51.5 ms versus JS 62.7/84/61.5 ms.
Six samples do not establish a confidence-bound release gate; earlier and later medians vary.
Raw evidence retains all cold, warm, palette-edit, identical-repeat, and end-to-end dispatch-to-canvas-submission samples.

The candidate passes 35 focused native tests and 20 built Wasm/factory tests.
Tests cover cutoff boundaries, all anchors, offset views, fused/staged equality, and cache-path selection.
The scalar build passes. No native nearest kernel changes or native speedup claims are made.

The [evidence archive](nearest-quarter-evidence.tar.xz) retains all three browser trials, image equality records, scripts, and package provenance.
Its SHA-256 is `8c1640edcec38a73d5aa0b4eee3926188a8869e51154d0ab3cda1560ed865a81`.
Compiled packages and PNGs remain outside compiler targets.
This completes the cutoff slice, not the broader JS performance target.
