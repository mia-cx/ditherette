# S33 measured public progress overhead

Callback overhead passes on all three browsers. Callback-disabled S32/S33 comparison passes on Firefox and WebKit.
Chromium remains inconclusive for cold Lab76 quantize and Lanczos3 resize. No case confirms a >10% regression.

This is bounded evidence for required progress support, not a universal speedup or release-performance pass.
The inherited S32 cold regressions remain S41 release blockers. No retry, retuning, or replacement trial follows these results.

## Sources and scope

Accepted S32 plus shared protocol is `ec640c6dfc48afcc8fe95edcf1d2fd16c8e74579`.
Candidate S33 is `4a75d479d38a92c75e8ff4ed96c916fec3aaf8f4`.
Both callback roles use that same candidate artifact. Delivery conformance checkpoint is `06d9ad0730669dac3008baf848b5eb463689c584`.
The clean measured-source worktree `../v1-s33-measured-source` stays retained.

Read [the declared matrix](74-benchmark.md) for workload construction and [the JSON report](74-benchmark-results.json) for exact values and hashes.
Raw evidence stays in `../v1-s33-bench/target/s33-trial-01`, including all six result folders and prepared snapshots.
Earlier `20507aa0` artifacts are superseded and unmeasured, even though their package bytes match the selected candidate.

Five cold public methods run in two comparisons, with two alternating pairs and three browsers.
Every role/case has 40 pooled single-call samples, from two workers with 20 samples each.
Each worker uses 50 ms warmup and a 10-second cap. There is no native timing or warm-cache timing in S33.

Processor creation/disposal and callback construction/reset are untimed. Each call starts with a fresh empty processor.
Ordinary input and durable output copies, content hashes, preparation, and processing stay timed.
Enabled callback dispatch and its bounded constant-storage observer work also stay timed.
The adapter checks every preflight, warmup, and sampled call outside timing, including output/source stability and valid final completion.
The untimed Process composition clears callback metadata and compares only output semantics.
Passing callback observations do not establish a latency guarantee, event count, or arbitrary application callback cost.

## Execution and exactness audit

All 120 workers started and reaped. Raw timestamps and lifecycle events establish a global maximum of one live worker.
The six runs each contain 20 workers and 400 samples, totaling 2,400 samples and 1,450 warmup calls.
Every expected request/result and alternating role order is present. All 60 actual role pairs are byte- and metadata-exact.
Each worker also matches its target-local frozen reference. All 120 three-way records and 360 component comparisons are exact.
The audit recomputes all 30 medians, pair ratios, and gates from raw results.
It verifies prepared worker hashes and every accepted/candidate prepared asset against its recorded digest.

Quiet clearance starts at 2026-09-08 16:43 UTC, with load 0.61 / 1.73 / 1.91 and owned jobs drained.
The host is Linux x86_64, AMD Ryzen 9 7950X, 24 logical CPUs, kernel `6.12.95+deb13-amd64`.
Browsers are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4, with Node 24.19.0 and Playwright 1.59.1.
All trial contexts are cross-origin isolated. The quiet record does not claim unrelated services were stopped.

## Measured medians

All times are microseconds. C/A means candidate divided by accepted.
For regression, A/C are S32/S33 with callbacks disabled. For callbacks, A/C are the same S33 package with callbacks disabled/enabled.
Pass means the fixed paired rule excludes a >10% regression, including its timer-resolution bound; it does not prove zero overhead.

### Callback-disabled S32 versus S33

| Browser | Workload | A µs | C µs | C/A | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| Chromium | Lab76 quantize | 1312.500 | 1277.500 | 0.9733 | inconclusive |
| Chromium | Lanczos3 resize | 832.500 | 845.000 | 1.0150 | inconclusive |
| Chromium | Bayer4 Oklab fused | 23180.000 | 23132.500 | 0.9980 | pass |
| Chromium | Lanczos2 + FS Process | 1525.000 | 1497.500 | 0.9820 | pass |
| Chromium | Bayer4 Oklab perturb | 22767.500 | 22785.000 | 1.0008 | pass |
| Firefox | Lab76 quantize | 7060.000 | 7010.000 | 0.9929 | pass |
| Firefox | Lanczos3 resize | 5380.000 | 5500.000 | 1.0223 | pass |
| Firefox | Bayer4 Oklab fused | 203620.000 | 203780.000 | 1.0008 | pass |
| Firefox | Lanczos2 + FS Process | 9270.000 | 9320.000 | 1.0054 | pass |
| Firefox | Bayer4 Oklab perturb | 201240.000 | 203750.000 | 1.0125 | pass |
| WebKit | Lab76 quantize | 1460.000 | 1470.000 | 1.0068 | pass |
| WebKit | Lanczos3 resize | 800.000 | 820.000 | 1.0250 | pass |
| WebKit | Bayer4 Oklab fused | 22420.000 | 22420.000 | 1.0000 | pass |
| WebKit | Lanczos2 + FS Process | 1600.000 | 1660.000 | 1.0375 | pass |
| WebKit | Bayer4 Oklab perturb | 22420.000 | 22070.000 | 0.9844 | pass |

### S33 callbacks disabled versus enabled

| Browser | Workload | A µs | C µs | C/A | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| Chromium | Lab76 quantize | 1270.000 | 1235.000 | 0.9724 | pass |
| Chromium | Lanczos3 resize | 835.000 | 880.000 | 1.0539 | pass |
| Chromium | Bayer4 Oklab fused | 23692.500 | 23345.000 | 0.9853 | pass |
| Chromium | Lanczos2 + FS Process | 1527.500 | 1590.000 | 1.0409 | pass |
| Chromium | Bayer4 Oklab perturb | 22880.000 | 23042.500 | 1.0071 | pass |
| Firefox | Lab76 quantize | 7110.000 | 7140.000 | 1.0042 | pass |
| Firefox | Lanczos3 resize | 5480.000 | 5510.000 | 1.0055 | pass |
| Firefox | Bayer4 Oklab fused | 203630.000 | 203150.000 | 0.9976 | pass |
| Firefox | Lanczos2 + FS Process | 9380.000 | 9290.000 | 0.9904 | pass |
| Firefox | Bayer4 Oklab perturb | 201060.000 | 201390.000 | 1.0016 | pass |
| WebKit | Lab76 quantize | 1480.000 | 1480.000 | 1.0000 | pass |
| WebKit | Lanczos3 resize | 840.000 | 850.000 | 1.0119 | pass |
| WebKit | Bayer4 Oklab fused | 22300.000 | 22520.000 | 1.0099 | pass |
| WebKit | Lanczos2 + FS Process | 1580.000 | 1600.000 | 1.0127 | pass |
| WebKit | Bayer4 Oklab perturb | 22080.000 | 21840.000 | 0.9891 | pass |

Chromium Lab76's regression pair ratios are 1.2046 and 0.9492; the pooled ratio is 0.9733.
Chromium Lanczos3's pair ratios are 1.0846 and 0.9795; their spread exceeds 10%, despite a pooled ratio of 1.0150.
Both remain inconclusive, not confirmed regressions or passes. Neither is marked timer-resolution-limited.
The largest observed callback median increase is Chromium Lanczos3 at +5.39%; its paired gate passes.

## Artifacts and validation

Candidate tarball SHA-256 is `f6e62526cc290d8ca1f9fdcb7fcfc9de39790c1982dab7118adb30a4a84173d2`.
Accepted tarball SHA-256 is `8a51e04d08cfdf73bd022ccef1167fed36c74f8267ea1615e19e46df7636fc80`.
Official native/public provenance binds both clean measured revisions. Copied worker/coordinator files remain independent of compiler targets.

| Artifact | Accepted bytes | Candidate bytes | Growth | Accepted gzip bytes | Candidate gzip bytes |
| --- | ---: | ---: | ---: | ---: | ---: |
| Packed package | 342373 | 370620 | 8.25% | already gzip | already gzip |
| Scalar Wasm | 306335 | 339664 | 10.88% | 139079 | 152754 |
| Threads Wasm | 413917 | 445275 | 7.58% | 171470 | 185120 |

Wasm gzip uses Node's default `gzipSync`. The measured public path is scalar; shipping threads size is reported separately.
Each artifact retains its own frozen oracle and manifest. The trusted guard passes at the selected candidate source.

Existing candidate validation passes 8/8 focused installed progress/stage tests and 4/4 broad tests across all three browsers.
Each engine checks 367 frozen Wasm Yliluoma vectors and 734 untimed benchmark-adapter calls.
Interface tests pass 35/35, private Wasm tests 17/17, and factory/staging tests 7/7.
The full native suite also passes. Reporting adds no build, test execution, or measurement.

## Reproduction and delivery

The JSON report records per-run source snapshots, exact medians and pair ratios, result-inventory hashes, and full artifact digests.
To audit, match each request and PID to its start/reap events, compare both outputs and target-local references, then pool raw samples.
Recompute the existing `paired.rs` rule using both role orders and each browser's recorded clock quantum.
Inventory hashes use compact JSON of filename-sorted `{path: filename, bytes, sha256}` records for every direct result-folder file.
JavaScript/Rust timing serialization can differ below nanosecond precision; the audit allows 1e-10 relative timing tolerance.
Output bytes and metadata use exact comparisons throughout.

The merge-preserving rebase onto final S32 `127a0428` leaves delivery head `06d9ad07` and its full tree unchanged.
Report-only commits follow it. File an unmerged non-draft PR on `impl/v1-s32-stages`; keep both measured sources reachable.
Return only the six compiler directories listed in JSON to the coordinator after PR handoff.
Preserve artifacts, copied executables, browser/runtime snapshots, conformance fixtures, all trial evidence, and the measured-source worktree.
The coordinator owns cleanup and S41 carry-forward. This report author performs neither.

