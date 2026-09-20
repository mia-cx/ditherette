# S25 delivery and measurement decision

Issue #66. Decision: retain the all-mode baseline. The dispatch candidate is rejected.

## Delivery checklist

- [x] Preserve the validated fifteen-mode implementation and recorded artifact identities.
- [x] Review the completed paired reports and record the selection without reusing samples.
- [x] Revalidate the rebased delivery and prepare its unmerged PR against `impl/v1-s24-bench`.

## Selected source

The selected implementation is `0085972a05a3dbdbbef6d47351d6e37bdd8625d2`.
Its literal native baseline is `17b2bb003a48fe3151dc7277e88f0dd55d8bf87b`.
Benchmark registration finishes at `009b9e37f5a306679bb6eb7a8a68819936b05521`.
Fresh artifact preparation uses `fb8878b9ce39f605df23e55b80f664b3e6e969f2`.
That revision adds build-freshness checks, not a production optimization.

Delivery checkpoint `bf1b887ca90409b909b742e5eb78f97ab00f5bbd` rebases this source onto S24 PR #113.
The base is `884a8868f52e82ae579835eee173c6385a9c9ca2`.
All `crates/`, `packages/`, and `scripts/` bytes match the selected prepared source at that checkpoint.
The 991-entry source inventory differs only in two historical S24 plan documents.
All 34 retained installed-package files match their recorded byte lengths and SHA-256 hashes.

The selected matcher keeps stable palette order and per-entry `distance_score` dispatch.
It reuses landed color conversion arithmetic and S24 allocation, alpha, warning, and publication behavior.
Missing metric functions began as literal frozen copies. Cylindrical integration adds frozen gray/hue normalization.
Existing resize kernels and shared helpers remain unchanged. Frozen spec, image, and policy bytes remain unchanged.
See [implementation evidence](66-specialized-matching.md) and [registration evidence](66-benchmark-integration.md) for their checks.

Rejected optimization `230046ff62588e8385df2562b21441db6839b78e` runs in candidate revision `4b7172969bf9f58d804112c3e41b70f015060a4d`.
Its dispatch-hoisting change is absent from delivery.

## Evidence and bounded work

The retained evidence root is:

`/home/mia/mia-cx/ditherette/.worktrees/v1-s25-dispatch/target/s25-trial-02`

Read `completion-audit.json` for counts and cleanup, and `artifact-audit.json` for prepared identities.
Read `results-{existing,new}-{native,chromium,firefox,webkit}/report.json` for each paired decision.
Those directories retain requests, actual verification outputs, raw samples, stderr, transport responses, and child events.
`prepared-*/prepared.json` or `prepared-*/pair/prepared.json` bind source, settings, files, runtime closures, and executable hashes.
`accepted-{native,public}/build-provenance.json` and corresponding candidate files bind the fresh artifacts.

Trial 02 starts and reaps 276 workers, with maximum concurrency one.
It records 5,133 timing samples. Every verified output is exact, including bytes and metadata.
Native checks verify deterministic endpoints, not every timed output. Public checks inspect every durable output outside timers.
The completion audit records no remaining processes.

Trial 01 starts and reaps two workers before the native revision check rejects stale candidate code.
Its accepted worker records 20 samples. None contribute to Trial 02 comparisons.
The combined worker count is 278. No retry or further S25 measurement follows this decision.
The earlier unmeasured public-build defect and both fixes remain in [build freshness evidence](66-build-freshness.md).

All complete-call cases use the same varied-RGBA 128×96 source, 64-entry palette, and preserve-alpha threshold 0.5.
Each case has two alternating AB/BA pairs, at most 20 samples, and 50 ms warmup.
The measurement cap is 250 ms, except complete CIEDE2000 calls use 10,000 ms.
That predeclared exception permits the minimum five samples; it is not a performance claim.
The matrix has fifteen complete-call modes, two native cylindrical conversions, and seven native metric families.

## Paired results

| Group | Cases | Samples | Decisions |
| --- | ---: | ---: | --- |
| Existing native | 5 | 400 | 5 pass |
| Existing Chromium | 5 | 400 | 4 pass, 1 inconclusive |
| Existing Firefox | 5 | 384 | 5 pass |
| Existing WebKit | 5 | 400 | 5 pass |
| New native | 19 | 1,520 | 14 pass, 4 inconclusive, 1 regression |
| New Chromium | 10 | 800 | 9 pass, 1 inconclusive |
| New Firefox | 10 | 429 | 10 pass |
| New WebKit | 10 | 800 | 10 pass |

The required Euclidean-score native control regresses by 24.4%.
Its accepted median is 18,626 ns; its candidate median is 23,175 ns.
The two pair ratios are 1.2380 and 1.3296.
Chord, arc, Rec. 601, and Rec. 709 score controls are inconclusive.
Existing Chromium sRGB and new Chromium CompuPhase complete calls are also inconclusive.
There are 62 passing decisions, six inconclusive decisions, and one regression.

The metric arithmetic is unchanged between roles. These results do not identify the regression's cause.
S41 may diagnose the retained artifacts. S25 adds no retry request or new release-blocking feature.
The candidate fails its required comparison set, so individual faster cases cannot justify promotion.

Existing-mode comparisons use S24 `f4b90ecfcde63531fb992cf87ebda04d4e373032` as accepted code.
New-mode comparisons use the selected all-mode baseline `fb8878b9` as accepted code.
Every candidate role uses rejected `4b717296`.
The selected baseline was never the candidate role in these measurements.
This trial does not establish selected-baseline-versus-S24 speedups for the five existing modes.

## Timed scope and limits

Native complete calls include preparation, owned result allocation, and result destruction.
Their source is borrowed. They do not include JavaScript boundary copies.
Public samples time one installed-package `quantize` call, including validation, copies, preparation, and durable result creation.
Initialization stays outside processing timers. Instances are primed; application caching and hashing are absent.
Public output inspection stays outside timers. Bounded retained result references can affect allocation and garbage collection.

Cylindrical component calls use preallocated output. Converter preparation and verification remain outside their timers.
Metric controls use preconverted coordinates, cyclic-successor pairing, preallocated f32 scores, and per-iteration black-box barriers.
Their fixtures and semantic identities bind the metric, coordinate space, and pairing version.
Score verification compares exact f32 bits and rejects nonfinite values or wrong counts.
These unchanged-math controls provide component coverage, not an acceleration claim.
There is no exact TypeScript indexed adapter, so these comparisons make no TypeScript quantize performance claim.

Observed runtimes are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
They use Node v24.19.0 and Playwright 1.59.1.
Measured pages are cross-origin isolated. Observed timer quanta are approximately 5 µs, 20 µs, and 20 µs respectively.
Raw samples preserve zero durations; the comparator accounts for timer resolution.
Engine compilation caches are uncontrolled. These are not cold-browser measurements or public Node-support claims.

## Artifact sizes

Sizes below come from retained artifact bytes. Compression uses gzip level 9 and Brotli quality 11.
The complete-implementation size budget remains a later release decision under #38.

| Artifact | Selected raw | Selected gzip | Selected Brotli | Rejected raw | Rejected gzip | Rejected Brotli |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Scalar Wasm | 222,298 | 101,646 | 85,289 | 223,780 | 102,251 | 85,653 |
| Threaded Wasm | 311,386 | 131,574 | 108,262 | 313,436 | 132,123 | 108,608 |

The selected npm tarball is 257,806 bytes; the rejected tarball is 259,105 bytes.
Selected SHA-256 identities are:

- Scalar Wasm: `620c944d2d593226dd921baeb3db85174568963e741727e7644217254c1c65e4`.
- Threaded Wasm: `a73414eb29994dadc0f609789c7c13940dbe7542e542df456e7f6378737e8427`.
- Tarball: `9ffab1675343c4fb4dd6c540f536ca45b310e867809ee3ab8fb7fcf8e0b532da`.
- Native worker: `e601ef4cfc9bc8f1ef23ac1eea0e834cfbcac8e4bc0697c3d12918e5e15c1bb8`.

Raw artifact paths are `accepted-public/ditherette.tgz`, its installed consumer, and `accepted-native/` under the evidence root.
Both role provenance files retain all other artifact and tool identities.

## Delivery validation

The implementation checkpoint records 309 native tests, nine private Wasm tests, and 21 public interface tests plus types.
It also records both release builds and installed-package checks in all three engines.
Delivery reuses that evidence only after the source and retained installed-file identity checks described above.
Post-rebase checks pass:

- `cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_quantize --test prod_quantize_allocation --test prod_processor_quantize`: 13 tests.
- `cargo test --locked --manifest-path crates/ditherette-bench/Cargo.toml --test metric_scores --test quantize_adapters --test paired_quantize --example matching_integration_plan`: nine integration tests and one budget test. The retained-artifact compatibility fixture stays explicitly ignored in this run; its earlier controlled validation remains recorded.
- `node --test scripts/prepare-public-benchmark.test.mjs scripts/prepare-native-benchmark.test.mjs`: seven controlled provenance tests.
- Both crate `cargo fmt --check` commands and `git diff --check` pass.
- Production, private Wasm adapter, package, and build-script bytes match prepared baseline `fb8878b9` exactly. Frozen spec/image and trusted guard policy match the S24 base.

Formatting commit `ba35205` only sorts benchmark module declarations. It does not change arithmetic or the public build.
These diagnostic test builds do not certify new measurement binaries. No delivery benchmark or package artifact build runs.
The assigned native target is returned to the coordinator after every owned process exits.

Implementation and delivery use GPT-6-astra with high reasoning in Codex.
The Codex coordinator's exact model identifier is unavailable; its measured evidence remains separately identified by revision.
