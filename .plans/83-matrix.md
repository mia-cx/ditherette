# S41 fixed matrix evidence

Release remains blocked. This retained attempt has 50 completed reports across five lanes.
Nine TypeScript comparisons regress, three bilinear cells are incorrect, and three initialization cells remain inconclusive.
The [historical anchor failures](83-anchors.md) also remain open. This report makes no release-pass claim.

Production stays at selected current `a895267baea624a6e89bfcef6c5147f170e8a8f7`.
The [batching candidate remains unselected](83-selection.md).
Release and initialization compare the selected implementation against itself.
Those ratios measure repeatability, not improvement against S40 or pre-S32.

## Completion and preservation

The [machine summary](83-matrix.json) preserves every recorded sample array, case identity, median, ratio, gate, and event.
It includes source/build identities, asset hashes, report hashes, prepared-manifest hashes, result hashes, and failure logs.
Browser transport arrays and serialized worker arrays both remain available.
Their largest re-encoding difference is 0.000000477 ns; they represent the same observations, not additional samples.

| Lane | Reports / required cells | Workers in reports | Samples in reports | Recorded gates |
| --- | ---: | ---: | ---: | --- |
| Release | 24 / 45 | 96 | 1,658 | 22 pass; 2 incorrect |
| Actual website TypeScript | 9 / 9 | 36 | 720 | 9 regression |
| Scalar initialization | 6 / 6 | 24 | 480 | 3 pass; 3 inconclusive |
| Required-thread initialization | 4 / 4 | 16 | 320 | 4 pass |
| Automatic, original attempt | 7 / 20 | 28 | 504 | 6 pass; 1 incorrect |
| Total | 50 / 84 | 200 | 3,682 | 35 pass; 9 regression; 3 incorrect; 3 inconclusive |

Completed reports contain two alternating pairs each.
Every worker records between five and twenty single-call samples.
Release workers reach 5, 6, 10, 11, 17, 18, or 20 samples.
Automatic workers reach 5, 7, or 20. Other completed lanes reach twenty throughout.
The 10-second accumulated measured-time cap does not prevent a slow call from exceeding that duration.

Incomplete attempts retain another 35 observations, excluded from the table and all completed-pair counts.

- Firefox release case 9 has two complete result records with five samples each.
  Its third worker exits, but its final result file is zero bytes after disk exhaustion.
  The third browser transport preserves five samples separately. Pair 1 lacks its accepted worker.
- Automatic Chromium case 7 retains twenty accepted-role samples.
  Its candidate worker fails scalar initialization with threaded bytes and leaves an empty result.
  There is no completed pair or case report.

These five extra Firefox transport observations remain transport-only evidence.
They do not replace the missing final worker result or complete the interrupted case.
The summary retains 3,717 unique recorded observations in total.
Original target paths identify the evidence source; cleanup may remove those payloads after preservation.
The compact summary omits raw image payloads and cannot reconstruct them.
[Disk recovery](83-disk-recovery.md) explains the archive and cleanup policy.

## Release baseline

The ordinary scalar package runs in host workers.
Both roles use the same selected source, native worker, and browser assets.
Fresh complete calls retain ordinary copies, hashing, preparation, processing, and output checks.
Warm final-hit controls retain their declared untimed prime.

Values are accepted / candidate pooled medians in milliseconds.
P means the regression gate passes, X means frozen exactness fails.
The machine summary keeps exact nanosecond medians and both alternating ratios.

| Case index and workload | Chromium, ms | Firefox, ms |
| --- | ---: | ---: |
| 0 preview-nearest | 5.250 / 5.250 P | 26.090 / 25.870 P |
| 1 common-direct-srgb16 | 303.815 / 303.030 P | 1966.030 / 1959.920 P |
| 2 large-nearest | 222.195 / 221.093 P | 1076.160 / 1079.570 P |
| 3 area-selected | 57.715 / 56.600 P | 356.820 / 354.750 P |
| 4 bilinear-selected | 57.250 / 57.832 X | 350.290 / 349.670 X |
| 5 lanczos3-selected | 142.580 / 141.898 P | 972.610 / 974.140 P |
| 6 common-process-floyd-steinberg | 587.185 / 583.870 P | 4084.840 / 4084.080 P |
| 7 preview-process-sierra-oklab | 16.057 / 16.297 P | 115.350 / 114.520 P |
| 8 extra-trilinear | 3.682 / 3.655 P | 24.900 / 24.640 P |
| 9 perturb-srgb16-random | 2775.727 / 2760.130 P | incomplete |
| 10 separable-oklab64-blue-adaptive2 | 222.852 / 224.793 P | unrun |
| 11 yliluoma-medium-p8-bayer4-srgb-adaptive-cold | 231.318 / 233.607 P | unrun |
| 12 yliluoma-process-p4-bayer4-oklch-adaptive-cold | 58.947 / 59.690 P | unrun |
| 13 preview-nearest-final-hit | 3.910 / 4.030 P | unrun |
| 14 yliluoma-medium-p8-bayer4-srgb-adaptive-cold-final-hit | 0.150 / 0.148 P | unrun |

All fifteen Chromium cases and the first nine Firefox cases have complete reports.
Firefox case 9 is interrupted; Firefox cases 10–14 and all fifteen WebKit cases are unrun.
The release completion file is zero bytes. It cannot establish elapsed lane time or successful completion.
The retained first-launch-to-last-reap span is 1,884.601 seconds, not a completed lane wall time.

Bilinear differs from the frozen reference in both browsers.
Each output has 33 differing bytes across 33 pixels, with maximum absolute difference one.
Both production roles agree exactly. Their agreement does not waive the frozen mismatch.
Measured bilinear medians remain diagnostic evidence under the incorrect gate.
The selected area and Lanczos3 fixtures pass exactness; this does not clear historical drift in other resize fixtures.

## Actual website TypeScript

The accepted role runs the actual website TypeScript closure.
The candidate role runs the selected ordinary scalar package.
Both execute on the page with no application-cache claim.
The fixtures admit nearest, exact-palette direct quantize, and nearest plus no-dither Process.
Their palettes retain order and explicit transparency.

Values are TypeScript / package medians in milliseconds.
Every comparison exceeds the 10% regression threshold in both alternating pairs.
All nine gates report confirmed regression with usable timer resolution.

| Engine | Workload | TypeScript / package, ms | Package / TypeScript |
| --- | --- | ---: | ---: |
| chromium | nearest-preview | 0.210 / 5.287 | 25.179 R |
| chromium | direct-palette-exact | 2.082 / 18.300 | 8.788 R |
| chromium | process-palette-exact | 1.427 / 8.847 | 6.198 R |
| firefox | nearest-preview | 0.180 / 25.140 | 139.667 R |
| firefox | direct-palette-exact | 2.250 / 114.450 | 50.867 R |
| firefox | process-palette-exact | 1.130 / 48.970 | 43.336 R |
| webkit | nearest-preview | 0.160 / 5.100 | 31.875 R |
| webkit | direct-palette-exact | 1.800 / 20.480 | 11.378 R |
| webkit | process-palette-exact | 1.220 / 9.280 | 7.607 R |

All 36 output checks and all eighteen production pairs are exact.
This proves equivalence for the admitted fixtures only.
General website field, diffusion, perceptual, and mixing parity remains unavailable.
Absent website modes retain package-only baselines; they do not establish TypeScript speedups.

The lane completes in 142.000 seconds.
Its nine coordinator exits are code 2 because the reports contain regressions.

## Initialization

Both roles use the same selected package and runtime mode.
Bytes and precompiled-module preparation remain separate timing scopes.
Initialization covers fresh setup; it does not measure a complete processing call.
Required-thread setup includes the required threaded initialization path.

| Mode | Engine | Preparation | Accepted / candidate, ms | Gate |
| --- | --- | --- | ---: | --- |
| Scalar | chromium | bytes | 0.472 / 0.472 | P |
| Scalar | chromium | compiled | 0.090 / 0.090 | I |
| Scalar | firefox | bytes | 5.490 / 5.240 | P |
| Scalar | firefox | compiled | 0.140 / 0.140 | I |
| Scalar | webkit | bytes | 1.230 / 1.220 | P |
| Scalar | webkit | compiled | 0.200 / 0.200 | I |
| Threads | chromium | bytes | 61.502 / 59.400 | P |
| Threads | chromium | compiled | 58.877 / 59.573 | P |
| Threads | firefox | bytes | 44.770 / 45.020 | P |
| Threads | firefox | compiled | 36.720 / 36.490 | P |

Scalar compiled initialization stays inconclusive in all three browsers.
Chromium is resolution-limited.
Firefox pair ratios are 0.857 and 1.286.
WebKit pair ratios are 1.056 and 0.955.
These spreads prevent passes even though pooled medians nearly match.

Scalar initialization completes in 77.304 seconds.
Required-thread initialization completes in 68.640 seconds.
Threaded WebKit remains excluded pending the atomic-wait cleanup gate.

## Automatic execution, original attempt

The accepted role disables threads. The candidate role requires threads.
Both use the same selected ordinary package in host workers, with no developer row policy.
Values are scalar / required-threaded pooled medians in milliseconds.
Only the first seven Chromium cases complete.

| Case index and workload | Scalar / threaded, ms | Threaded / scalar |
| --- | ---: | ---: |
| 0 common-direct-srgb16 | 304.778 / 139.918 | 0.459 P |
| 1 area-selected | 57.620 / 52.825 | 0.917 P |
| 2 bilinear-selected | 56.662 / 52.217 | 0.922 X |
| 3 lanczos3-selected | 143.110 / 110.468 | 0.772 P |
| 4 perturb-srgb16-random | 2733.715 / 1435.513 | 0.525 P |
| 5 separable-oklab64-blue-adaptive2 | 221.922 / 151.627 | 0.683 P |
| 6 yliluoma-medium-p8-bayer4-srgb-adaptive-cold | 230.695 / 80.383 | 0.348 P |

Six completed cells pass with lower threaded medians.
Bilinear remains incorrect despite the lower diagnostic median.
No new size threshold is selected from these timings.

Chromium case 7, mixing Process, fails during candidate staged verification.
The retained exception reports `DitheretteError: Scalar initialization failed.`
The benchmark role remap supplies threaded bytes to scalar initialization.
The coordinator then reports an incomplete transport response and exits 1.

Chromium cases 8–9 and all ten Firefox cases remain unrun in this attempt.
The completion record spans 345.255 seconds, including the failed case.
It records stopped execution, not automatic-lane completion.
A corrected fresh continuation must remain separate and must not overwrite this failed attempt.

## Exactness and serial audit

All 100 production-role pairs in completed reports agree exactly.
All 100 separate artifact-local reference pairs also agree.
The 200 verification entries comprise one production record and one reference-probe record per pair.
They are not 200 independent production pairs.

Of 200 worker outputs in completed reports, 188 equal the frozen reference.
The twelve bilinear outputs account for every frozen mismatch.
No additional instability gate appears in the completed reports.

The event journals record 205 starts and 205 matching reaps.
Each owned worker exits before the next starts, across all five lanes.
Every complete result reports one live benchmark process.
Every reported case uses accepted/candidate order in pair 0, then candidate/accepted order in pair 1.
The journals prove serial owned execution; they do not prove global host quiescence.
The preparation attestation records drained agents, builds, and tests while leaving unrelated services untouched.

| Lane | First recorded event, UTC | Last recorded event, UTC |
| --- | --- | --- |
| Release | 00:53:42.591 | 01:25:07.192 |
| TypeScript | 17:19:12.535 | 17:21:33.062 |
| Scalar initialization | 17:21:34.379 | 17:22:50.395 |
| Thread initialization | 17:22:51.663 | 17:23:59.292 |
| Automatic original attempt | 17:24:39.515 | 17:30:23.258 |

All events occur on 2026-09-09.
The overnight interruption separates the release attempt from the resumed lanes.
This audit launches no processing, browser, build, or timing work.

## Artifact identities and remaining gates

| Artifact | SHA-256 |
| --- | --- |
| Selected ordinary tarball | `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d` |
| Selected native worker | `2c26ec86d5ba668667438713ecbeb9c44fc2c384ab0e05a32e3ff09aa4320650` |
| Scalar Wasm | `cdc795079ed5f8f0d635d9d59b78fdcf6b5f4abced1d836fda4fb5827933189e` |
| Threaded Wasm | `eddd706d6137587124db9575f52ffe756dba040a2b99085e3ae451b1b5dcd4e2` |
| Frozen oracle manifest | `3edcef61c36a69bbfb257e3855ea1edf9d180e15df1ca48d499856839f817831` |
| Compiled TypeScript benchmark adapter | `f3a8020138ad1959dc1acb8bc13391525be00f7a33848943e45797fb84bb1339` |
| TypeScript compiler inputs | `b8243b6aa2853a57867149bb4837bb1f01577591399ded0ea3164ac5dd630b0f` |

The machine summary retains the complete selected provenance record from the anchor report.
Per-case prepared manifests independently bind the same selected source and executable content.
It also preserves TypeScript closure file hashes, browser asset-tree identities, runtime executable hashes, and observed versions.

Build tools are Rust 1.97.0, Node 24.19.0, pnpm 11.13.0, and TypeScript 6.0.3.
Wasm tools are wasm-pack 0.15.0, wasm-bindgen 0.2.121, and wasm-opt 117.
Threaded artifacts use `nightly-2024-08-02`.
Browser versions are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4, with Playwright 1.59.1.
The recorded Linux host has an AMD Ryzen 9 7950X and 24 logical CPUs.

Release remains blocked by the recorded regressions, incorrect bilinear cells, and inconclusive initialization cells.
Historical anchors still have seven confirmed regressions and four inconclusive cells.
The missing release and automatic cases remain required.
The [maximum-output resource check](83-capped-probe.md) fails transport before establishing feasibility.
All capped measurements remain held; no smaller substitute satisfies that gate.
Threaded WebKit and general TypeScript parity remain unavailable as described above.

