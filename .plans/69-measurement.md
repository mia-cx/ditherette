# S28 measured diffusion selection

Issue #69. The coordinator selects the exact three-row implementation for this slice.
All eight native comparisons pass, with medians 7.798% to 90.919% lower than the literal baseline.
The selection also satisfies the required width-bounded scratch contract.
This is not public optimization or release acceptance. WebKit's public self-pair remains INCONCLUSIVE; retain its gaps for S41.

## Checkpoints and comparison scope

- Literal frozen-copy checkpoint: `91cd93207e1935c53e04cd7b9678cdae742e6f99`.
- Literal native adapter, committed before optimization: `c47419e4a412cbe49edf09b1e268a8f5a2cf9f7e`.
- Three-row candidate: `90909b93987cd25eaf6fe5e1b3b9e915f24dcd9d`.
- Public integration: `e5793547d50d37a2dba5f4403086e15ceee14dae`; candidate adapters: `ffe996bf26062b5ade13d4b554ee9e5cea44a562`.
- Both independently rebuilt measurement roles: clean `058f276d2bf18e1207b74f1a743b5e91f13816e4`.

The measured source joins S27 `44cbe43546e739f3d11f7b0bd08d7453afb83da1`.
The delivery rebase uses `git rebase --rebase-merges origin/impl/v1-s27-bench` onto S27 PR #116 head `c9666288cbe03a9f4dcfb14042cfcbff0fe61ca7`.
Rebased checkpoint `e82df719c2d79b4ce1d6b2f161fd6020871f431c` differs from measured source only in two inherited S27 documentation files.
Mechanical conflict resolutions retain those current parent notes and the measured S28 test files.
Every tracked file under crates, packages, and scripts matches the measured revision byte-for-byte. Subsequent delivery changes concern only these S28 notes.
No processing, frozen spec, image, policy, or landed shared-helper behavior changes during delivery.

Native subjects are `prod:dither-and-quantize:diffusion:full-image-v1` and `candidate:dither-and-quantize:diffusion:three-row-v1`.
Both borrow source and time preparation, work/output allocation, and result destruction. Neither includes the public boundary source copy.
Public roles both run `public:dither-and-quantize:diffusion:package` with the same ring package in primed instances.
That complete-call scope includes public boundary copies. It does not compare a full-image public baseline against the ring.

Three rows start with source coordinates. Recycling retains frozen tap order and every f64 contribution followed by an f32 store.
Work capacity is `width * 3 * sizeof([f32; 3])`, currently 36 bytes per column, with actual allocation capacity accounted.
Prepared palette/matcher/converter, temporary placement conversion, owned source, indices, records, and policy overhead also count toward the public limit.
Height growth adds source/index storage, not work rows. Existing landed palette, matching, conversion, and placement helpers remain in use.
The literal full-image callable remains available as benchmark evidence, not the bounded public path.

## Declared budget and completed execution

The coordinator runs the exclusive trial. This delivery pass only reads its retained files.
Eight recipes use 65×33 RGBA, sixteen palette entries including transparency, all four kernels, and both feedback modes.
Matching rotates through sRGB Euclidean, Oklab Euclidean, and CIEDE2000; scans, all three alpha policies, and placement rotate across recipes.
Adaptive radii one and two, plus zero/fractional/opaque alpha, are represented without a timed Cartesian expansion.

The declaration is two alternating pairs, at most twenty single-call samples per worker, 50 ms warmup, and a 10,000 ms measurement window.
Collectors may stop after at least five samples when that window expires. Twenty is a ceiling, not a guaranteed retained count.
The fixed matrix permits 128 serial workers and at most 2,560 samples. There are no retries or extra candidate revisions.
Raw journals show 128 started and 128 reaped workers, no residual live workers, and maximum live count one per execution journal.
All 128 worker results also report maximum live count one. The coordinator confirms exclusive execution across the complete phase.
Actual retained samples total 2,356. Every report's four verifications per case are exact, with no relaxed output tolerance.

## Native medians

These are the exact numeric medians stored in the report, in nanoseconds. All eight case gates are PASS.
Percentages show lower candidate medians, rounded to three decimals; they describe only these bounded native recipes.

| Kernel / feedback | Literal median ns | Ring median ns | Lower median |
|---|---:|---:|---:|
| Floyd-Steinberg / sRGB bytes | 2687138.5 | 244008.5 | 90.919% |
| Floyd-Steinberg / matching | 25767616.5 | 23636670 | 8.270% |
| Sierra / sRGB bytes | 22785893.5 | 21008938.5 | 7.798% |
| Sierra / matching | 2654063 | 382271 | 85.597% |
| Sierra Lite / sRGB bytes | 26186752.5 | 23563840 | 10.016% |
| Sierra Lite / matching | 22855339 | 20696979 | 9.444% |
| Atkinson / sRGB bytes | 2656578.5 | 293334 | 88.958% |
| Atkinson / matching | 26018794 | 23495578 | 9.698% |

The faster cases use sRGB matching with everywhere placement. Adaptive recipes retain landed placement work and have smaller gains.
No claim isolates ring storage from call-owned converter reuse or extends these measurements to larger images.

## Public self-pair medians

These tables retain the reports' numeric precision in nanoseconds. Role names do not denote distinct public implementations.

### Chromium

| Kernel / feedback | Accepted role median ns | Candidate role median ns | Gate |
|---|---:|---:|---|
| Floyd-Steinberg / sRGB bytes | 500000 | 494999.997317791 | PASS |
| Floyd-Steinberg / matching | 135287499.99962747 | 134864999.99836087 | PASS |
| Sierra / sRGB bytes | 111555000.00156462 | 110947500.00163913 | PASS |
| Sierra / matching | 714999.9998509884 | 737499.9988824129 | PASS |
| Sierra Lite / sRGB bytes | 135169999.9999255 | 134577500.00059605 | PASS |
| Sierra Lite / matching | 110462500.00037253 | 110029999.99932945 | PASS |
| Atkinson / sRGB bytes | 589999.9998509884 | 592499.9993294477 | PASS |
| Atkinson / matching | 134735000.0012666 | 135244999.99918044 | PASS |

### Firefox

| Kernel / feedback | Accepted role median ns | Candidate role median ns | Gate |
|---|---:|---:|---|
| Floyd-Steinberg / sRGB bytes | 3239999.9999999804 | 3199999.999999989 | PASS |
| Floyd-Steinberg / matching | 1199740000.0000002 | 1200359999.9999995 | PASS |
| Sierra / sRGB bytes | 977559999.9999999 | 976030000.0000002 | PASS |
| Sierra / matching | 4780000.000000001 | 4799999.999999997 | PASS |
| Sierra Lite / sRGB bytes | 1208340000.0000002 | 1198450000.0000002 | PASS |
| Sierra Lite / matching | 974360000.0000001 | 976800000.0000001 | PASS |
| Atkinson / sRGB bytes | 3819999.999999993 | 3780000.000000001 | PASS |
| Atkinson / matching | 1211760000.0000002 | 1200380000 | PASS |

### WebKit

| Kernel / feedback | Accepted role median ns | Candidate role median ns | Gate |
|---|---:|---:|---|
| Floyd-Steinberg / sRGB bytes | 560000.0000000023 | 520000.00000001024 | INCONCLUSIVE |
| Floyd-Steinberg / matching | 128409999.99999996 | 128700000.00000018 | PASS |
| Sierra / sRGB bytes | 105420000.00000007 | 105549999.99999994 | PASS |
| Sierra / matching | 760000.0000000051 | 759999.999999998 | PASS |
| Sierra Lite / sRGB bytes | 129309999.99999997 | 128549999.99999997 | PASS |
| Sierra Lite / matching | 105149999.99999997 | 104830000.00000004 | PASS |
| Atkinson / sRGB bytes | 600000.0000000085 | 620000.0000000045 | INCONCLUSIVE |
| Atkinson / matching | 129060000.00000006 | 130090000.00000006 | PASS |

WebKit Floyd-Steinberg/sRGB-bytes pair ratios are 0.8965517241379514 and 0.9999999999999863, producing an inconclusive pair-noise gate.
WebKit Atkinson/sRGB-bytes is resolution-limited; its pair ratios are 0.9999999999999771 and 1.0333333333333263.
Neither is an output mismatch. Keep both baseline gaps for S41. No retry, public speedup claim, or all-gates-pass claim follows.

## Retained evidence and identities

All paths below are relative to `/home/mia/mia-cx/ditherette/.worktrees/v1-s28-bench/target/s28-trial-01/`.
Each `<engine>-results/` directory retains report, prepared descriptor, requests, complete worker results, stderr, verification evidence, and lifecycle journal.
The table hashes each `<engine>-results/report.json` using SHA-256.

| Engine | Overall gate | Samples | Report SHA-256 |
|---|---|---:|---|
| native | PASS | 640 | `f9e762b25601f3c7521410cd6507208bd41ecbae978c636336939d346ef06810` |
| chromium | PASS | 640 | `d3d1beb77d0d594fea04c631e21c132b0b933c910401bbf093b22a61b8778725` |
| firefox | PASS | 436 | `aa0b9ccd22f6970ec3dc0d02bdeea206c0fccfe6b6dd160fcfaa3c796ad92632` |
| webkit | INCONCLUSIVE | 640 | `46f560f00b95ff105f9a0eac40a4c5686a013f2bc968e62397bf33631ff53200` |

Both fresh role directories retain their build provenance and complete source identities. Both native worker hashes are identical:
`e81b9dd5a94f8365027e55dae5610837d56a1130f8bd637a234099dda7d4979f`.
Both public tarballs are `920a5924cb236cc38990f82e92770c090e626632b5c36efe0bdee6ede02f545f`.
Both frozen-only oracle Wasm files are `993d28f1093620ab2ab0a180305fdc8d70d1fa58d3b54db39fecaf8d8a6e71f7`.
The external `accepted-native/ditherette-bench-pair` coordinator is `32ae47300f7d17e8ac9f3dd351501f86592357cfff9c653e048c8a4534918755`.

Prepared descriptor SHA-256 values:

- `native-prepared/prepared.json`: `398f485a93b324994a814399745908571989c1cdf54a256a494c4426806bddd1`.
- `chromium-prepared/pair/prepared.json`: `4999b10e6b3df49abc48f80e774b6a161d9f3faf9d0509fb73ae01c9cb166005`.
- `firefox-prepared/pair/prepared.json`: `7346d51efef880b5ca57f5129ff63463130645729740adf8905c4ac3110e31fd`.
- `webkit-prepared/pair/prepared.json`: `b1ce117892c5a6569e23d028d2471108b9cc90e53378ee5906343b757c68f8e6`.

`readiness.json` and `role-checksums.json` retain the preparation identities and original unstarted status, not current execution status.
Firefox uses the fully revalidated retained S26 runtime closure. `firefox-retained-provenance.json` binds its descriptor, complete inventories, aliases, modes, and digests.
No live profile or unrelated browser process was changed.

## Conformance and delivery validation

The permanent frozen-only Wasm oracle binds source inputs, dependencies, compiler/stdlib, frozen digest, and emitted bytes.
Its disposable context closes before package initialization, warmup, or timers. The oracle is absent from the public tarball.
The retained untimed run passes 423 identified cases per engine in Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
All 360 diffusion cases match both native frozen and target-local Wasm references. The other cases cover 47 quantize, twelve field, and four blue-noise fixtures.
The inherited `blue-noise-oklab-adaptive2` native/Wasm difference remains in diagnostics. It does not affect diffusion exactness.
Fresh public tarballs and oracle Wasm match the conformance bytes; fresh-role browser suites were not rerun.
Full reference reports remain under `../s28-validation-01/oracle-evidence/`, with hashes recorded in `readiness.json`.

Prior validated checks include 10,800 native ring/processor frozen comparisons, 240 zero-strength baseline cases, 120 registered subject combinations, failure recovery, and width/height capacity witnesses.
Seventeen native kernel/processor tests, thirteen protocol/oracle/generator tests, 69 public/private/transport Node tests, scalar/threaded builds, and TypeScript/declaration checks pass.
The later shared-output rejection has 28 focused browser/timing tests passing separately.
The coordinator's full trusted S18 guard passes at `0fb19ad59e68c5227aa2ca128edab147b801500f`, including native/Wasm/threaded isolation, compiler, dependency, syntax, and content checks.
Frozen revision remains `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`; content digest is `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

Delivery rechecks source byte identity, raw report/journal counts, all exact verification statuses, artifact hashes, and `git diff --check`.
The documentation-only rebase invalidates no processing test or measurement result; no new compiler build or test run is claimed.
The coordinator holds compiler caches for S30 assignment and owns cleanup. This pass removes no compiler output and reclaims no disk space.
After cache ownership ends, follow the PRD cleanup rule while preserving immutable snapshots and evidence inside target directories.
