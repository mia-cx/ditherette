# S34 startup evidence

Issue #76 adds optional threaded initialization and teardown. Reuse the existing initialization collector and paired browser protocol.
The benchmark owner receives this worktree after S33 handoff and joins final S33 `b2ca677ed9927165a1010f5c52646a989d8a02ca`.

## Fixed experiment

Two initialization cases use a one-pixel nearest probe outside the initialization timer.
One case supplies already-loaded Wasm bytes. The other supplies an already-compiled module.
Both exclude package import and the initial Wasm fetch. Each new threaded pool can still load worker modules inside initialization.
Browser compilation caches are not reset.
Record these scopes rather than claiming cold network or first-ever compilation costs.

Run these engine-specific comparisons:

1. Fresh S33 accepted versus S34 candidate, both with threads disabled, across Chromium, Firefox, and WebKit.
2. The same S34 artifact in both roles, both with threads required inside a dedicated host worker, across Chromium and Firefox only.

Each comparison has two cases, two alternating role pairs, and twenty single-call samples per worker.
Use 50 ms warmup and a 10-second measurement cap. This is 40 serial workers and at most 800 samples.
The scalar comparison contributes 24 workers; the required-thread self-control contributes 16.
Attempt 01 is retained below. Root must approve any new artifact and measurement budget after the functional context correction.
Preserve noisy or failed cases; no independent retry or startup tuning belongs to this worktree.
Scalar regressions above 10% and inconclusive required evidence carry forward explicitly to S41.
Threaded and scalar absolute times are different initialization paths, not a claim that threading should initialize faster.

### Missing WebKit required-thread cell

WebKit required-thread startup is blocked and unmeasured, not passed or represented by scalar results.
The pinned engine retains locks after terminating workers parked in Wasm `atomic.wait`.
The runtime owner independently reproduces this with upstream's tiny Wasm fixture; real lifecycle checks pass Chromium and Firefox.
Repeated unreaped pools invalidate startup trial isolation, so neither initialization case runs in WebKit with threads required.
The missing cell represents eight workers and at most 160 samples from the original 48-worker declaration.
Preserve this unresolved lifecycle and measurement gap as S41 release evidence. Do not infer complete three-engine threaded support.
Do not add speculative capability probes, alter public APIs, or retry measurements to conceal this gap.
The relevant upstream correction is [WebKit's worker termination fix](https://github.com/WebKit/WebKit/commit/03e836de2f7bd5627a95f60357633d59fb6bb18d).
That correction is not evidence that the pinned engine contains the fix.

## Implementation TODOs

- [x] Add optional typed role thread policies to the developer protocol and evidence. Historical declarations retain scalar behavior.
- [x] Pass the selected public threads option through the actual adapter's preload and measured create call.
- [x] Reuse each role's existing Wasm asset entry for its selected scalar or threaded artifact. Bind exact bytes and worker assets through normal provenance.
- [x] Add focused protocol, actual adapter, and generator tests without running timing measurements.
- [x] Declare host-worker execution and reuse the existing collector there. Preserve historical page execution.
- [x] Prove repeated real required initialization and exact probes in the host without timing installed calls.
- [x] Prepare clean revision-bound accepted and candidate artifacts after runtime validation. Preserve identical benchmark protocol in both.
- [x] Drain agents/builds/tests, audit processes, and run the fixed matrix only under root's exclusive clearance.
- [x] Record worker reaping, exact probe outputs, browser/tool versions, sample counts, startup medians, and retained artifact hashes.

The runtime owner has `v1-s34-threads`; installed fixtures belong to `v1-s34-public`.
This worktree owns benchmark sources and tests only. Preserve frozen specs, production kernels, and package API policy.
Lifecycle tests prove real pool startup, partial cleanup, disposal, and host termination. Timing evidence does not replace those tests.
No public backend or worker-count control is added. No publishing, merging, rollout, or routine review occurs here.

## Protocol checkpoint

Optional `browser.threads` declares accepted/candidate policies using the existing typed `Threads` enum.
Historical cases and evidence omit the field unchanged; the actual adapter explicitly uses `disabled` when omitted.
Evidence must match both declared role policies. The package receives the selected policy during untimed preload and every factory call.
Eleven focused Node fixtures pass, including full initialization trials for bytes/compiled input, historical scalar defaults,
exact probes, disposal, and required failure without fallback. Twenty-six Rust browser protocol/worker fixtures pass.
These use fake clocks/packages for protocol behavior, not actual startup timing or proof of real worker pools.
The runtime/installed fixture owners supply real required-pool evidence before artifacts enter the fixed trial.

Compiler ownership is listed in the artifact handoff below. No S33 compiler output is reused.
The preparer copies `benchmark-host-worker.mjs` into the existing manifest-bound script tree.
Historical page/oracle exchanges keep their routes unchanged; only host execution loads the new helper.

`startup_integration_plan regression|threaded NEW_JSON HOST_LOAD_NOTES` reuses the S20 nearest fixture and its two initialization scopes.
The probe is exactly 1×1 RGBA `[11, 23, 47, 127]`, with 1×1 output and center nearest.
Both plans retain two pairs, twenty samples, 50 ms warmup, and a 10-second cap.
The generator does not select engines. The coordinator applies the engine matrix above, totaling 40 serial workers.
The generator fixture's full three-engine calculation is not permission to run the blocked WebKit cell.
Two generator fixtures pass, including frozen probe equality and the inherited S20 declaration test.
All Rust tests/examples compile with the existing stage-example unused-import warning only.

The coordinator selects `package/dist/wasm/scalar/ditherette_wasm_bg.wasm` for each disabled role.
Each required role selects `package/dist/wasm/threads/ditherette_wasm_bg.wasm` through its existing bundle-source Wasm entry.
Make selection in new source descriptors before immutable snapshots; preserve the complete package tree, including factory/bootstrap/worker assets.
Normal snapshot provenance binds every retained file and the selected entrypoint. Do not mutate an existing prepared snapshot.
Accepted preparation remains final S33 runtime plus this shared protocol, never the S34 implementation.

## Retained attempt 01

Root retains `target/s34-trial-01` with all raw requests, outputs, events, and immutable snapshots.
Twenty-four scalar workers complete 480 exact samples. The first Chromium threaded worker fails untimed preflight before any sample.
Firefox threaded remains unrun. Root confirms all launched children are reaped and ends the quiet phase.
The failure comes from pinned Rayon's `builder.build` reaching forbidden `Atomics.wait` on the main JavaScript thread.
Runtime corrects requested-thread capability handling separately; this worktree changes only the declared benchmark execution context.
Keep failed attempt 01 separate from any later evidence. Do not relabel it as a successful host-worker trial.

## Host-worker protocol checkpoint

Optional `browser.execution` declares `page` or `host-worker`; omitted historical records retain page execution.
The required-thread generator declares the host context for both roles. Scalar regression stays in the page.
The adapter rejects a context mismatch before package creation. Typed observation evidence binds the actual context to the declaration.
Host execution currently accepts only ordinary package initialization cases.

The page owns one dedicated module worker per trial. That host fetches the request and runs the existing `runTrial` and collector.
Package import, supplied Wasm fetch, host startup, HTTP serialization, and acknowledgement remain outside each initialization timer.
Timers run beside actual `createDitherette` calls inside the host. Per-pool worker/bootstrap loading remains inside those create calls.
Preload, exact probes, disposal, and compiled-input preparation retain their existing untimed scopes.
The host uploads results through the existing bounded HTTP route and reports success or the concrete failure.
The page terminates its host after either outcome. Existing browser shutdown owns cleanup after renderer/process failure.
The frozen oracle stays in its separate disposable page and closes before package initialization.

Focused Rust browser/worker/generator suites pass 28 tests. Node browser/startup/host suites pass 22 tests with the installed fixture skipped.
The separately enabled installed fixture passes both Wasm input forms using retained candidate `1d1cba89` in Chromium.
Each form completes preload plus three repeated required create/probe/dispose calls without timers and returns exact RGBA.
Every observed nested worker receives termination. This proves invocation and output, not browser-internal thread reaping or performance.
The fake-package host fixture exercises the actual collector with a fake clock, exact output, disposal, and propagated setup failure.

Reproduce the untimed host check from this worktree with an existing installed artifact descriptor:

```sh
DITHERETTE_BENCH_STARTUP_BUNDLE=/absolute/existing/bundle-source.json \
  node --test scripts/benchmark-host-worker.test.mjs
```

The checked descriptor points to `v1-s34-threads/target/s34-candidate-1d1cba89-public/bundle-source.json`.
That retained artifact predates the runtime capability correction; its host-worker initialization path is unchanged.
Root must regenerate the required-thread declaration with the newly built generator because older executables omit `execution`.

## Historical artifact handoff

Preserve accepted artifacts built at clean `bf7912db096810bf63ab3cfa39aba20f1d20291d` in this worktree:

- `target/s34-accepted-bf7912db-native` contains the copied worker, coordinator, and build provenance.
- `target/s34-accepted-bf7912db-public` contains the installed package, scripts, oracle, and build provenance.
- `target/compiler/release/examples/startup_integration_plan` declares either comparison without measuring it.

These `bf7912db` paths describe an earlier checkpoint. Attempt 01 instead uses accepted `f55f100fd3efdbe6ae63014b22d0285fa0d64d04` artifacts.
Both artifact sets remain intact. The host-worker protocol requires new official artifacts at clean joined sources after root authorization.
Before snapshot preparation, root retains a clean detached source checkout at `bf7912db096810bf63ab3cfa39aba20f1d20291d`.
New accepted source descriptors point `source_checkout` there because provenance validates the exact HEAD and all tracked inputs.
Leave the original artifact descriptor and provenance untouched. No rebuild belongs to this amendment.
Compiler ownership remains this worktree's `target/compiler` and `crates/ditherette-wasm/target/{scalar,threads}` until root's cleanup handoff.

After root authorizes and builds the new protocol, declare fresh plans with that source's generator:

```sh
target/compiler/release/examples/startup_integration_plan regression NEW_REGRESSION_JSON HOST_LOAD_NOTES
target/compiler/release/examples/startup_integration_plan threaded NEW_THREADED_JSON HOST_LOAD_NOTES
```

Use absolute paths for each `NEW_*` destination and describe current host activity in `HOST_LOAD_NOTES`.
For each authorized engine, root creates a new `BrowserSources` JSON with that engine's existing runtime descriptor.
Regression binds accepted S33 and candidate S34 workers, their exact revisions, and scalar Wasm entries in both roles.
Threaded control binds the same candidate S34 worker, revision, package, and threaded Wasm entry in both roles.
Prepare each cell with the copied coordinator; this does not run workloads:

```sh
target/s34-accepted-bf7912db-native/ditherette-bench-pair prepare-browser \
  EXPERIMENT_JSON ACCEPTED_WORKER FULL_ACCEPTED_REVISION \
  CANDIDATE_WORKER FULL_CANDIDATE_REVISION SOURCES_JSON NEW_PREPARED_DIRECTORY
```

The command shape above remains unchanged; replace historical coordinator and worker paths with root's new revision-bound artifacts.
The amended matrix has three regression snapshots and two threaded snapshots, subject to root's explicit new budget approval.
Record WebKit threaded as blocked without fabricating a snapshot or result.
Root alone grants quiet clearance and runs measurements after all builders and tests drain.

## Completed trial 02

The [final report](76-benchmark-results.md) and its JSON companion supersede the historical artifact paths above.
Accepted artifacts use `3f2cc41a`; measured candidate artifacts use `c01467f9`.
Root completes all forty scheduled workers with 800 samples, then audits zero owned processes at 18:13:34 UTC.
All recorded outputs are exact. Four case gates pass and six remain inconclusive; none confirms a slowdown above 10%.
WebKit threaded remains blocked and unmeasured. Attempt 01 and every earlier artifact remain retained.
No further trial, tuning, or promotion is authorized by this completion note.
