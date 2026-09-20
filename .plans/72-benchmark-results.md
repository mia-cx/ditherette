# S31 preparation benchmark results

Retain the required S31 preparation baseline. Warm Lanczos3 improves across all four runtimes and satisfies the bounded preparation target.
Chromium and Firefox pass the declared regression gate. Native and WebKit remain inconclusive.
No case confirms a regression above 10%. This is not a universal performance pass.

The coordinator selected no additional candidate or retry. This report records the completed trial; it does not change the protocol or policy.

## Scope and source identity

Issue [#72](https://github.com/mia-cx/ditherette/issues/72), using the matrix declared in [72-benchmark.md](72-benchmark.md).

- Accepted uncached S30 plus shared protocol: `863889e52f1b752b6adfc22a9c775b3823f2997e`.
- Candidate S31 preparation plus the same protocol: `972d4e9a5882b25bca3de5f0786ad1525b5e6329`.
- Frozen reference: `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, artifact `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

Only `*-prepared-v2` artifacts supplied the four `*-results` directories.
The initial unsuffixed preparations bind `fef6cc63`. They were retained unused; no measurements ran against them.
Candidate package conformance separately used byte-identical package artifacts from that earlier build, as explained below.

Four workloads each have cold and warm cases:

| Workload | Measured input and settings |
| --- | --- |
| Quantize | 32×24 RGBA8, 256 ordered colors, Lab76 matching |
| Lanczos3 resize | 129×97 → 65×49, center anchor, scale-aware support |
| Trilinear resize | 256×192 → 65×49, center anchor |
| Process | 129×97 → 65×49, scale-aware center Lanczos2, 16 colors, Floyd–Steinberg byte feedback, strength 0.7, serpentine |

Each runtime ran two alternating role pairs per case, with 20 single-call samples per worker.
Warmup was 50 ms; the measurement cap was 10 seconds.
Cold creates an empty processor outside each timer. Warm primes once with changed source bytes but identical geometry, palette, and settings.
Priming is untimed; measured calls restore the identity-named source bytes.
Ordinary calls include key hashing, preparation, input copies, and durable result construction.
Source-content hashing belongs to S32 and is absent here.

Native result observation, prior output destruction, and processor setup/destruction are untimed.
Earlier S30 native subjects timed output destruction; their historical numbers are not comparable to this trial.
Browser initialization, priming, output observation, and disposal are also untimed.
The authoritative scope record is `trial-index.json`.

## Completed execution and exactness

| Runtime | Gate | Started/reaped | Actual pairs | Samples | Warmup calls |
| --- | --- | --- | --- | --- | --- |
| native | inconclusive | 32/32 | 16 | 640 | 4380 |
| chromium | pass | 32/32 | 16 | 640 | 1095 |
| firefox | pass | 32/32 | 16 | 640 | 240 |
| webkit | inconclusive | 32/32 | 16 | 640 | 932 |
| Total | inconclusive | 128/128 | 64 | 2,560 | 6,647 |

Raw events confirm a global maximum of one live benchmark worker, with matching PIDs and no unreaped workers.
All 128 workers recorded 20 samples and one call per sample.
The 64 actual accepted/candidate pairs use both alternating orders.

Read-only verification recomputed role medians and pair ratios from raw samples.
It compared every worker output to its artifact-local frozen output, then compared both role outputs and references in each pair.
All 128 target-local outputs, all 64 pair outputs, and all 128 recorded three-way proofs are exact.
That includes dimensions, indexed bytes or RGBA bytes, palette metadata, transparency, and warnings.
These are target-local comparisons, not a claim that all native and browser arithmetic is identical.

## Timing results

Each entry is candidate median divided by accepted median, pooled over 40 samples per role.
Lower is faster. `I` marks an inconclusive gate, not a failed exactness check.
Unrounded medians and both per-order ratios are in [72-benchmark-results.json](72-benchmark-results.json).

| Case | Native | Chromium | Firefox | WebKit |
| --- | --- | --- | --- | --- |
| Lab76 cold | 1.0100 | 1.0702 | 1.0317 | 1.0615 (I) |
| Lab76 warm | 1.0188 | 1.0711 | 1.0471 | 1.0820 (I) |
| Lanczos3 cold | 1.0092 | 1.0097 | 1.0133 | 1.0204 (I) |
| Lanczos3 warm | 0.8750 | 0.9500 | 0.9368 | 0.8947 |
| Trilinear cold | 1.0000 | 0.9967 | 0.9895 | 0.9855 |
| Trilinear warm | 1.0079 | 0.9796 | 0.9946 | 1.0172 |
| Process cold | 0.9772 | 1.0000 | 1.0119 | 1.0159 |
| Process warm | 1.0109 (I) | 0.9817 | 0.9433 | 0.9643 |

Warm Lanczos3 is 12.50% faster natively, 5.00% in Chromium, 6.32% in Firefox, and 10.53% in WebKit.
Both alternating ratios are below 1 in every runtime.
Trilinear provides no consistent warm preparation benefit. Lab76 is slower in this matrix, without a confirmed >10% regression.
There is no universal filter speedup claim.

Four cases remain inconclusive:

| Runtime/case | Alternating ratios | Reason |
| --- | --- | --- |
| Native warm Process | 1.051548, 0.945289 | Pair spread exceeds the 1.10 stability bound |
| WebKit cold Lab76 | 1.069767, 1.045455 | Timer-quantum margin cannot establish the ≤1.10 gate |
| WebKit warm Lab76 | 1.098361, 1.032787 | Timer-quantum margin cannot establish the ≤1.10 gate |
| WebKit cold Lanczos3 | 0.960000, 1.041667 | Timer-quantum margin cannot establish the ≤1.10 gate |

The gate requires both orders to establish the threshold and accounts for one timer quantum per elapsed observation.
It is not a confidence-interval estimate. No inconclusive case is promoted to a pass.

## Artifact sizes

| Artifact | Accepted bytes | Candidate bytes | Change |
| --- | --- | --- | --- |
| Packed package .tgz | 308,343 | 327,780 | +19,437 (6.30%) |
| Installed package raw | 806,698 | 867,086 | +60,388 (7.49%) |
| Installed package gzip sum | 325,666 | 344,906 | +19,240 (5.91%) |
| Scalar Wasm raw | 267,046 | 289,912 | +22,866 (8.56%) |
| Scalar Wasm gzip | 122,604 | 131,540 | +8,936 (7.29%) |
| Threaded Wasm raw | 358,234 | 395,756 | +37,522 (10.47%) |
| Threaded Wasm gzip | 153,710 | 164,014 | +10,304 (6.70%) |

Installed-package totals cover 38 regular files. Gzip sums compress each file separately with Node zlib level 9.
The packed `.tgz` figures are original archive sizes; they use a different compression/layout boundary.
Threaded Wasm is packaged but was not measured here. These browser calls use the scalar backend.

Accepted package SHA-256: `c2725329e6dd9016c956274a1c914349b7806a373762d25411b46ed7e4d0cc71`.
Candidate package SHA-256: `ce556c8dc5a7f747ebdd572d84d0185a9d2db2796cd9d3af085905fa3d9bb4a0`.

## Evidence and limits

Raw evidence remains read-only under:
`/home/mia/mia-cx/ditherette/.worktrees/v1-s31-preparation/target/s31-trial-01`.

The JSON report records source revisions, worker/coordinator/package/Wasm hashes, provenance hashes, and per-runtime result inventories.
Each inventory digest covers every direct result-directory file, including requests, results, stderr, browser records, events, and prepared metadata.
Its encoding is documented beside the digest so it can be recomputed.
Prepared worker hashes and every browser asset file were checked against their manifests.
Prepared package files also match the corresponding fresh role artifact.

Role artifacts remain in `v1-s31-bench/target/s31-baseline-863889e-*` and
`v1-s31-preparation/target/s31-candidate-972d4e9a-*`.
The final public artifact's `conformance.md` binds the corrected bounded-memory fixture to the byte-identical tested package.
All 38 files match the preceding package build, including both Wasm variants.
Its retained Chromium, Firefox, and WebKit conformance reports bind the candidate tarball digest.
The conformance note's “No performance measurement has run” sentence describes its pre-trial checkpoint, not this completed trial.

Execution used `athena-hephaestus`, Linux `6.12.95+deb13-amd64`, AMD Ryzen 9 7950X, with 24 exposed logical CPUs.
Toolchains were Rust 1.97.0, Node 24.19.0, and Playwright 1.59.1.
Browsers were Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Observed timer resolution was 5 µs in Chromium and 20 µs in Firefox/WebKit.
All browser records were cross-origin isolated.
The trial ran on 2026-09-08, from 14:58:25 to 15:03:12 UTC, serially by runtime.
Host load is retained in `host-before.json` and every lifecycle event.
No temperature readings were available; no temperature normalization was applied.

This bounded matrix does not establish larger-image, universal filter, or threaded performance.
No further measurements, builds, tests, source edits, or protocol edits were made while preparing this report.
The coordinator owns PR integration and eventual compiler-cache cleanup. Preserve measurement and conformance evidence.
