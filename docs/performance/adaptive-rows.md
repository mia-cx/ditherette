# Adaptive placement row reuse

Adaptive placement repeatedly converts the same source pixels when measuring eight neighbors.
Three caller-owned coordinate rows now reuse those conversions.
Radius-one forward scans convert each source row once. Other radii preserve exact clamping and neighbor order.
Field reconstruction also reuses the center coordinate.

Public scalar perturb, diffusion, and process calls account for optional row storage in their memory budgets.
Native complete calls use the same helper. Low budgets and worker-band paths keep direct conversion.
RNG draws, callbacks, alpha rules, and frozen mathematical output stay unchanged.
The pinned threaded toolchain uses the equivalent `map_or` expression instead of the newer `is_none_or` API.

## Native selection

Accepted revision `95aad503e5e61a1d046f2b9d0b971c3632468adb`.
Candidate revision `0ebcae59c1404ce027633c4e5b2917055b77ddc7`.
The later compatibility edit changes no calculation.

`ditherette-bench` completes 140 serial workers and 11,200 samples across 35 cases.
All cases pass the existing gate and match frozen output byte for byte.
Each case uses two alternating pairs, 80 samples per worker, 250 ms warmup, and a 1,000 ms measurement budget.

| Case group | Candidate / accepted time |
|---|---:|
| 24 retained non-adaptive diffusion controls | 0.978–1.035 |
| Floyd-Steinberg adaptive radius 1, both feedback modes | 0.441–0.444 |
| Sierra adaptive radius 2, both feedback modes | 0.582 |
| Sierra-lite adaptive radius 1, both feedback modes | 0.429–0.439 |
| Atkinson adaptive radius 2, both feedback modes | 0.558–0.567 |
| Bayer8/Oklab adaptive field | 0.299 |
| Random/Oklch adaptive field | 0.203 |
| Blue-noise/Cielch non-adaptive field control | 1.006 |

These are native complete-call comparisons, not native-versus-browser ratios.

## Browser comparison

Measured revision `29cfdb8cc6c4e6ed5a17647bd299591a346a6bf1` includes sparse nearest and the compatibility edit.
Historical JS remains at `a895267baea624a6e89bfcef6c5147f170e8a8f7`.
The Celeste input is 2600 by 4168 pixels. Eight color spaces each cover adaptive ordered, diffusion, and random processing.
Their respective scales are 25%, 10%, and 5%, with varied resize filters and alpha policies.

All 72 browser recipes complete without errors. Each has six cold and warm samples with alternating backend order.
Cold calls use fresh workers and processors; module import and initialization are excluded.
Warm calls recompute the requested dimensions after priming another size.
Settings-warm calls reuse dimensions after priming a different palette.
Compilation caches can persist. End-to-end dispatch-to-canvas-submission measurements remain separate in the raw results.

| Browser | Cold target passes / 24 | Warm target passes / 24 | Settings-warm target passes / 24 |
|---|---:|---:|---:|
| Chromium | 23 | 23 | 24 |
| Firefox | 24 | 24 | 24 |
| WebKit | 16 | 17 | 19 |

The target is 20% less public processing time than JS. Six samples are not a confidence-bound release gate.
Chromium's remaining warm miss is ordered linear RGB, 178.45 ms versus JS 212.55 ms.

WebKit retains several diffusion misses, despite improving against the previous Wasm package.
A fresh eight-case previous-build comparison confirms those gains.

| WebKit diffusion space | Previous Wasm warm ms | Candidate Wasm warm ms | Candidate-run JS warm ms |
|---|---:|---:|---:|
| sRGB | 33.0 | 28.0 | 16.5 |
| Linear RGB | 64.0 | 52.0 | 46.0 |
| Oklab | 87.0 | 63.5 | 83.0 |
| CIELAB | 44.0 | 34.0 | 48.0 |
| Oklch | 113.0 | 78.0 | 126.5 |
| Weighted RGB | 53.0 | 41.0 | 51.0 |
| Weighted RGB 601 | 33.5 | 29.0 | 17.5 |
| Weighted RGB 709 | 65.0 | 54.5 | 38.5 |

All 24 Chromium PNGs match the pre-adaptive baseline exactly.
All eight WebKit diffusion PNGs match the previous Wasm build exactly.
Every repeated output also matches its first output within each browser.
Firefox has no before/after PNG comparison in this trial; its repeated-output checks pass.

Integrated validation passes 464 native tests and 21 private Wasm checks. Scalar and threaded builds pass.
The delivery diff leaves `spec/` and shared `image/` unchanged.

The [evidence archive](adaptive-rows-evidence.tar.xz) retains native requests/results, frozen comparisons, executable provenance, browser timings, manifests, and test/build logs.
Its SHA-256 is `906ea942a28211879851d5b2e698dd41c161fc2bef6f1b3becc2ee1bc446bafd`.
Compiled snapshots and PNGs remain outside compiler target directories.
The broader performance target and release holds remain open.
