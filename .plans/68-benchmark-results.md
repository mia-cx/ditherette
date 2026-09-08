# S27 exact blue-noise baseline results

Issue #68. All four trial gates pass, with exact verification for every case and both roles.
Retain the literal fixed 32×32 lookup. No new optimization candidate was needed or promoted.
These are independent rebuilds of the same accepted code, not a TypeScript speedup or cross-revision improvement claim.
S41 owns remaining full-call bottlenecks and fresh equivalent TypeScript comparisons before release acceptance.

## Source and scope

Both roles use clean source `44cbe43546e739f3d11f7b0bd08d7453afb83da1`.
The immediate S26 base is `59036e1aef87943e462b4cce6b371e5edd082979`, branch `impl/v1-s26-integration`.
S27 includes validated blue-noise dependency `e4a44b718d49c48902806c0a35726ca1acac06ef`.
The pre-filing `--rebase-merges` onto the fetched S26 base preserves the measured source SHA and complete tracked tree.
This report and plan completion are documentation-only changes after measurement.

The runtime exposes `{ algorithm: 'blue-noise' }` through `perturb` and separable `ditherAndQuantize`.
It reuses landed field conversion, placement, reconstruction, and quantization helpers.
The rank asset retains SHA-256 `bcd93746b99ef8ad678ad425f21e1890b4248050b1ea1b382800d7da977e5943`.
The frozen spec, shared image modules, and freeze policy remain unchanged.
Generation/provenance tooling and the test-only Wasm oracle stay outside the npm package.

## Fixed budget and actual execution

The input is varied 65×33 RGBA8. Strength is 0.7.
Adaptive placement uses threshold 10 and softness 5.
The separable recipe uses YCbCr adaptive radius 1, Oklch hue-arc matching, and palette64 with alpha threshold 0.5.
Five native cases and four complete calls in each browser produce 17 cases.
Two alternating accepted/candidate pairs produce 68 serial workers.
Each worker requests at most 20 single-call samples after 50 ms warmup.
The threshold cap is 250 ms; complete calls have a 10-second cap. There is no application cache.
Browser calls use primed instances, durable public output copies, and untimed exact preflight.
Oracle execution and comparison snapshots remain outside all package timers.

| Runtime | Workers started/reaped | Actual samples | Gate |
|---|---:|---:|---|
| Native | 20/20 | 400 | Pass |
| Chromium | 16/16 | 320 | Pass |
| Firefox | 16/16 | 224 | Pass |
| WebKit | 16/16 | 320 | Pass |
| Total | 68/68 | 1,264 | Pass |

Firefox collects eight samples per worker for both adaptive recipes before the predeclared time cap stops collection.
Every other worker collects 20. No cap, sample budget, noise gate, or exactness requirement was changed.
All four coordinator runs exit successfully. Event logs and every raw result record maximum live benchmark processes of one.
All workers are reaped; no retries or further measurements occur.
Recorded run windows are 2026-09-08 08:32:51–08:32:57 UTC native, 08:33:22–08:34:23 Chromium,
08:39:07–08:41:59 Firefox, and 08:42:13–08:43:26 WebKit.
Per-event host-load observations remain in each `events.jsonl`.

## Observed medians

Values are the reports' pooled single-call medians in milliseconds, rounded here to six decimals.
Roles have identical source and artifact bytes. Their small differences describe this baseline run, not optimization gains.
Every row passes the unchanged per-case regression and pair-noise gates without timer-resolution warnings.

| Runtime | Recipe | Accepted ms | Candidate ms |
|---|---|---:|---:|
| Native | Threshold grid | 0.001391 | 0.001400 |
| Native | sRGB everywhere | 2.561002 | 2.544416 |
| Native | Oklab adaptive2 | 26.352329 | 26.133485 |
| Native | YCbCr everywhere | 2.558326 | 2.549396 |
| Native | YCbCr adaptive1 → palette64 | 26.678570 | 26.580132 |
| Chromium | sRGB everywhere | 14.845000 | 14.880000 |
| Chromium | Oklab adaptive2 | 149.827500 | 148.940000 |
| Chromium | YCbCr everywhere | 15.030000 | 14.815000 |
| Chromium | YCbCr adaptive1 → palette64 | 149.270000 | 148.972500 |
| Firefox | sRGB everywhere | 132.660000 | 132.390000 |
| Firefox | Oklab adaptive2 | 1332.370000 | 1332.860000 |
| Firefox | YCbCr everywhere | 132.700000 | 132.360000 |
| Firefox | YCbCr adaptive1 → palette64 | 1323.180000 | 1324.220000 |
| WebKit | sRGB everywhere | 14.180000 | 14.460000 |
| WebKit | Oklab adaptive2 | 142.980000 | 143.580000 |
| WebKit | YCbCr everywhere | 14.160000 | 14.210000 |
| WebKit | YCbCr adaptive1 → palette64 | 142.700000 | 141.640000 |

Host is `athena-hephaestus`, Linux x86_64 kernel `6.12.95+deb13-amd64`, Ryzen 9 7950X, 24 visible logical CPUs.
Engines are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4, with Node 24.19.0 and Playwright 1.59.1.
Native/scalar compilation uses Rust 1.97.0. Threaded package validation uses its pinned 1.82.0 nightly toolchain.
The oracle uses Rust 1.97.0, wasm-bindgen 0.2.121, release `opt-level=s`, and the approved frozen dependency features.

## Exact reference and retained diagnostics

Native cases use the unchanged native frozen oracle.
Browser cases use that same frozen spec compiled for wasm32 in a separate disposable context.
That context closes before package initialization, warmup, or timed calls.
The oracle independently validates complete source bytes, normalized settings, semantics, and output dimensions through `CaseIdentity`.
Existing asset provenance binds frozen content, source inputs, toolchain, dependencies, and emitted oracle bytes.
Every timed trial requires exact RGBA or indices, palette, transparency, and warning metadata.

Native and Wasm frozen outputs differ at one byte in the Oklab adaptive2 fixture.
Pixel `(8,26)`, RGBA byte 6792, is native red 95 versus Wasm red 94; the other 8,579 bytes agree.
The unchanged frozen Wasm oracle matches the complete actual package frame.
This is shared forward-color floating-point math across compilation targets, not a blue-noise lookup defect.
No fixture replacement, tolerance, expected-output override, or production compensation was applied.
Each browser worker retains both references in `*.reference-diagnostics.json` beside its raw result.
Both roles' untimed `{accepted,candidate}-conformance/{chromium,firefox,webkit}-references.json` retain 63 full reference pairs each.
Earlier stage-localization evidence remains in `target/s27-preparation/`.

## Retained artifacts and commands

All paths below are relative to `.worktrees/v1-s27-bench/target/s27-trial-02/`.
The coordinator is `accepted-native/ditherette-bench-pair`.
Its measurement entrypoint is `run PREPARED_JSON NEW_RESULTS_DIRECTORY` under the coordinator's exclusive quiet phase.
The completed inputs/results are listed below. Do not rerun into these immutable directories.

| Runtime | Prepared input | Results directory |
|---|---|---|
| Native | `native-prepared/prepared.json` | `native-results/` |
| Chromium | `chromium-prepared/pair/prepared.json` | `chromium-results/` |
| Firefox | `firefox-retained-prepared/pair/prepared.json` | `firefox-results/` |
| WebKit | `webkit-prepared/pair/prepared.json` | `webkit-results/` |

Each results directory contains `report.json`, raw `*.result.json`, requests, and `events.jsonl`.
`readiness.json` is the retained pre-measurement handoff, not the final trial status.
`native.json` and `public.json` retain the fixed declarations; the matrix generator is `blue_noise_integration_plan`.
Each role has independent `*-native/build-provenance.json` and `*-public/build-provenance.json`.
`role-checksums.json` records identical accepted/candidate hashes:

| Artifact | SHA-256 |
|---|---|
| Native worker | `fd9f53d46d36895e782c6934911095115c1ebb14daffee08dbc8c1f5bc48bead` |
| Coordinator | `ca45fab5f3ff7a4e3d7f55dd3bc2583c14c4373aedcee02385ac8c41f9050d4a` |
| Package tarball | `32a844782798e87c8e561fd2ca9dc2fd5d653db94ff65648c14702b92f5ecfe1` |
| Frozen-only oracle Wasm | `e3049543fbfefee45608e4893ebd98e2e3d0647f1187e41683d0aad2b91279df` |
| Native report | `44f6424d9a95448e582b20b5fe5caa4b12fdbb7d712565d5a312efdaef5ed63b` |
| Chromium report | `257155f0842f252e40e5f659a42a953f68d6ddb49cb69a9df214b36ce2845ed8` |
| Firefox report | `d63330dfef02550c81eca5357d02288819f6e134375594749e18f5eebaead3a0` |
| WebKit report | `67204e9ad63a26e5fab866003ec04722188ffcc6e4eda7fa84522d1a7ed39a39` |

Firefox preparation uses the retained immutable S26 runtime because the live installation contains another task's profile lock.
The full retained manifest, descriptor, inventory, content hashes, read-only modes, and hardlink aliases passed validation.
The new Firefox snapshot preserves that complete runtime inventory and digest.
`firefox-retained-provenance.json` records the source descriptor and its SHA-256 `448a9ff346f3b69ecb7c1560a1013bfdc0d12f5db32cdec11f4bbfad0e447b2b`.
Its source is `.worktrees/v1-s26-integration/target/s26-trial-01/firefox-prepared/pair/prepared.json`.
The partial failed `firefox-prepared/`, superseded `target/s27-trial-01/`, and all earlier evidence remain intact.

## Validation and delivery boundary

The final measured source passes 38 focused Rust tests and 31 Node protocol/preparation tests.
The historical S26 replay remains explicitly ignored; this slice does not need a new S26 replay.
Both independently built installed roles pass all three engines with 47 quantize and 16 field fixtures,
including the four full S27 recipes and 12 actual `quantize(perturb(...))` compositions per engine.
The implementation checkpoint also passes 110 frozen package vectors and 1,650 compositions per engine, documented in [68-blue-noise.md](68-blue-noise.md).
The post-measurement report/rebase changes no executable source, so these checks remain valid without rebuilding.

The S27 PR remains unmerged against S26. No publication, deployment, release tag, or rollout is enabled.
The coordinator owns issue availability and stack-ledger updates.
After jobs drained, it removed eight compiler profile directories from the returned S24 native/S23 Wasm caches.
That cleanup reclaimed 6,867,780 KiB, about 6.55 GiB. Copied binaries, snapshots, reports, and custom target evidence remain intact.
Rebuild compiler outputs only when review needs them.
