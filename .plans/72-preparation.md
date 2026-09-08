# S31 bounded preparation reuse

Issue #72. Implementation and bounded measurements are complete.
Branch `impl/v1-s31-preparation` targets S30 PR #119, `impl/v1-s30-process`.
Reviewable unmerged PR: https://github.com/mia-cx/ditherette/pull/121.
The joined parent is `f408bc99a80d3c83b6caee0b5c1d19868f0db876`.
Both benchmark roles use protocol `863889e52f1b752b6adfc22a9c775b3823f2997e`.
The measured preparation baseline is `972d4e9a5882b25bca3de5f0786ad1525b5e6329`.
Subsequent report and handoff commits leave runtime and protocol unchanged.

## Scope

Reuse prepared palettes, resize plans, and idle scratch through existing public calls.
Preserve landed kernels, shared helpers, frozen spec/image, and S30 composition boundaries.
S32 owns image-result caching. This slice retains no valid source or image-stage result.
Decision #37 and frozen `spec/contract/cache.rs` plus `cache.md` define the control contract.

## TODOs

- [x] Copy the missing frozen cache control baseline literally and verify preparation identities and capacity/lifecycle fixtures.
- [x] Implement actual-capacity preparation ownership, allocation-free identities, one LRU, pinning, and commit-on-success publication.
- [x] Reuse palette preparation across quantize, ditherAndQuantize, and Process without changing execution kernels.
- [x] Reuse resize plans and typed idle scratch, preserving trilinear overwrite and diffusion row initialization.
- [x] Verify native/public cold-warm equality, budget pressure, failure recovery, isolation, and durable results.
- [x] Join benchmark support, prepare fresh artifacts, and complete coordinator-owned exclusive cold/warm trials.
- [x] Join the report-only commit and file reviewable stacked PR #121; retain compiler caches through validation and handoff.

## Accounting design

One instance owns retained preparation and idle scratch. Retained entries share both approved caps:
128 entries and `min(256 MiB, memoryLimitBytes / 4)`. Idle scratch counts against total memory separately.
Whole-call preflight includes cache metadata, pinned preparation, pending misses, active scratch, buffers, and boundary records.
Pressure drops idle scratch first, then unpinned published LRU entries. Capacity moves between owners without double counting.
Misses publish only after final boundary completion succeeds. Expected failure drops pending owners and active scratch.

Prepared palette identity uses the retained ordered prefix, truncation flag, alpha policy, and matching policy.
Resize identity includes source dimensions and complete output settings. Neither includes image bytes or execution policy.
Validation still runs before hits, preserving palette-tail validation and error ordering.

The runtime charges heap-owned preparation records separately from their nested heap capacities.
Diffusion borrows the prepared quantizer and owns width-dependent scratch separately.
Trilinear image storage is scratch, never a cached image.

## Validation and ownership

The runtime agent owns this worktree's `target/compiler` (3.0 GiB),
`crates/ditherette-wasm/target/scalar` (250 MiB), and `crates/ditherette-wasm/target/threads` (234 MiB).
No local `tools/spec-freeze/syntax/target` exists. The borrowed S30-base syntax cache has returned to the coordinator.
Keep assigned compiler caches through PR validation and handoff, then remove only these validated compiler paths.
Preserve `target/s31-candidate-*` artifacts and conformance evidence. The coordinator owns `target/s31-trial-01`.
The coordinator also owns issues, ledger, trial selection, and benchmark results; the benchmark agent supplies the report-only commit.
All implementation, build, and test jobs drained before exclusive measurements. The bounded trial has finished.

## Literal checkpoint evidence

At checkpoint `0e90491500efcad950982a5b44df6013283c44aa`, the literal model was the only S31 implementation.
`prod/contract/cache.rs` remains byte-identical to frozen `spec/contract/cache.rs`.
Its relative imports resolve to production contracts, but the active pipeline uses the physical preparation store instead.
The complete copy retains reference identity vocabulary for comparison; S31 activates preparation keys only.

`prod_preparation_baseline` passes 10 tests. Nine replay frozen capacity/lifecycle fixtures unchanged.
The additional test compares 450 prepared-palette keys and 126 resize-plan keys against frozen implementations.
It covers all 15 matching policies, alpha threshold precision/signed zero, palette order/duplicates/truncation, seven filters, and nine anchors.
The existing frozen cache suite passes all 16 tests in the same native run.
Those checkpoint results establish modeled behavior only. The runtime checks below verify physical ownership and live reuse.

## Runtime checkpoint

All five methods now use one preparation transaction and isolated store. Keys stream canonical JSON directly into SHA-256.
The store reserves 128 inline optional entry records; each populated value owns a fallibly reserved single-record Vec.
Accounting includes those Vec capacities, prepared heap capacity, active records, and idle scratch without double counting.
Published resize entries separate mutable scratch accounting from deterministic plans. Pressure releases scratch before plans.
Diffusion borrows the same prepared quantizer and separately owned rows through the unchanged execution loop.

Six private ownership tests and one integration test comparing active streaming keys directly against frozen keys pass.
They cover cross-method hits, two Process pins, both retention caps, LRU refresh, scratch-first pressure,
failed final-copy publication, and trilinear overwrite after failure.
The native Processor (12), field (6), quantize (3), and Process (5) suites pass after cold allocation-fixture updates.
Full native tests, 34 public interface tests, 16 private Wasm tests, and four benchmark preparation protocol tests pass.
Two preparation matrix tests verify all four declared workloads cold and after changed-source priming against frozen outputs.
Trusted frozen guard passes native, Wasm, and threaded production isolation against the approved S30-base policy.
Independent review found one diffusion scratch growth overlap. Dropping old rows before reservation fixes it;
a native allocator witness verifies live bytes during the exact-budget replacement allocation.

The installed tarball suite passes all four tests across Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Each engine checks 423 Process compositions, 360 diffusion vectors, 367 frozen Wasm Yliluoma vectors,
and 734 untimed actual benchmark-adapter calls, alongside existing public and field checks.
The bounded browser fixture now reserves 32 KiB for preparation-control records instead of 4,000 bytes.
It still rejects the original 40,400-byte source and verifies recovery.
Final public artifacts match all 38 previously tested package files byte-for-byte.
Their shared tarball SHA-256 is `ce556c8dc5a7f747ebdd572d84d0185a9d2db2796cd9d3af085905fa3d9bb4a0`.
`target/s31-candidate-972d4e9a-public/conformance.md` binds tests, artifact provenance, and retained per-engine oracle results.

## Bounded trial outcome

All measured outputs are exact. No case confirms a median regression above 10%.
Chromium and Firefox pass overall. Native warm Process and three WebKit cases remain inconclusive.
Warm Lanczos3 improves across native (12.5%), Chromium (5.0%), Firefox (6.3%), and WebKit (10.5%).
The coordinator retains the required preparation baseline without retry or extra tuning.
Read the joined `.plans/72-benchmark-results.md` for per-case results and uncertainty before citing performance.
Report commit `d5d085dcd9c433e553de14d9087ae04d52fcce02` joins here as `680ec7f0`.
PR preparation rebased onto the latest S30 parent, resolving only the previously fixed duplicate Vec-record charge.
Full-tree equality against `a59d299f` passed after reconciliation; a no-code merge preserves the measured candidate's ancestry.
All five focused Process tests pass after reconciliation, including the independent separable-converter budget check.
