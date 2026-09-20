# S29 literal Yliluoma delivery and candidate results

Issue #70. Retain literal baseline `50cd96d17535ee7f81b1d7a63288751afab50e89`.
Candidate `fdb3921ae1cb7cc3834e42c204d61bb0a63c7cea` remains unselected under the predeclared acceptance rule.
All four overall timing gates are inconclusive. Every measured output is exact.
The conversion-heavy cases show large observed gains, but required timing evidence remains incomplete.
This is not a finding that the candidate is slower, semantically wrong, or a confirmed regression.
No retry, new candidate, or gate adjustment extends this experiment.

## Selected implementation and dependency

The public scalar baseline preserves literal frozen pair/ratio enumeration, strict ties, f32 arithmetic, alpha preparation, and adaptive placement.
It reuses landed Bayer, matching, palette, memory-budget, source/output preflight, and caught-boundary helpers.
Mixture search uses constant scratch. No palette cross-product table, memo, approximation, or generator enters the published package.
The selected path still constructs the existing converter for each target read.
The held candidate changes only that call to reuse PreparedQuantizer's converter and adds its read-only accessor.

Original literal math remains at `9718b16841296c9094e304fe1c4b9d2b6f0e3b85`.
Literal public checkpoint remains `4e0c134d5c846d32af6b253ced0099993122b251`; indexed browser-worker repair is `092f2dd0499ed0c5c3784e6533d726d25c7e142b`.
Candidate production checkpoint is `abe8241`. Its branch, code, and raw evidence remain unchanged.
Both measured roles include S27 source `44cbe43546e739f3d11f7b0bd08d7453afb83da1`.
Delivery rebases with merges preserved onto S27 PR #116, branch `impl/v1-s27-bench`, head `c9666288cbe03a9f4dcfb14042cfcbff0fe61ca7`.
Rebased checkpoint `e3004c8f6f6769c219f1c56acf22f9924c00c626` differs from measured literal source only in S27's two completed-report documents.
The conflict resolution retains the exact previously tested Yliluoma transport regression and the delivered S27 plan.
This report and plan completion add documentation only. Frozen spec, shared image, freeze policy, and runtime bytes remain unchanged.

## Fixed budget and actual execution

Eight explicit cases cover 32x24 sRGB/palette2/size2, 8x8 linear/palette16/size2, and 16x12 Oklab/palette8/size4.
They also cover 8x8 CIELAB/palette16/size4, 4x4 CIEDE2000/palette4/size4, and 8x8 adaptive OKLCH/palette8/size4.
The palette256 control is 1x1/size2; the size16 control is 2x2/palette2.
These are bounded complete calls, not a Cartesian workload or kernel-only timing.
Native calls include validation, preparation, allocation, and destruction while borrowing source storage.
Public calls include boundary copies and durable output, using primed instances with no application cache.
Frozen-oracle execution and immutable comparison copies remain outside package timers.

Two alternating pairs and two roles across native and three browsers declare 128 serial workers.
Every worker collects 20 samples after 50 ms warmup, within its declared 10-second measurement cap.
Actual totals are 128 workers started/reaped, 2,560 samples, and maximum live benchmark processes of one.
Each runtime contributes 32 workers and 640 samples. All four overall gates are `inconclusive`.
The coordinator enforces the exclusive quiet phase. No builds, tests, implementation, cleanup, or extra workers overlap it.
All workers exit before implementation resumes. No outputs or failures are omitted.

## All observed results

Pooled medians are milliseconds, rounded to six decimals. Ratios are candidate/literal; lower values are faster.
Every row passes exact verification, with no resolution-limited flag.
The unchanged pair-noise gate leaves native CIEDE2000, Chromium palette256, Firefox size16, and WebKit palette2/size2 plus size16 inconclusive.
An isolated noisy pair above 1.10 is not a confirmed regression.
Full-precision ratios and verification statuses are retained in [70-results.json](70-results.json).
Complete raw verifier reports, samples, requests, output images, events, and immutable snapshots remain at the evidence paths below.

### Native

| Case | Literal ms | Candidate ms | Ratio | Pair ratios | Gate |
|---|---:|---:|---:|---|---|
| srgb-p2-b2 | 0.984427 | 0.056151 | 0.057039 | 0.056740, 0.057696 | pass |
| linear-p16-b2 | 0.209813 | 0.137732 | 0.656447 | 0.620954, 0.673290 | pass |
| oklab-p8-b4 | 0.596450 | 0.344295 | 0.577240 | 0.574361, 0.581761 | pass |
| cielab-p16-b4 | 0.490733 | 0.411782 | 0.839116 | 0.834015, 0.844767 | pass |
| ciede2000-p4-b4 | 0.224354 | 0.209773 | 0.935009 | 0.916995, 1.037252 | inconclusive |
| oklch-p8-b4-adaptive | 0.970565 | 0.892515 | 0.919582 | 0.907277, 0.923052 | pass |
| srgb-p256-b2-control | 0.475338 | 0.474703 | 0.998665 | 0.986544, 1.011329 | pass |
| srgb-p2-b16-control | 0.014630 | 0.009931 | 0.678787 | 0.678377, 0.679036 | pass |

### Chromium

| Case | Literal ms | Candidate ms | Ratio | Pair ratios | Gate |
|---|---:|---:|---:|---|---|
| srgb-p2-b2 | 5.447500 | 0.115000 | 0.021111 | 0.020711, 0.021306 | pass |
| linear-p16-b2 | 0.720000 | 0.260000 | 0.361111 | 0.357639, 0.363636 | pass |
| oklab-p8-b4 | 1.965000 | 0.660000 | 0.335878 | 0.341280, 0.331633 | pass |
| cielab-p16-b4 | 1.237500 | 0.780000 | 0.630303 | 0.616142, 0.641975 | pass |
| ciede2000-p4-b4 | 0.360000 | 0.245000 | 0.680556 | 0.700000, 0.655629 | pass |
| oklch-p8-b4-adaptive | 4.830000 | 4.387500 | 0.908385 | 0.893114, 0.929208 | pass |
| srgb-p256-b2-control | 0.975000 | 0.980000 | 1.005128 | 0.964467, 1.120104 | inconclusive |
| srgb-p2-b16-control | 0.055000 | 0.025000 | 0.454545 | 0.454545, 0.454545 | pass |

### Firefox

| Case | Literal ms | Candidate ms | Ratio | Pair ratios | Gate |
|---|---:|---:|---:|---|---|
| srgb-p2-b2 | 48.150000 | 0.740000 | 0.015369 | 0.015233, 0.015426 | pass |
| linear-p16-b2 | 5.680000 | 1.760000 | 0.309859 | 0.304196, 0.321555 | pass |
| oklab-p8-b4 | 16.340000 | 4.420000 | 0.270502 | 0.271892, 0.268949 | pass |
| cielab-p16-b4 | 9.350000 | 5.360000 | 0.573262 | 0.571429, 0.573876 | pass |
| ciede2000-p4-b4 | 3.120000 | 2.120000 | 0.679487 | 0.666667, 0.692557 | pass |
| oklch-p8-b4-adaptive | 42.630000 | 38.570000 | 0.904762 | 0.908019, 0.902171 | pass |
| srgb-p256-b2-control | 6.340000 | 6.190000 | 0.976341 | 0.985782, 0.965409 | pass |
| srgb-p2-b16-control | 0.440000 | 0.180000 | 0.409091 | 0.391304, 0.454545 | inconclusive |

### Webkit

| Case | Literal ms | Candidate ms | Ratio | Pair ratios | Gate |
|---|---:|---:|---:|---|---|
| srgb-p2-b2 | 5.180000 | 0.120000 | 0.023166 | 0.023077, 0.027027 | inconclusive |
| linear-p16-b2 | 0.680000 | 0.280000 | 0.411765 | 0.441176, 0.411765 | pass |
| oklab-p8-b4 | 1.960000 | 0.680000 | 0.346939 | 0.346939, 0.343434 | pass |
| cielab-p16-b4 | 1.260000 | 0.840000 | 0.666667 | 0.674603, 0.661417 | pass |
| ciede2000-p4-b4 | 0.360000 | 0.280000 | 0.777778 | 0.722222, 0.777778 | pass |
| oklch-p8-b4-adaptive | 4.740000 | 4.280000 | 0.902954 | 0.901053, 0.900844 | pass |
| srgb-p256-b2-control | 1.060000 | 1.040000 | 0.981132 | 0.962264, 1.000000 | pass |
| srgb-p2-b16-control | 0.060000 | 0.040000 | 0.666667 | 0.666667, 0.500000 | inconclusive |

## Validation and target diagnostics

Both measured roles pass 25 focused native tests, 12 private ABI tests, and 25 public interface/type tests.
Both scalar/threaded builds and full trusted S18 freeze/isolation guards pass.
Benchmark assets, transport classification, permanent-oracle identity, and typed Yliluoma adapters pass their focused checks.
Each installed package passes 367 permanently identified frozen-Wasm cases and 734 untimed actual adapter calls in each engine.
The same runs preserve 110 field vectors and 1,650 compositions per engine.
They cover all 15 matching policies, all matrix sizes, alpha modes, placements, source/result ownership, exact/one-under budgets, failures, reentry, and disposal.

Seven native/Wasm target diagnostics remain exactly cases 236, 248, 251, 254, 257, 260, and 263.
These matte CIELAB/CIEDE2000 cases expose inherited target-local float rounding in exhaustive mixture ties.
Production equals the unchanged frozen oracle on each target. This does not assert universal native/Wasm byte parity.
The native and independently generated Wasm fixtures remain intact. No tolerance or arithmetic repair hides the differences.

Fresh preparation genuinely rebuilds each role through the shared-cache invalidation helpers.
Both freshly prepared package tarballs exactly equal the tarballs already tested in all three engines.
Fresh-role conformance is not rerun; complete tarball equality binds those results to the prepared packages.
Each role independently validates clean source inventory, tools, dependencies, frozen profile, and output digests.
Native executable build-info verifies its embedded clean revision and complete digest.

Literal oracle Wasm SHA-256 is `2accd975d637e53287de4d3e69210bd92c9eb75ee18574dec562b4f71155f786`.
Candidate oracle Wasm SHA-256 is `2c18e76b00fe6251fb3e053274c5db50caf6d00e516a92904c3077df84f945bb`.
Only custom `name` section Rust symbol hashes differ. All other sections, including complete executable code and data, are byte-identical.
Neither oracle is stripped, edited, or substituted. Independent manifests validate their actual bytes.
Literal manifest SHA-256 is `2309c6210966b5c7be3ef0a1646c00ac506ccbc7cc49ec590a0c77cca0b42e9a`.
Candidate manifest SHA-256 is `ad733308c91920d0533bea9d9938e6dec6b9246f224235bddc0920e9d638b2c2`.
The retained `validation-proof.json` and `validate-preparation.mjs` record and reproduce section equality.

| Artifact | Literal bytes | Candidate bytes |
|---|---:|---:|
| Scalar Wasm | 243,911 | 243,912 |
| Threaded Wasm | 333,042 | 333,034 |

Literal tarball SHA-256 is `abc36ca5fb79e3ac27189790df0bb268dc401355dca82375ea141efa6bcc253c`.
Candidate tarball SHA-256 is `99964391219a2ec954998fa8a6e0a05d76d72676339213d545ccc47e5a90b9f4`.
Native executable SHA-256 values are `885e2b8746fc3d3090ad4c27857890d9243fc58f32f839b16e0ff2c3ebb263ba` and `1312f407cd3b4df5e33d5f5b122a49ab1767a3a5082a1122d93e29222d224d63`.

## Retained evidence and held work

Raw trial root is `/home/mia/mia-cx/ditherette/.worktrees/v1-s29-converter/target/s29-trial-01/`.
Each `{native,chromium,firefox,webkit}-results/` contains `report.json`, `events.jsonl`, all worker samples, and full verification.
Prepared snapshots are `native-prepared/prepared.json` and `{chromium,firefox,webkit}-prepared/pair/prepared.json`.
Runtime versions are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Firefox uses the validated retained immutable closure, without touching the live profile.
WebKit snapshots preserve hardlinked library aliases.

| Raw report | SHA-256 |
|---|---|
| native-results/report.json | `4fbe400051bf72c25370c2a4f3aa81896a2e97f82bd9e8921c4f0e49c981081f` |
| chromium-results/report.json | `576feb5fcda2a14100e0fc93ec6d93a5cb173d102bfd15223a62dfac9bb1c0fe` |
| firefox-results/report.json | `e1830ca7626f06e5ac10560cd6b73c8cd3a5e6b67a6238ef292df6d84c329aeb` |
| webkit-results/report.json | `6d5a06155ee38953202161eebce3bdc1a93db1aaf0eac7b1e054566385d3bbe9` |

S41 retains release-level performance acceptance, equivalent fresh TypeScript comparisons where applicable, and unresolved complete-call bottlenecks.
No faithful TypeScript Yliluoma adapter is registered. These measurements do not establish a TypeScript speedup.
Exact mixture precomputation/memoization and broader large-image acceptance remain held, not silently completed by converter reuse.
The slice delivers a bounded, exact scalar baseline with an unsuccessful selection gate, not an optimized-performance claim.
The coordinator owns issue availability, ledger updates, later joins, selection, and completed-PR compiler cleanup.
Preserve source and all raw/immutable evidence during cleanup. This delivery does not merge, release, publish, or deploy.
