# S22 restored convolution measurements

## Artifacts and scope

Measured runtime `1761705e2c6935544b0232427d48129059d89615` joins S21 and S22 without replacing landed kernels.
Fresh native accepted `e64ee3f43d547edd2424c34392e01990c2442c5d` retains restored production from `467542f4` byte-for-byte.
Its only overlay fixes benchmark identity validation. Both roles use the same protocol and toolchain.
Candidate native calls include fallible plan and scratch preparation, execution, and temporary cleanup.
The caller owns output allocation outside native timing. Public timing includes the complete package call.

The installed package SHA-256 is `91729b064dc856fa3cac256369e73f98a41b847aed95f568cf32e2a4a544ac5e`.
The retained tarball is `.worktrees/v1-resize-integration/target/resize-public-build-01/ditherette.tgz`.
Raw requests, outputs, timings, manifests, review PNG/JSON, and process events remain under:
`.worktrees/v1-resize-integration/target/resize-trial-01/s22-{native,chromium,firefox,webkit}-results`.
Corresponding `*-prepared` directories contain immutable executable and runtime snapshots.

Node 24.19.0 and Playwright 1.59.1 run headless Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
WebKit runtime aliases retain their hardlink identity. No extra browser launch arguments were added.

## Completed budget

Two alternating AB/BA pairs cover six filter/support combinations in latency and throughput.
Each worker permits 20 samples, 50 ms warmup, 250 ms measurement, and a 2 ms throughput target.
No retry or tuning run follows this matrix.

| Run | Workers launched and reaped | Samples |
| --- | ---: | ---: |
| Native | 48 | 960 |
| Chromium | 48 | 960 |
| Firefox | 48 | 711 |
| WebKit | 48 | 960 |

The complete S21/S22 phase reaps all 304 workers and retains 5,760 samples.
Every process trace records at most one live benchmark worker. Agents, builds, and tests remained drained throughout measurement.
All eight coordinators finish with exit 2 and complete reports because strict reference gates reject existing output differences.
No transport failure, missing result, or unstable-output trial occurred.

## Native preservation and cost

All accepted/candidate production images are byte-identical across both pairs.
The report alternates production comparisons with cross-artifact reference checks; these are distinct entries.
Fixed-support cases pass the strict frozen gate. Scale-aware cases retain their existing bounded reference differences.

| Filter/support | Candidate/accepted latency | Candidate/accepted throughput |
| --- | ---: | ---: |
| Bicubic fixed | 0.997349 | 1.009280 |
| Lanczos2 fixed | 1.001730 | 0.997001 |
| Lanczos3 fixed | 0.990491 | 1.001148 |
| Bicubic scale-aware | 0.992136 | 0.983533 |
| Lanczos2 scale-aware | 0.994703 | 1.007802 |
| Lanczos3 scale-aware | 0.989336 | 0.992300 |

No native pair shows a slowdown above 10%. These results do not justify changing the landed loops or plans.
No new approximation or optimization is selected.

## Public diagnostics

Cells show package milliseconds / actual website TypeScript milliseconds for reduction latency.
These are non-equivalent-output diagnostics, not release passes or equivalent-output speedups.

| Filter/support | Chromium | Firefox | WebKit |
| --- | ---: | ---: | ---: |
| Lanczos2 fixed | 1.5875 / 2.3875 | 13.24 / 1.24 | 1.44 / 1.28 |
| Lanczos3 fixed | 2.79 / 2.6925 | 25.77 / 1.56 | 2.56 / 1.62 |
| Lanczos2 scale-aware | 3.35 / 4.17 | 31.64 / 2.92 | 3.26 / 2.36 |
| Lanczos3 scale-aware | 4.6625 / 5.3475 | 44.93 / 3.95 | 4.48 / 3.06 |

The website lacks bicubic. Its declared controls compare the same package artifact against itself.
Fixed bicubic candidate latency is 1.575 / 13.23 / 1.42 ms across Chromium / Firefox / WebKit.
Scale-aware bicubic candidate latency is 3.2975 / 31.82 / 3.17 ms.
The paired control ratios remain near one. Fixed bicubic passes strict frozen equality; scale-aware keeps its existing differences.

S41 must investigate complete-call browser costs, especially Firefox, using freshly paired artifacts and faithful comparisons.
These integration slices do not claim release-performance readiness. The restored kernels remain unchanged.
