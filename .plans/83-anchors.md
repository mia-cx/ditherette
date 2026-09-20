# S41 historical anchor results

The historical regression gate fails. Seven of 32 runtime/workload cells show
confirmed regressions; four are inconclusive and 21 pass. Warm reuse gains do not
clear the cold regressions. This report makes no release-pass or promotion claim.

The accepted role remains freshly rebuilt pre-S32 `d51a70a2`. The protocol's
candidate role means final selected current production `a895267b` here.
The SHA-write candidate stays rejected, as recorded in [the selection report](83-selection.md).

## Scope and results

All eight original cases retain their input, settings, measurement scope, and
cache metadata. Browser roles both use ordinary scalar packages on the page.
Execution, threads, progress, row-policy, and retained-output overrides are absent.
These are historical cold/warm anchors, not a host-worker or threaded comparison.

The workloads are Lab76 quantize with 256 palette entries at 32×24; Lanczos3
resize from 129×97 to 65×49; Bayer4/Oklab separable processing with 16 entries at
65×49; and Lanczos2 plus Floyd–Steinberg Process from 129×97 to 65×49 with 16 entries.

Values below are current/pre-S32 pooled median latency ratios. Below 1 is faster.
P means the existing regression gate passes, R means confirmed regression,
and I means inconclusive. P alone is not evidence of a speedup.
Exact medians in nanoseconds and both alternating pair ratios are in
[the machine summary](83-anchors.json).

| Workload | Native | Chromium | Firefox | WebKit |
| --- | ---: | ---: | ---: | ---: |
| Quantize cold | 1.031 P | 1.129 I | 1.068 P | 1.072 I* |
| Quantize warm | 0.026 I | 0.175 P | 0.095 P | 0.169 P |
| Lanczos3 resize cold | 1.151 R | 1.516 R | 1.434 R | 1.729 R |
| Lanczos3 resize warm | 0.148 P | 0.716 P | 0.402 P | 0.765 I |
| Separable field cold | 1.001 P | 1.012 P | 0.997 P | 1.010 P |
| Separable field warm | 0.043 P | 0.022 P | 0.013 P | 0.029 P |
| Process cold | 1.039 P | 1.276 R | 1.191 R | 1.281 R |
| Process warm | 0.823 P | 0.957 P | 0.850 P | 1.018 P |

Cold resize regresses by 15.1% natively, 51.6% in Chromium, 43.4% in Firefox,
and 72.9% in WebKit. Cold Process regresses by 27.6%, 19.1%, and 28.1% in those
three browsers. Each regression exceeds 10% in both alternating pairs and passes
the timer-resolution check. Native cold Process remains below the regression threshold.

Four comparisons remain inconclusive:

- Native warm quantize has a pooled ratio of 0.0258, but pair ratios
  0.029162 and 0.025809 exceed the permitted relative spread.
- Chromium cold quantize has pair ratios 1.0971 and 1.2430. They straddle the
  regression threshold; the pooled 12.9% slowdown is not a confirmed regression.
- WebKit cold quantize is the starred resolution-limited cell. Both nominal
  ratios are about 1.073, but the measured 20 µs clock quantum prevents a pass.
- WebKit warm resize has pair ratios 0.8235 and 0.7429. Their relative spread
  exceeds 10%, despite the lower pooled current median.

Warm quantize passes with lower medians in all three browsers. Warm resize
passes with lower medians in native, Chromium, and Firefox. Warm separable
processing passes with lower medians in every runtime. Warm Process improves in
native, Chromium, and Firefox; WebKit's 1.018 ratio passes without a speedup.

Warm samples use a fresh processor and an untimed declared prime for every call.
Quantize primes no-dither, resize primes the same call, separable processing
primes perturb, and Process primes resize. Priming and disposal stay outside
timers. Ordinary complete-call copies, hashing, and processing remain inside.
The native and browser complete-call scopes stay separate.

## Completion and exactness audit

The lane runs from `2026-09-09T00:32:16.683Z` to
`2026-09-09T00:38:16.719Z`, taking 360.036 seconds within its 20-minute launch deadline.
Native, Chromium, Firefox, and WebKit coordinators run sequentially. Each exits 2
because its completed report contains a regression, not because transport failed.

There are 128 started workers and 128 matching reaps, with no overlapping owned
workers. Each runtime has 32 workers. The event audit verifies accepted/candidate
order in pair 0 and candidate/accepted order in pair 1 for all eight cases.
Every worker reports one live benchmark process. No missing or short worker
records occur. All 128 workers record 20 single-call samples, totaling 2,560.
Each case/runtime has 40 samples per role across two fresh pairs.
Warmup is 50 ms and the accumulated measured-time cap is 10 seconds.

All 64 actual production pairs have identical complete outputs, including indexed
palette metadata and warnings. All 128 worker outputs equal their local frozen
reference. The 64 separate artifact-local reference pairs also agree.
The report contains 128 exact verification records because each actual pair has
a separate reference-probe record. These are not 128 independent production pairs.
No mismatch or instability gate fires. The permanent browser observer checks
each output; native conformance keeps its established before/after observation
scope and does not claim transient per-sample detection.

The recorded host attestation drains agents, builds, and tests before timing.
The launch/reap audit proves serial owned execution, not global host quiescence.
The coordinator records unrelated host load without stopping background services.
No retry or candidate promotion occurs. This reporting task only reads retained
evidence and writes these report files.

## Fresh identities and retained evidence

| Role | Full source revision | Ordinary tarball SHA-256 |
| --- | --- | --- |
| Accepted pre-S32 | `d51a70a2daf054357d16ea66b235da3733c02888` | `83a07b5a9bc7f0f04ba1bcb4c6db13f8daa6101bf4b656651aa06be9f1cf59eb` |
| Final selected current | `a895267baea624a6e89bfcef6c5147f170e8a8f7` | `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d` |

Native worker SHA-256 digests are
`c0498434d41f774cc076d9d587a6944669b73140a4cb123e7bfbb35ac47bc715` for accepted and
`2c26ec86d5ba668667438713ecbeb9c44fc2c384ab0e05a32e3ff09aa4320650` for current.
The current tarball reproduces the retained S40 ordinary artifact byte-for-byte.

Raw requests, outputs, all samples, completion records, event streams, and
prepared asset snapshots remain in
`.worktrees/v1-resize-integration/target/s41-anchors-01`.
Fresh source and build provenance remain in `.worktrees/v1-s32-bench` under
`target/s41-anchor-{native,public}` and `.worktrees/v1-s41-current-source`
under `target/s41-current-{native,public}`. Neither immutable source tree changes.

The machine summary records full native/public provenance hashes, coordinator
hashes, both packaged Wasm hashes, role asset-tree hashes, frozen oracle manifest
hashes, runtime hashes, and per-report/prepared/event hashes. It also retains
actual build-tool versions and executable digests.

Both roles use Rust 1.97.0 (`2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`),
Node 24.19.0, pnpm 11.13.0, wasm-pack 0.15.0, wasm-bindgen 0.2.121, wasm-opt 117,
and TypeScript 6.0.3. The packaged threaded variant uses genuine
`nightly-2024-08-02`; it is not measured in this lane.
Browser versions are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4,
with Playwright 1.59.1. Observed clock quanta are about 5 µs, 20 µs, and 20 µs.
The recorded Linux host has an AMD Ryzen 9 7950X and 24 logical CPUs.

| Report | SHA-256 |
| --- | --- |
| native | `ae46c1baf609a8d5acd7fb55f272c527c7e6fa4c6b235645cecf31350d9d6bb0` |
| chromium | `93c16d439b15c21d91df9440ea66a1e18194dcf8069310c3f502e98d3afb2829` |
| firefox | `e42f96dcb728ad3f8e917511d3210ee23c6eed781f00fe67fa5609653cc1edd5` |
| webkit | `6b3effb87ff7b0a17b8941a551a5870e01d3571303a7ee01fbb929c08c7523d5` |

The frozen oracle is exact for these fixtures. Their failure is performance,
not inherited resize drift or newly accepted approximation. Confirmed cold
regressions and inconclusive required cases remain open release gates.
