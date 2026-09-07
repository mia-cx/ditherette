# #61 Public browser transport

## Scope

Measure installed-package calls and the real website TypeScript nearest implementation.
The paired protocol owns validation and acceptance. The Rust worker owns reference output and artifact snapshots.

## TODOs

- [x] Add the TypeScript adapter and deterministic timing tests, including zero samples and per-sample preparation.
- [x] Add the immutable-asset browser transport using the shared trial schema and existing resource cleanup.
- [x] Verify real TypeScript output and installed-package calls without running measurements; record runtime provenance and handoff.
- [x] Add worker-supplied frozen output preflight and preserve structured mismatch output before timing.

## Constraints

No benchmark runs during implementation. Fake clocks verify timing mechanics.
S19 has no application cache or hashing. Fresh instances and primed instances describe allocation/runtime state only.
Initialization excludes module loading and network fetch. Bytes mode includes compilation; compiled mode excludes it.
The TypeScript adapter preserves its existing nearest rounding, including the known 2→49 mismatch at x=24.
Only center-aligned TypeScript nearest is available. Identity calls include the durable byte copy within the measured operation.

## Validation

TODO 1 passes six deterministic Node fixtures and focused TypeScript checking.
The offline compiler emits the six real adapter/source modules, with explicit `.js` imports and compiler/input hashes.
Warmup uses wall time including preparation/disposal, with a bounded stalled-clock failure. Throughput calibration uses operation time only.
No real operation timing runs during these tests.

TODO 2 passes eleven focused fixtures. The HTTP server and browser routing share one manifest allowlist.
Undeclared dependencies, external requests, runtime-version drift, and isolation drift fail the trial.
The existing leased transport still owns browser/server signal and stdin-liveness cleanup.
Initialization probes and disposal occur after each create timer. Fresh processing creates before each call timer.

TODO 3 passes the installed-tarball fixture in Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
The fixture invokes `prepareOperation` only. It never invokes the measured runner or wraps real operations in clocks.
Each engine checks independent 4→3 vectors, durable identity copying, fresh instances, both initialization scopes, and missing-import rejection.
TypeScript remains stateless for the fresh-instance comparison. Its ordinary call prepares plans each time; only the package creates an instance.

All three engines reproduce the inherited TypeScript mismatch at source 2→output 49, x=24.
Source red channels `[10, 11]` produce TypeScript red 10 and frozen-compatible package red 11 at that position.
Do not label those dimensions exact or repair the website algorithm in S20.

### Reproducible conformance

```sh
node --test scripts/benchmark-public-browser.test.mjs scripts/benchmark-public-timing.test.mjs scripts/prepare-benchmark-typescript.test.mjs
pnpm exec tsc --noEmit --target es2022 --module esnext --moduleResolution bundler --skipLibCheck --ignoreConfig scripts/benchmark-typescript.ts
DITHERETTE_BENCH_TEST_TARBALL=/tmp/ditherette-s20-fixture.qc4Tgh/ditherette-0.1.0.tgz DITHERETTE_TEST_WEBKIT_EXECUTABLE=/tmp/ditherette-webkit-libs.2dS6Yu/webkit node --test scripts/benchmark-public-conformance.test.mjs
```

The first command passes 13 fixtures. The browser command passes three engine subtests and its containing test, four reported tests.
Node is 24.19.0, pnpm 11.13.0, Playwright 1.59.1, and TypeScript 6.0.3.
The supplied S19 tarball SHA-256 is `bed93cd2085df64a2ca8ba578fd6d72babccc539042e83847e66a691bde59c1d`.
It comes from validated integration `7de86d799a25a132c8de41ee54696bd8e54bdf76` and serves only as conformance input.
Actual measurements require fresh clean builds and immutable preparation.

WebKit uses the task-local launcher and extracted libraries documented in [S19 browser provenance](60-browser-runtime.md).
The fixture installs the tarball offline into its own temporary consumer and removes that consumer after all owned browsers/servers close.
No host package installation, website preview, benchmark, or deployment occurs.

### Frozen preflight and integration

The browser worker supplies `TrialRequest.reference_output` using its frozen implementation.
Before warmup, the page runs one untimed actual operation and compares dimensions, all RGBA bytes, and warnings.
Mismatch returns the complete actual output with `timing_skipped: "reference-mismatch"`, empty samples, and zero work counts.
The worker preserves that response for S05 mismatch artifacts. It cannot become performance evidence.
A fake-operation fixture proves this path executes once without starting warmup. All three real browsers retain the TypeScript witness through this check.

The wire format follows shared protocol commits `011e29586332f6f1a9cd55e36898bf7f55ef693c` and `12e0d04`.
Node receives one trial JSON file and emits one result JSON object. Diagnostics use stderr.
The Rust worker validates all asset/runtime hashes before and after Node; this transport enforces the served-path and network allowlist.
Only the worker/coordinator may invoke measured `runTrial` under the existing shared lease and quiet contract.
Cold/warm cache tags remain rejected until real application cache controls exist. Future cold preparation belongs in the per-sample `prepare` hook after warmup.
