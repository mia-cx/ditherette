# S41 automatic continuation

Automatic coverage now includes all twenty required Chromium and Firefox cells.
Its gates still fail. Firefox's two warm controls regress, Chromium warm mixing is inconclusive, and both bilinear cells are incorrect.

The continuation completes thirteen fresh cells, with 52 workers and 814 samples.
Combined with the seven [original Chromium reports](83-matrix.md), coverage contains 80 workers and 1,318 samples.
The failed original Chromium case 7 remains a separate attempt.
None of its samples or incomplete pairs contribute to these coverage totals.

## Results

The accepted role disables threads. The candidate role requires threads.
Both run the selected ordinary package in host workers, with no developer row policy.
Each case uses two fresh alternating pairs and the existing complete-call protocol.

Values are scalar / required-threaded pooled medians in milliseconds, followed by the threaded/scalar ratio.
P means pass, R confirmed regression, I inconclusive, and X incorrect.
An asterisk identifies the seven earlier Chromium reports. Every other cell belongs to this continuation.
Exact nanosecond medians and both pair ratios remain in the [machine summary](83-automatic-continuation.json).

| Case index and workload | Chromium, ms; ratio | Firefox, ms; ratio |
| --- | ---: | ---: |
| 0 common-direct-srgb16 | 304.778 / 139.918; 0.459 P * | 1935.420 / 857.030; 0.443 P |
| 1 area-selected | 57.620 / 52.825; 0.917 P * | 354.350 / 330.940; 0.934 P |
| 2 bilinear-selected | 56.662 / 52.217; 0.922 X * | 346.670 / 320.520; 0.925 X |
| 3 lanczos3-selected | 143.110 / 110.468; 0.772 P * | 984.500 / 671.480; 0.682 P |
| 4 perturb-srgb16-random | 2733.715 / 1435.513; 0.525 P * | 24360.000 / 12344.120; 0.507 P |
| 5 separable-oklab64-blue-adaptive2 | 221.922 / 151.627; 0.683 P * | 1981.460 / 1297.590; 0.655 P |
| 6 yliluoma-medium-p8-bayer4-srgb-adaptive-cold | 230.695 / 80.383; 0.348 P * | 2029.930 / 677.950; 0.334 P |
| 7 yliluoma-process-p4-bayer4-oklch-adaptive-cold | 59.362 / 41.158; 0.693 P | 524.480 / 343.100; 0.654 P |
| 8 preview-nearest-final-hit | 3.885 / 4.252; 1.095 P | 19.770 / 24.260; 1.227 R |
| 9 yliluoma-medium-p8-bayer4-srgb-adaptive-cold-final-hit | 0.145 / 0.157; 1.086 I | 0.520 / 0.670; 1.288 R |

Fifteen combined cells pass. Fourteen have lower threaded medians; Chromium warm nearest has a higher median.
The passing gate checks regressions against 10%; it does not prove a speedup.
No size threshold or production policy changes follow from this report.

Firefox warm nearest regresses by 22.7%.
Its alternating ratios are 1.255 and 1.191.
Firefox warm mixing regresses by 28.8%.
Its alternating ratios are 1.360 and 1.269.
Both pairs exceed 10% in each case, with usable timer resolution.

Chromium warm mixing remains inconclusive.
Its pooled ratio is 1.086, but pair ratios 1.121 and 1.088 straddle the regression threshold.
The pooled median cannot clear that gate.
Chromium warm nearest passes at a 1.095 ratio; it does not show a threaded speedup.

Bilinear remains incorrect in both browsers.
Each production output differs from frozen reference by 33 bytes across 33 pixels, with maximum difference one.
Scalar and threaded production outputs agree exactly.
Their agreement does not clear the frozen mismatch, so bilinear timings remain diagnostic.

## Completion and exactness

| Evidence scope | Completed cells | Workers | Samples | Gates |
| --- | ---: | ---: | ---: | --- |
| Original Chromium 0–6 | 7 | 28 | 504 | 6 pass; 1 incorrect |
| Fresh Chromium 7–9 | 3 | 12 | 240 | 2 pass; 1 inconclusive |
| Fresh Firefox 0–9 | 10 | 40 | 574 | 7 pass; 2 regression; 1 incorrect |
| Combined automatic coverage | 20 | 80 | 1,318 | 15 pass; 2 regression; 2 incorrect; 1 inconclusive |

The continuation runs from `2026-09-09T17:44:49.221Z` to `2026-09-09T18:07:14.403Z`.
Its wall time is 1,345.182 seconds, within the forty-minute launch budget.
All thirteen outcome records exist. Four coordinator exits are code 2 for completed nonpassing reports.
There is no transport failure, missing report, or incomplete worker in this continuation.

All 52 starts have matching reaps. Each owned worker exits before the next starts.
Every complete worker reports one live benchmark process.
Each case runs accepted/candidate in pair 0, then candidate/accepted in pair 1.
The event audit proves serial owned execution, not global host quiescence.
The preparation attestation records drained agents, builds, and tests, with unrelated services untouched.

Workers record 5, 6, 8, 10, 11, 12, 15, or 20 single-call samples.
The existing ten-second accumulated measured-time cap permits slow calls to exceed ten seconds.
Warm controls keep their declared untimed primes; ordinary copies, hashing, and processing remain inside complete-call timers.

All 26 production pairs in the continuation agree exactly.
All 26 separate artifact-local reference pairs also agree.
The 52 verification entries represent these two checks per pair, not 52 production pairs.
Forty-eight of 52 worker outputs equal frozen reference; four Firefox bilinear outputs account for the failures.
No additional instability gate appears.

Across both attempts' completed cells, all forty production pairs and forty reference pairs agree.
Seventy-two of eighty worker outputs equal frozen reference.
The eight bilinear outputs account for all frozen mismatches.

The machine summary preserves all 52 result sample arrays and all 52 browser transport arrays.
These arrays represent 814 observations, not 1,628 independent samples.
Their largest re-encoding difference is 0.000000239 ns.
Full event journals, exactness details, identities, and SHA-256 hashes remain in the summary.
Raw source and output image payloads are omitted and cannot be reconstructed from it.

## Fresh artifact bindings

Continuation source is `f0736c782615a911652ecaf21d1f9a8660ae3e3d`.
The [composition fix](83-composition-fix.md) retains each role's thread policy during untimed staged verification.
The source diff against selected `a895267b` contains only plan documents and three benchmark scripts.
Production content stays unchanged, and the fresh ordinary tarball reproduces the selected artifact byte-for-byte.

Earlier Chromium cells retain their original `a895267baea624a6e89bfcef6c5147f170e8a8f7` build identities.
The continuation uses its actual new source and native executable identities.
Coverage joins complete cells across the two attempts; it does not relabel artifacts or combine their pairs.

| Artifact | SHA-256 |
| --- | --- |
| Ordinary tarball, unchanged | `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d` |
| Fresh native worker | `4d32bbf3041bcfa1487f7271d477293e855e8c1b11182601667e2c2cc8d0cc81` |
| Fresh coordinator | `a07e0567e849488055737991f832f4718224d01815b223eca5c63adee14c1388` |
| Scalar Wasm, unchanged | `cdc795079ed5f8f0d635d9d59b78fdcf6b5f4abced1d836fda4fb5827933189e` |
| Threaded Wasm, unchanged | `eddd706d6137587124db9575f52ffe756dba040a2b99085e3ae451b1b5dcd4e2` |
| Public provenance | `0334ed3b790472c78c4ca4f733bdc5c85d1dd7a6ad82d144f2018173b5c44743` |
| Native provenance | `9ea93a5e267043958fa3048185e4b6d8e2ffd6443a2e952cdada69c16a3f515c` |
| Corrected benchmark page | `971716576c7ceb330af8b879395bc1d4d7b0ebe426e15d087ff134b1cc1a8230` |
| Frozen oracle manifest | `2259b9dfcd3a12118859ecfad3e0bca3f400b86e55777c16ce95bad8e69cfee8` |

The summary preserves per-case input/settings identities, prepared-manifest hashes, result hashes, and browser asset-tree identities.
It also retains the original summary hash and every included case's report hash.
Each source/build record binds the continuation revision without changing the selected package's identity.

Versions remain Rust 1.97.0, Node 24.19.0, pnpm 11.13.0, TypeScript 6.0.3, and Playwright 1.59.1.
Wasm tools are wasm-pack 0.15.0, wasm-bindgen 0.2.121, and wasm-opt 117.
Threaded artifacts use `nightly-2024-08-02`.
Measured browsers are Chromium 147.0.7727.15 and Firefox 148.0.2.
The recorded Linux host has an AMD Ryzen 9 7950X and 24 logical CPUs.

## Remaining gates

Automatic coverage is complete, but its two regressions, two incorrect cells, and one inconclusive cell remain blocking.
The [historical anchor report](83-anchors.md) retains seven confirmed regressions and four inconclusive cells.
The original matrix retains nine TypeScript regressions and three inconclusive scalar initialization cells.
Its Chromium and Firefox release bilinear cells also remain incorrect.

Release still lacks 21 case reports.
Firefox case 9 remains interrupted; Firefox cases 10–14 and all fifteen WebKit cases remain unrun.
The [capped resource check](83-capped-probe.md) fails transport, so capped measurements remain held.
Threaded WebKit stays excluded pending atomic-wait cleanup.
General website field, diffusion, perceptual, and mixing equivalence remains unavailable.

This continuation clears the missing automatic coverage, not the release gate.
This reporting task runs no build, test, browser, or timing work.
