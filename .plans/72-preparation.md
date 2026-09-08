# S31 bounded preparation reuse

Issue #72. Branch `impl/v1-s31-preparation` starts from validated S30 PR #119,
`88eb79fc129662fcfc6d4554d3109855348d0316`. Immediate PR parent is `impl/v1-s30-process`.

## Scope

Reuse prepared palettes, resize plans, and idle scratch through existing public calls.
Preserve landed kernels, shared helpers, frozen spec/image, and S30 composition boundaries.
S32 owns image-result caching. This slice retains no valid source or image-stage result.
Decision #37 and frozen `spec/contract/cache.rs` plus `cache.md` define the control contract.

## TODOs

- [x] Copy the missing frozen cache control baseline literally and verify preparation identities and capacity/lifecycle fixtures.
- [ ] Implement actual-capacity preparation ownership, fallible identities, one LRU, pinning, and commit-on-success publication.
- [ ] Reuse palette preparation across quantize, ditherAndQuantize, and Process without changing execution kernels.
- [ ] Reuse resize plans and typed idle scratch, preserving trilinear overwrite and diffusion row initialization.
- [ ] Verify native/public cold-warm equality, budget pressure, failure recovery, isolation, and durable results.
- [ ] Join benchmark support and prepare cold/warm artifacts for coordinator-owned exclusive trials before filing the PR.

## Accounting design

One instance owns retained preparation and idle scratch. Retained entries share both approved caps:
128 entries and `min(256 MiB, memoryLimitBytes / 4)`. Idle scratch counts against total memory separately.
Whole-call preflight includes cache metadata, pinned preparation, pending misses, active scratch, buffers, and boundary records.
Pressure drops idle scratch first, then unpinned published LRU entries. Capacity moves between owners without double counting.
Misses publish only after final boundary completion succeeds. Expected failure drops pending owners and active scratch.

Prepared palette identity uses the retained ordered prefix, truncation flag, alpha policy, and matching policy.
Resize identity includes source dimensions and complete output settings. Neither includes image bytes or execution policy.
Validation still runs before hits, preserving palette-tail validation and error ordering.

The literal baseline models capacity; it is not yet safe physical allocation enforcement or public reuse.
Actual integration must reconcile quantizer inline capacity with resize's separately counted inline record.
Diffusion quantizer ownership must separate from width-dependent scratch. Trilinear image storage is scratch, never a cached image.

## Validation and ownership

Only this worktree's ordinary `target/compiler` is assigned for native/Wasm compiler outputs.
Scalar/thread package profiles will use explicitly assigned children here. No shared old cache is used.
No measurements run until the coordinator drains all agents and grants the exclusive phase.
Compiler cleanup follows the PRD after completed-PR handoff; retained evidence survives cleanup.
Gauss owns benchmark protocol in a separate worktree. The coordinator owns issues, ledger, final joins, and trial selection.

## Literal checkpoint evidence

`prod/contract/cache.rs` is byte-identical to frozen `spec/contract/cache.rs`.
Its relative imports resolve to existing production contracts. No production pipeline calls this model yet.
The complete copy retains the reference's identity vocabulary for this baseline; S31 activates preparation keys only.

`prod_preparation_baseline` passes 10 tests. Nine replay frozen capacity/lifecycle fixtures unchanged.
The additional test compares 450 prepared-palette keys and 126 resize-plan keys against frozen implementations.
It covers all 15 matching policies, alpha threshold precision/signed zero, palette order/duplicates/truncation, seven filters, and nine anchors.
The existing frozen cache suite passes all 16 tests in the same native run.
These results establish modeled behavior only, not actual allocation enforcement or live cache reuse.
