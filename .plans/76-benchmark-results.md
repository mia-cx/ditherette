# S34 startup results

Trial 02 completes with exact recorded outputs and no confirmed slowdown above 10%.
Four case gates pass and six remain inconclusive. WebKit required-thread startup remains blocked and unmeasured.
This evidence supports the reviewable S34 implementation, not release promotion or a kernel speedup.

## Sources and retained evidence

The PR stacks on S33 `b2ca677ed9927165a1010f5c52646a989d8a02ca`.
Accepted S33 runtime plus shared protocol is `3f2cc41a9fa13f6b29f286a6cb7d5a9e690c32d8`.
Measured S34 candidate is `c01467f9ed45ba79d855b95421564f2088929693`.
Delivery before this report is `3863cadce82b4d272a1730f3ebad828b18ac4434`; its only post-measurement change is documentation.
The ancestry-preserving rebase onto S33 leaves that HEAD and its tree unchanged.

These paths supersede earlier `bf7912db`, `f55f100f`, `2afd1802`, and `1d1cba89` artifact handoffs:

- Accepted native/public: `v1-s34-bench/target/s34-accepted-3f2cc41a-{native,public}`.
- Candidate native/public: `v1-s34-threads/target/s34-candidate-c01467f9-{native,public}`.
- Trial 02: `v1-s34-bench/target/s34-trial-02`, including five result folders, prepared snapshots, and `quiet-phase.json`.
- Detached provenance sources: `v1-s34-accepted-3f2cc41a` and `v1-s34-measured-c01467f9`.

Paths are relative to `.worktrees/`. Earlier artifacts and attempt 01 remain intact.
The [machine-readable companion](76-benchmark-results.json) records absolute paths, full SHA-256 values, raw inventory hashes, and browser versions.

## Scope and audit

Each comparison uses two initialization cases, two alternating role pairs, and twenty single-call samples per worker.
Warmup is 50 ms with a 10-second measurement cap. The exact nearest probe is 1×1 RGBA `[11, 23, 47, 127]`.

Scalar regression compares S33/S34 with threads disabled in the page.
Required-thread controls compare the same S34 artifact in both roles inside a dedicated host worker.
The timer surrounds actual `createDitherette`, including per-pool bootstrap/loading and Rayon priming.
Host startup/RPC, package import, supplied Wasm fetch, precompilation, probes, disposal, and serialization stay outside.
Browser compilation caches are not reset. These are neither cold-network nor first-ever compilation measurements.

The audit matches 40 started PIDs to 40 reaped events, with global maximum live benchmark workers of one.
All forty workers contribute twenty samples: 800 total, plus 2,453 warmup calls.
Twenty actual accepted/candidate role pairs match exactly.
Forty recorded outputs match both recorded native and target-local Wasm references.
All forty three-way records and their 120 comparison components are exact.
Warmup/sample probes are checked during collection; this audit does not invent separately retained bytes for every transient result.

Every pooled median below reproduces directly from the raw samples.
Copied worker hashes match snapshot identities, and package/script/TypeScript files match official provenance.
Root starts the quiet phase at 18:06:46 UTC and audits zero owned processes at 18:13:34 UTC on 2026-09-08.
The recorded machine is Linux x86_64, AMD Ryzen 9 7950X, with 24 visible logical CPUs.
Browser versions are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4; Node is v24.19.0 and Playwright 1.59.1.

## Startup medians

Times are milliseconds. Ratio is candidate / accepted; threaded rows use the same candidate artifact in both roles.

| Comparison | Input | Accepted ms | Candidate ms | Ratio | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| regression-chromium | bytes | 0.4650 | 0.4750 | 1.0215 | pass |
| regression-chromium | compiled | 0.1050 | 0.0950 | 0.9048 | inconclusive |
| regression-firefox | bytes | 4.6900 | 4.5000 | 0.9595 | inconclusive |
| regression-firefox | compiled | 0.1400 | 0.1400 | 1.0000 | inconclusive |
| regression-webkit | bytes | 1.2300 | 1.2000 | 0.9756 | inconclusive |
| regression-webkit | compiled | 0.1900 | 0.2000 | 1.0526 | inconclusive |
| threaded-chromium | bytes | 60.1775 | 62.1975 | 1.0336 | inconclusive |
| threaded-chromium | compiled | 59.1575 | 60.3875 | 1.0208 | pass |
| threaded-firefox | bytes | 42.7200 | 43.9400 | 1.0286 | pass |
| threaded-firefox | compiled | 36.4500 | 37.0900 | 1.0176 | pass |

The existing gate requires both role orders to support the threshold and sufficiently stable pair ratios.
Clock-resolution uncertainty also prevents a pass. Firefox scalar compiled is explicitly resolution-limited.
A favorable pooled median alone does not resolve an inconclusive gate.
Threaded initialization medians span about 36–62 ms. They measure pool startup, not parallel image-processing speed.

Required controls pass both Firefox scopes and Chromium compiled input.
Chromium required bytes remains inconclusive. Scalar Chromium bytes passes; all other scalar scopes remain inconclusive.
No measured case confirms a slowdown above 10%. Inconclusive cases remain S41 evidence gaps, not passes.

## Retained failures

Attempt 01 has 25 started/reaped workers and 480 scalar samples.
The first Chromium threaded worker fails untimed preflight; its empty result file and concrete stderr remain.
Firefox threaded is unrun. Root and runtime reproduce the underlying main-JS blocking-wait failure independently.

The correction checks blocking-wait capability only when threads are requested.
Main-JS preferred initialization falls back to scalar; required initialization reports capability failure.
Failed builder cleanup releases generated JS ownership without reentering potentially trapped Rust borrow state.
The benchmark now declares its threaded host context explicitly. Trial 02 follows those functional corrections, not noise tuning.

Pinned WebKit retains worker locks after disposal and processing-host termination.
The independent tiny-Wasm witness reproduces the engine failure without Ditherette or Rayon.
The [upstream termination fix](https://github.com/WebKit/WebKit/commit/03e836de2f7bd5627a95f60357633d59fb6bb18d)
does not establish that the pinned engine is fixed.
Both required-thread WebKit cases remain blocked: eight excluded workers, at most 160 samples, and no fabricated result.
This lifecycle gate and the missing evidence remain unresolved S41 release blockers.

## Artifacts and validation

Accepted tarball SHA-256 is `f6e62526cc290d8ca1f9fdcb7fcfc9de39790c1982dab7118adb30a4a84173d2`.
Candidate tarball SHA-256 is `33a46ac0de03c1d9947302af356648549cd288f8cfcbf5b3953843af65d75c9d`.
The accepted package is byte-identical to S33. Candidate and accepted share identical benchmark transport, page, and host-helper bytes.
Frozen oracle binaries differ by target build; each role uses its own manifest-bound oracle and all recorded comparisons are exact.

| Artifact | Accepted bytes | Candidate bytes |
| --- | ---: | ---: |
| Package tarball | 370620 | 375167 |
| Scalar Wasm raw / gzip | 339664 / 152754 | 339443 / 152653 |
| Threaded Wasm raw / gzip | 445275 / 185120 | 445270 / 185060 |

Gzip sizes use Node's default `gzipSync`. The tarball grows by 4,547 bytes, about 1.23%.
Existing focused capability, pool, generated-builder, host, and partial-start validations are recorded in [the runtime plan](76-threaded-init.md).
The exact candidate tarball passes the untimed repeated-host startup check for both input forms.
The trusted frozen guard passes at measured source `c01467f9`. No broad tests, builds, or measurements run while writing this report.

Compiler cleanup belongs to root after PR handoff.
Retain all copied native binaries, public artifacts, detached sources, snapshots, raw results, and conformance/diagnostic evidence.
The six compiler directories are `target/compiler`, `crates/ditherette-wasm/target/scalar`, and `crates/ditherette-wasm/target/threads`
under each of `v1-s34-bench` and `v1-s34-threads`; they are not evidence directories.
