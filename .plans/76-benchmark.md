# S34 startup evidence

Issue #76 adds optional threaded initialization and teardown. Reuse the existing initialization collector and paired browser protocol.
Root owns this worktree until the S33 report owner returns and receives an explicit handoff.

## Fixed experiment

Two initialization cases use a one-pixel nearest probe outside the initialization timer.
One case supplies already-loaded Wasm bytes. The other supplies an already-compiled module.
Both exclude package import and asset fetch. Browser compilation caches are not reset.
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

- [ ] Add optional typed role thread policies to the developer protocol and evidence. Historical declarations retain scalar behavior.
- [ ] Pass the selected public threads option through the actual adapter's preload and measured create call.
- [ ] Reuse each role's existing Wasm asset entry for its selected scalar or threaded artifact. Bind exact bytes and worker assets through normal provenance.
- [ ] Add focused protocol, actual adapter, and generator tests without running timing measurements.
- [ ] Prepare clean revision-bound accepted and candidate artifacts after runtime validation. Preserve identical benchmark protocol in both.
- [ ] Drain agents/builds/tests, audit processes, and run the fixed matrix only under root's exclusive clearance.
- [ ] Record worker reaping, exact probe outputs, browser/tool versions, sample counts, startup medians, and retained artifact hashes.

The runtime owner has `v1-s34-threads`; installed fixtures belong to `v1-s34-public`.
This worktree owns benchmark sources and tests only. Preserve frozen specs, production kernels, and package API policy.
Lifecycle tests prove real pool startup, partial cleanup, disposal, and host termination. Timing evidence does not replace those tests.
No public backend or worker-count control is added. No publishing, merging, rollout, or routine review occurs here.
