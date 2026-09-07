# #59 Freeze the complete reference

## Summary

Freeze the validated S17 reference and its shared image dependency closure.
Enforce content identity, independent compilation, and trusted-base validation.

## Acceptance criteria

- [x] Record the fixed S17 commit, tree IDs, content hashes, inventory, and blue-noise provenance.
- [x] Reject frozen changes and direct or indirect semantic imports in both directions.
- [x] Bind reference dependencies, features, and Rust 1.97 without blocking unrelated S02/S06 integration.
- [ ] Demonstrate controlled mutations fail and temporary trees are restored.
- [ ] Push implementation for coordinator review and the unmerged S18 PR.

## TODOs

- [x] Capture the immutable checkpoint and implement content validation with a shared-dependency audit.
- [x] Add Rust syntax checks, independent compilation, and resolved dependency validation.
- [~] Add trusted-base CI wiring and focused temporary mutation tests.
- [ ] Record final checks and hand off the pushed implementation.

## Notes

- Base/checkpoint: `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, branch `impl/v1-s17-processor`.
- Worktree/branch: `.worktrees/v1-s18-freeze`, `impl/v1-s18-freeze`.
- The coordinator owns `slices.md` and `stack-ledger.md`. Preserve its existing uncommitted slice status update.
- No semantic edits, benchmarks, PR creation, merges, tags, deployment, or publication.
- Read the approved PRD and resolutions in #40. Rechecked SHA-256 cache and thread-pool models in final S17.
- Content validation passed. Frozen SHA-256: `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
- `node tools/spec-freeze/guard.mjs` passed content, compiler, syntax/profile checks, four dependency contexts, native/Wasm spec/prod isolation, and native threaded prod isolation.
- Initial syntax allowlists rejected Serde attributes. Replaced general allowlists with narrow source-injection restrictions; ordinary derives and local macros remain available.
