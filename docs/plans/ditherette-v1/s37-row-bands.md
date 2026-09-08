# S37 Yliluoma row-band measurements

The cold calls support exact row bands in four measured recipe classes. Two workers win every cold case in both engines. Four workers improve the medium case further. Several warm final-output controls remain inconclusive; S41 still owns those evidence gaps.

## Evidence

Both roles use the same threaded package from source `5d16c5682f354fecd75ca7f761802d9e2ea75ab5`. Accepted forces scalar mixing; candidate changes only the mixing row policy. Calls run through the ordinary package in a blocking-capable, cross-origin-isolated host Worker. Pool initialization and policy setup remain outside method timing. Input/output copies, hashing, preparation, and the complete method remain inside it.

The fixed matrix uses two alternating pairs, 20 requested single-call samples per worker, 50 ms warmup, and a 10-second worker measurement budget. After at least five samples, a worker stops when its accumulated measured time reaches that budget. Twenty samples are the requested maximum, not a mandatory completion condition. S37 contributes 12 cases and 96 browser workers across both engines. The full sweeps completed and reaped their 200 workers per engine. Their overall `incorrect` status includes other slices' retained frozen-reference drift; it does not describe S37's outputs.

| Engine | Report under `.worktrees/v1-s35-37-bench/target/` | Report SHA-256 |
|---|---|---|
| Chromium 147.0.7727.15 | `rows-trial-02/chromium-results/report.json` | `160255130439e20dbbcd46a224f4911bed8fc5c10cc3840f46627a09e2148642` |
| Firefox 148.0.2 | `rows-trial-03/firefox-results/report.json` | `77af173eb9b0c2203d8ad43a75181a68e90d57cf2a8388c1e7b1784603594e73` |

Every S37 case has four exact verification records per engine. All 96 records retain zero differing indices and matching metadata against both frozen reference and accepted scalar output. No S37 reference tolerance changed.

S36's compact `rows-trial-03/combined-analysis.json` records 960 achieved Chromium samples and 864 Firefox samples for S37. Chromium reaches 20 samples in every S37 worker. Firefox's capped cases are below; each pair lists scalar / candidate samples. All other Firefox S37 workers reach 20, including every warm control.

| Firefox case | Pair 0 samples | Pair 1 samples |
|---|---|---|
| Medium, two/4 | 5 / 9 | 5 / 9 |
| Process, two/4 | 19 / 20 | 19 / 20 |
| Medium, four/16 | 5 / 15 | 5 / 15 |
| Process, four/16 | 19 / 20 | 19 / 20 |

## Cold complete calls

Times are accepted → candidate medians in milliseconds. Ratio means candidate / accepted. A `pass` gate permits a non-regression; a ratio below one establishes the observed speed improvement.

| Case | Requested workers / band height | Chromium ms | Ratio / gate | Firefox ms | Ratio / gate |
|---|---|---:|---|---:|---|
| Tiny, 9×7, p2, Bayer2, sRGB everywhere | 2 / 4 | 0.640 → 0.510 | 0.797 / pass | 4.410 → 2.780 | 0.630 / pass |
| Small, 33×25, p4, Bayer4, sRGB everywhere | 2 / 4 | 7.100 → 4.938 | 0.695 / pass | 57.140 → 37.510 | 0.656 / pass |
| Medium, 65×49, p8, Bayer4, sRGB adaptive | 2 / 4 | 239.290 → 140.088 | 0.585 / pass | 2044.240 → 1176.570 | 0.576 / pass |
| Process, 65×49 → 33×25, p4, Bayer4, OKLCH circular hue adaptive | 2 / 4 | 62.055 → 40.762 | 0.657 / pass | 528.680 → 342.090 | 0.647 / pass |
| Tiny | 4 / 16 | 0.677 → 0.655 | 0.967 / inconclusive | 4.430 → 4.480 | 1.011 / pass |
| Small | 4 / 16 | 7.175 → 4.747 | 0.662 / pass | 57.170 → 37.330 | 0.653 / pass |
| Medium | 4 / 16 | 243.250 → 80.848 | 0.332 / pass | 2039.150 → 681.650 | 0.334 / pass |
| Process | 4 / 16 | 62.240 → 40.960 | 0.658 / pass | 529.420 → 340.790 | 0.644 / pass |

At height 16, tiny has only one band; small and Process have two. Requested workers do not imply four active workers. Medium has four bands. Adaptive fixtures use radius 2, threshold 5, and softness 10. Process uses centered nearest resize. Every fixture uses premultiplied alpha.

## Warm controls and remaining gates

Warm controls prime the same complete call before each sample. They use the four-worker/height-16 policy, but the measured final-output cache hit skips the mixing kernel. Their timing cannot establish a row-band speedup.

| Control | Chromium ratio / gate | Chromium pair ratios | Firefox ratio / gate | Firefox pair ratios |
|---|---|---|---|---|
| Tiny | 1.048 / inconclusive | 0.864, 1.211 | 1.125 / inconclusive | 1.125, 1.000 |
| Small | 0.824 / pass | 0.875, 0.857 | 1.032 / inconclusive, resolution limited | 1.067, 1.000 |
| Medium | 1.016 / inconclusive, resolution limited | 1.065, 0.984 | 0.957 / pass | 0.929, 1.000 |
| Process | 1.000 / pass | 1.016, 1.017 | 1.029 / inconclusive, resolution limited | 1.044, 1.000 |

The tiny Firefox aggregate exceeds 10%, but its paired result remains inconclusive. This is an unresolved control, not a confirmed regression or a passed gate. Automatic selector overhead is new code after the measured artifact; these trials do not measure that overhead. S41 must retain the warm-control gaps and validate the final public artifact. WebKit's threaded cleanup gate remains open, and this evidence includes no WebKit required-thread trial.

## Conservative selection

Select only premultiplied-alpha calls in the following recipe classes. Palette cardinality counts visible entries among the retained first 256 entries. Palette bytes, source content, and seeds never enter selection.

| Recipe class | Minimum output width × height | Selected policy |
|---|---|---|
| p2, Bayer2, sRGB Euclidean, everywhere | 9×7 | 2 workers / 4 rows |
| p4, Bayer4, sRGB Euclidean, everywhere | 33×25 | 2 workers / 4 rows |
| p8, Bayer4, sRGB Euclidean, adaptive | 65×49 | 4 workers / 16 rows with at least four pool workers; otherwise 2 / 4 |
| Process with nearest resize, p4, Bayer4, OKLCH circular hue, adaptive | 33×25 | 2 workers / 4 rows |

These width and height lower bounds are conservative selected thresholds, not proven universal crossover points. Applying them to larger images or other adaptive masks is an inference from the measured class. Unmeasured alpha policies, palette cardinalities, matching recipes, matrix sizes, smaller/skinny outputs, and standalone OKLCH adaptive calls stay scalar. A one-worker pool stays scalar. A three-worker pool selects the measured two-worker configuration. Explicit development overrides remain separate from automatic selection.

Reject four/16 for tiny because Chromium is inconclusive and Firefox is slower. Keep two/4 for small and Process because four/16 offers no consistent material improvement and uses only two bands. Keep the previous S29 converter experiment unselected; the literal landed converter and ordered mixture arithmetic remain unchanged.

Row scheduling retains shared palette preparation and full-source adaptive reads. Its capacity preflight includes assignment metadata and one live temporary converter per active worker. Joined callbacks stay on the caller, failed calls publish no entries, and content identities exclude execution policy.

## Native selection validation

The joined automatic selector passes complete-call tests in native pools of one, two, three, and four workers. Tests compare direct medium and nearest/OKLCH Process results with frozen indices and metadata. They check the measured joined-progress configurations, independent stage overrides, explicit scalar overrides, and unchanged cache identity. Existing budget and callback-failure checks still pass.

Release validation passes 16 tests with `bench-subjects`, eight with `threads,bench-subjects`, and three focused selector tests in each build. The frozen spec, shared image infrastructure, and guard have no source changes. These are correctness checks; final installed-package validation and S41's warm-control evidence remain separate.

## Final installed-package disposition

The publication branch joins S36 PR 127 head `b542bd94a5dbc724de73815ae0008a22985147fd` and rebases onto its published branch.
All production and package build inputs are byte-identical to combined tested source `dc81818a`.
The ordinary tarball SHA-256 is `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.

The trusted frozen guard passes. The untimed automatic host fixture passes 23 checks, including nine cases per Chromium/Firefox engine.
Medium mixing and nearest/OKLCH Process match ordinary scalar output exactly, including public metadata.
The fixture verifies progress and worker disposal; both ordinary binaries exclude the developer policy export.
Exact invocation and completed browser cleanup live in `.plans/77-79-auto-host.md`.

Installed correctness is complete. S41 still owns warm-control uncertainty, automatic-policy timing follow-up, unmeasured classes, and WebKit cleanup.
No new measurement, approximation, frozen expectation, or processing change accompanies this PR.
