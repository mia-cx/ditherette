# S32 measured image-stage reuse

All four runtime performance gates are **regression**. Required image-stage caching is functionally delivered, but this is not a performance promotion.
Seven confirmed cold regressions remain S41 release blockers. No tuning candidate, retry, or replacement trial was run.

## Sources and retained evidence

- Accepted preparation-only S31: `d51a70a2daf054357d16ea66b235da3733c02888`.
- Candidate image-stage S32: `d638f87c3a16824ef52964bbb611ef91b173f2ee`.
- Corrected S31 parent: `59b1fe3acdbeae27bbb8ab780b46d2b6b9d67a76`.
- Final installed conformance checkpoint: `59b5a004c98c4cff255b30f51025d8ec8143786d`. Later changes are tests and plans, not measured runtime.

Read [the declared matrix](73-benchmark.md) for exact inputs, settings, and priming methods.
The [machine-readable report](73-benchmark-results.json) retains exact medians, both pair ratios, gates, evidence hashes, and artifact sizes.
Raw evidence stays in `target/s32-trial-01` in the S32 stages worktree, including prepared copies, four result folders, and `quiet-start.md`.
Accepted native/public artifacts remain in the S32 benchmark worktree. Candidate artifacts remain in the S32 stages worktree.
Their absolute paths and SHA-256 digests are in the JSON report.

## Execution and exactness audit

The declared bound is four workloads × cold/warm × two role pairs × four runtimes × two roles.
All 128 workers started and reaped, with 32 per runtime. Raw lifecycle events establish a maximum of one live worker across the sequential runs.
Every worker also records `max_live_benchmark_processes = 1`. Each has 20 valid single-call samples, totaling 2,560.
Warmup totals 4,555 calls: native 3,243; Chromium 658; Firefox 164; WebKit 490.
Both pair orders are present for every case. No extra result, missing worker, zero sample, or incomplete case was found.

All 64 actual accepted/candidate pairs are byte- and metadata-exact. Each worker matches its own target-local frozen reference.
The coordinator's 128 three-way records contain 384 exact component comparisons; every record has frozen status and no mismatch.
The audit independently recomputed pooled medians, pair ratios, and all 32 case gates from raw worker results.
Successful protocol execution checks every warm prime outside timing and observes every measured output before teardown.
This does not assert equality between native and browser floating-point targets.

The quiet record starts at 2026-09-08 15:57 UTC, with host load 0.40 / 1.66 / 1.65 and owned jobs drained.
The recorded host is Linux x86_64, AMD Ryzen 9 7950X, 24 logical CPUs, kernel `6.12.95+deb13-amd64`.
Browsers are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4, using Node 24.19.0 and Playwright 1.59.1.
Browser trial contexts are cross-origin isolated. Observed clock quanta are approximately 5 µs for Chromium and 20 µs for Firefox/WebKit.
No inference about unrelated host-service quiescence is added beyond the coordinator's record.

## Timing scope

Cold calls start with a fresh empty processor. Warm calls get a fresh processor plus the declared prime before every call.
The prime uses identical input bytes and the declared same-call or cross-method method; partial stages never accumulate final-output hits across samples.
Priming, verification, processor creation/disposal, and native owned-result destruction are untimed.
Ordinary input copies, source hashes, downstream materialized-content hashes, preparation, and durable result copies stay inside method timers.
Thus cold hashing and retention overhead is visible, not moved into setup.
Historical S31 once-per-worker changed-source priming is a different lifecycle and is not used here.

## Measured medians

Medians pool 40 samples per role/case. C/A is candidate divided by accepted; below 1 is a lower observed median.
A pass excludes a confirmed >10% regression under the paired rule; it does not promise a statistically established speedup.
All overall runtime gates remain regression even where individual warm cases pass.

### Native

| Workload | State | Accepted µs | Candidate µs | C/A | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| Lab76 quantize | cold | 500.467 | 519.423 | 1.0379 | pass |
| Lab76 quantize | warm | 492.279 | 12.995 | 0.0264 | inconclusive |
| Lanczos3 resize | cold | 169.808 | 189.852 | 1.1180 | regression |
| Lanczos3 resize | warm | 142.033 | 20.869 | 0.1469 | pass |
| Bayer4 Oklab fused | cold | 4283.921 | 4174.409 | 0.9744 | pass |
| Bayer4 Oklab fused | warm | 4184.738 | 179.339 | 0.0429 | pass |
| Lanczos2 + FS Process | cold | 493.618 | 520.433 | 1.0543 | pass |
| Lanczos2 + FS Process | warm | 472.861 | 394.586 | 0.8345 | pass |

### Chromium

| Workload | State | Accepted µs | Candidate µs | C/A | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| Lab76 quantize | cold | 1120.000 | 1232.500 | 1.1004 | inconclusive |
| Lab76 quantize | warm | 1052.500 | 197.500 | 0.1876 | pass |
| Lanczos3 resize | cold | 515.000 | 837.500 | 1.6262 | regression |
| Lanczos3 resize | warm | 370.000 | 265.000 | 0.7162 | pass |
| Bayer4 Oklab fused | cold | 23070.000 | 23132.500 | 1.0027 | pass |
| Bayer4 Oklab fused | warm | 23000.000 | 500.000 | 0.0217 | pass |
| Lanczos2 + FS Process | cold | 1222.500 | 1540.000 | 1.2597 | regression |
| Lanczos2 + FS Process | warm | 1112.500 | 1060.000 | 0.9528 | pass |

### Firefox

| Workload | State | Accepted µs | Candidate µs | C/A | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| Lab76 quantize | cold | 6480.000 | 7150.000 | 1.1034 | inconclusive |
| Lab76 quantize | warm | 6430.000 | 600.000 | 0.0933 | pass |
| Lanczos3 resize | cold | 3740.000 | 5480.000 | 1.4652 | regression |
| Lanczos3 resize | warm | 3440.000 | 1320.000 | 0.3837 | pass |
| Bayer4 Oklab fused | cold | 203340.000 | 204410.000 | 1.0053 | pass |
| Bayer4 Oklab fused | warm | 203910.000 | 2650.000 | 0.0130 | pass |
| Lanczos2 + FS Process | cold | 7630.000 | 9270.000 | 1.2149 | regression |
| Lanczos2 + FS Process | warm | 7240.000 | 6250.000 | 0.8633 | pass |

### WebKit

| Workload | State | Accepted µs | Candidate µs | C/A | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| Lab76 quantize | cold | 1360.000 | 1460.000 | 1.0735 | inconclusive |
| Lab76 quantize | warm | 1340.000 | 200.000 | 0.1493 | inconclusive |
| Lanczos3 resize | cold | 500.000 | 840.000 | 1.6800 | regression |
| Lanczos3 resize | warm | 360.000 | 260.000 | 0.7222 | pass |
| Bayer4 Oklab fused | cold | 22630.000 | 23260.000 | 1.0278 | pass |
| Bayer4 Oklab fused | warm | 22610.000 | 630.000 | 0.0279 | pass |
| Lanczos2 + FS Process | cold | 1280.000 | 1600.000 | 1.2500 | regression |
| Lanczos2 + FS Process | warm | 1100.000 | 1140.000 | 1.0364 | pass |

## S41 release blockers and inconclusive cases

Cold Lanczos3 resize confirms regressions on native (+11.80%), Chromium (+62.62%), Firefox (+46.52%), and WebKit (+68.00%).
Cold Process confirms regressions on Chromium (+25.97%), Firefox (+21.49%), and WebKit (+25.00%).
Each blocker crosses 10% in both alternating pairs and survives the browser clock-resolution bound where applicable.
Keep all seven as S41 release blockers. S32 implements mandatory cache semantics; those semantics do not waive the performance release gate.

Five cases remain inconclusive:

- Native warm Lab76 and WebKit warm Lab76 have pair-ratio spread above 10%, despite much lower observed medians.
- Chromium cold Lab76 has mixed pair thresholds, with a pooled +10.04% median.
- Firefox cold Lab76 has a nominal +10.34% median, but clock resolution prevents confirmation.
- WebKit cold Lab76 cannot establish the pass threshold within its clock resolution.

Warm resize and the partial perturb-stage case show lower observed medians across all runtimes.
Warm Process is lower on native, Chromium, and Firefox, but WebKit is +3.64%; its no-regression gate still passes.
No new optimization loop or remeasurement is authorized by these observations. The coordinator owns the S41 issue handoff.

## Package size and conformance

| Artifact | Accepted bytes | Candidate bytes | Growth | Accepted gzip bytes | Candidate gzip bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Packed package | 327828 | 342373 | 4.44% | already gzip | already gzip |
| Scalar Wasm | 290207 | 306335 | 5.56% | 131682 | 139079 |
| Threads Wasm | 396244 | 413917 | 4.46% | 164161 | 171470 |

Gzip sizes use Node's default `gzipSync` on the complete Wasm file; the tarball is already gzip-compressed.
Both oracle Wasm files are 785,376 bytes. Their complete hashes differ, so each artifact's own recorded frozen oracle remains authoritative.
The frozen specification digest remains `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

Accepted and candidate tarballs pass installed three-engine conformance, including stage ownership and all five methods.
Each engine passes 367 frozen Wasm Yliluoma vectors and 734 untimed actual benchmark-adapter calls.
Candidate checkpoint `59b5a004` also records 34 interface tests and 16 private Wasm ABI tests passing.
Public behavior proves durable copies, mutation isolation, and recovery, not invisible cache-hit or publication state.
Separate native fixtures establish image hits, shared capacity pressure, and success-only publication.

## Delivery and cleanup

File this required functionality as an unmerged PR stacked on `impl/v1-s31-preparation`; preserve measured-source ancestry.
The rebase with `--rebase-merges` onto `59b1fe3a` preserved the full tree and head `59b5a004` before report changes.
All copied worker/coordinator binaries, package artifacts, frozen oracles, prepared snapshots, and result files remain outside compiler targets.
After PR handoff, return only the three compiler targets in each of `v1-s32-stages` and `v1-s32-bench` to the coordinator.
Preserve all evidence. The coordinator performs audited cleanup; this report performs no deletion, build, or measurement.
