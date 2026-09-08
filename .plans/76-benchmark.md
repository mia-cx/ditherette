# S34 startup evidence

Issue #76 adds optional threaded initialization and teardown. Reuse the existing initialization collector and paired browser protocol.
The benchmark owner receives this worktree after S33 handoff and joins final S33 `b2ca677ed9927165a1010f5c52646a989d8a02ca`.

## Fixed experiment

Two initialization cases use a one-pixel nearest probe outside the initialization timer.
One case supplies already-loaded Wasm bytes. The other supplies an already-compiled module.
Both exclude package import and the initial Wasm fetch. Each new threaded pool can still load worker modules inside initialization.
Browser compilation caches are not reset.
Record these scopes rather than claiming cold network or first-ever compilation costs.

Run two comparisons across Chromium, Firefox, and WebKit:

1. Fresh S33 accepted versus S34 candidate, both with threads disabled. This measures scalar initialization regression.
2. The same S34 artifact in both roles, both with threads required. This records threaded startup cost and a same-artifact control.

Each comparison has two cases, two alternating role pairs, and twenty single-call samples per worker.
Use 50 ms warmup and a 10-second measurement cap. This is 48 serial workers and at most 960 samples.
Complete the fixed matrix once. Preserve noisy or failed cases; no retry or startup tuning belongs to S34.
Scalar regressions above 10% and inconclusive required evidence carry forward explicitly to S41.
Threaded and scalar absolute times are different initialization paths, not a claim that threading should initialize faster.

## Implementation TODOs

- [x] Add optional typed role thread policies to the developer protocol and evidence. Historical declarations retain scalar behavior.
- [x] Pass the selected public threads option through the actual adapter's preload and measured create call.
- [x] Reuse each role's existing Wasm asset entry for its selected scalar or threaded artifact. Bind exact bytes and worker assets through normal provenance.
- [x] Add focused protocol, actual adapter, and generator tests without running timing measurements.
- [ ] Prepare clean revision-bound accepted and candidate artifacts after runtime validation. Preserve identical benchmark protocol in both.
- [ ] Drain agents/builds/tests, audit processes, and run the fixed matrix only under root's exclusive clearance.
- [ ] Record worker reaping, exact probe outputs, browser/tool versions, sample counts, startup medians, and retained artifact hashes.

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

The only compiler target currently claimed is this worktree's new ordinary `target/compiler`.
No S33 compiler output is reused. No new helper imports or manual browser routes are needed.

`startup_integration_plan regression|threaded NEW_JSON HOST_LOAD_NOTES` reuses the S20 nearest fixture and its two initialization scopes.
The probe is exactly 1×1 RGBA `[11, 23, 47, 127]`, with 1×1 output and center nearest.
Both plans retain two pairs, twenty samples, 50 ms warmup, and a 10-second cap, totaling 48 serial workers.
Two generator fixtures pass, including frozen probe equality and the inherited S20 declaration test.
All Rust tests/examples compile with the existing stage-example unused-import warning only.

The coordinator selects `package/dist/wasm/scalar/ditherette_wasm_bg.wasm` for each disabled role.
Each required role selects `package/dist/wasm/threads/ditherette_wasm_bg.wasm` through its existing bundle-source Wasm entry.
Make selection in new source descriptors before immutable snapshots; preserve the complete package tree, including factory/bootstrap/worker assets.
Normal snapshot provenance binds every retained file and the selected entrypoint. Do not mutate an existing prepared snapshot.
Accepted preparation remains final S33 runtime plus this shared protocol, never the S34 implementation.
