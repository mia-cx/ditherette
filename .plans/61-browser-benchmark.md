# S20 complete public browser measurements

Implement issue #61 from S19 PR #105 at `7de86d799a25a132c8de41ee54696bd8e54bdf76`.
Read the approved PRD, [S20 acceptance](../docs/plans/ditherette-v1/slices.md#s20), [preflight](61-browser-preflight.md), and benchmark execution/paired contracts before work.
The immediate PR base is `impl/v1-s19-integration`. Preserve all frozen and literal-copy checkpoints.

## TODOs

- [x] Extend the typed paired protocol and comparison checks for public operations, browser runtime/artifact identity, and truthful cache/init scopes.
- [x] Snapshot browser/package/TypeScript assets and dispatch owned browser workers through the existing paired lease protocol. Preserve shared-library file aliases before real trials.
- [x] Add clean-source build/pack/install provenance with complete source and output digests.
- [x] Implement actual installed-package calls and equivalent TypeScript adapters, with one call per latency sample and separate throughput/init measurements.
- [x] Test operation registration, exact output proof, asset tampering, malformed transport, and owned-child cleanup without measurements.
- [x] Join the independent deliveries and validate real browser conformance plus existing native protocol compatibility.
- [x] Declare a bounded initial case/pair budget, prepare clean artifacts, drain implementation, and collect exclusive fresh browser trials. Trial 02 completes all three engines; trial 01 remains retained separately.
- [x] Record raw samples, complete identities, exact output checks, performance outcomes, and cleanup evidence; preserve failures without treating them as accepted optimizations.
- [x] Rebase with merge/checkpoint preservation, validate affected checks, and open an unmerged PR with current progress and dependencies.

## Ownership

The protocol agent owns `paired.rs`, its coordinator, public-operation protocol modules, and paired protocol fixtures.
The transport agent owns new browser scripts, the TypeScript operation adapter, and focused script fixtures.
The artifact/worker agent owns browser asset preparation, the Rust browser worker, CLI dispatch, and their fixtures.
The coordinator owns shared manifests and module registrations at integration, this plan, evidence, progress, and PR filing.
Agents work in separate worktrees. Agree the typed request/result contract before crossing ownership boundaries.

## Boundaries

Reuse S04 ownership and S05 verification. The process chain is external coordinator → one benchmark worker → Node → browser.
All latency samples call the real public method exactly once. Include boundary copies and public preparation inside timing.
S19 has no content hashing or application cache. State that explicitly; reject fake cold/warm cache claims.
Initialization has a separate declared loading/compilation scope. TypeScript has no equivalent Wasm initialization.
Use the actual TypeScript nearest implementation for center-anchor equivalents; record other anchors as unavailable in TypeScript.
Hash and retain the entire served dependency closure, runtime identity, and inputs/settings, not only the worker executable.
Keep native prepared evidence readable. New public operation tags must be extensible by later processing slices.

Only preparation and controlled diagnostics run during implementation. The coordinator starts any real benchmark after every agent and owned build/test exits.
No merges of PRs, publication, tags, deployment, rollout, spec edits, root-user edits, or non-exact acceptance.

## Integration evidence

Transport checkpoints `fe06a927`, `d21352b6`, and `15d1292a` join the coordinator branch.
Their controlled timing tests preserve zero samples and avoid stalled-clock warmup.
The actual TypeScript closure compiles offline. All three engines confirm the known 2→49 tie-rounding mismatch without measurements.
The clean-build preparer owns build/pack/install before the quiet phase and emits the worker's agreed provenance schema.
Three focused provenance fixtures pass, covering hidden Git index edits, build overrides, complete sorted hashes, and rejected symbolic links.

Joined revision `279a080c9b2c257e177cddc62be1c00fd1cd790e` passes 44 top-level Rust benchmark tests, 16 focused JavaScript tests, the typed nine-case matrix test, and the trusted S18 guard.
Release benchmark executables and both independent package builds complete at that clean revision.
The actual builder removes a planted obsolete generated-output sentinel before packing.
Both fresh source/output manifests pass the Rust source validator.
Their artifacts remain at `/tmp/ditherette-s20-final.oJvHLy/accepted` and `/tmp/ditherette-s20-final.oJvHLy/candidate`.
The package tarball digest is `bed93cd2085df64a2ca8ba578fd6d72babccc539042e83847e66a691bde59c1d`.

Fresh installed-package conformance passes Chromium 147.0.7727.15 and Firefox 148.0.2.
The copied WebKit launcher starts but aborts during page creation; the original launcher passes the same operation.
All 65 copied installation/library files have identical bytes and read/execute bits.
Copying symbolic aliases into separate files loses shared-library inode identity. A fresh hardlink-preserving copy passes the same page test.
Correction `1d0b7c8fc633ae5a6b4f173f5eef36ed08da7131` joins as `785696fc` and binds canonical alias groups into the content manifest.
Controlled fixtures preserve aliases through a second snapshot and reject split or falsely merged groups.
The coordinator reruns full installed-tarball conformance with the corrected runtime. Chromium, Firefox, and WebKit pass all four reported tests.
Retain the failed runtime as diagnostic evidence. Rebuild and prepare both roles after this correction before any measurements.
Store full trial results on the workspace filesystem because `/tmp` has only 2 GiB free.
The first exclusive Chromium run starts after all implementation processes exit, but its transport fails at worker 62.
Read [the retained failure evidence](61-trial-01-failure.md) before resuming measurements.
All 62 workers are reaped; 61 complete results contain 6,100 exact samples. The comparison remains incomplete.
The bulk-data transport correction joins as `b1a86c75`. Read [its bounded-memory evidence](61-ipc-memory-fix.md).
The coordinator verifies 20 focused checks and all three browser conformance subtests, plus their parent test.
The large untimed HTTP echo transfers both arrays under a 128 MiB Node heap limit, with maximum Playwright message 1,244 bytes and peak RSS 270,976 KiB.
An actual renderer-crash test rejects pending transport work. Image timing and the fixed case/pair budget are unchanged.
All implementation processes exit before rebuilding and preparing fresh trial-02 artifacts. No performance gate or optimization is accepted.
Trial 02 completes from clean revision `e84a55eddb0014f97b64446408bfb5f656deb5d4`.
Read [the actual measurement report](61-public-measurement.md) for every engine, case, and gate.
Every output is exact, but all three engine reports contain regressions. S41 must resolve these before release readiness.
A merge-preserving rebase onto the current S19 parent preserves the exact measured revision and tree.
Post-rebase validation passes 45 top-level Rust benchmark tests and 20 focused JavaScript tests.
The independent evidence audit verifies 216 exact outputs, 21,481 retained samples, and 2,465 snapshot files.
All report medians and gates recompute. The 52 WebKit zero samples remain visible and inconclusive.
Open, non-draft [PR #107](https://github.com/mia-cx/ditherette/pull/107) stacks on S19 without merging.
Its creation head is `b97e0e5b20b198692fe37944f89ccdfc51734426`; the measured implementation remains `e84a55ed`.
S20 tooling is ready. Known performance regressions remain tracked in S41, with the focused diagnosis still in progress.
