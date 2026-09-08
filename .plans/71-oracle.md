# S30 frozen-only Process oracle

Issue #71. This bounded subtask adds the complete frozen Process request to the existing independent Wasm oracle.
Production process, public/native benchmark registration, and timed comparisons remain separate work.

## Ownership and base

Branch `impl/v1-s30-oracle` starts from validated S30 join `22b6dd78a6e552596c34aa9e693ad74850426b23`.
Only the oracle library, its focused integration tests, and this plan change.
The oracle continues to import frozen spec/image modules and shared identity code, never production semantics.
All builds use this worktree's ordinary `target/compiler` directory. No sibling or shared compiler cache is used.

## TODOs

- [~] Add failing Process wire/identity tests against the existing frozen native reference, then add typed frozen-only dispatch.
- [ ] Verify complete recipe identities, changed output dimensions, indexed metadata, invalid inputs, and rejection of caller reference overrides.
- [ ] Run focused native tests and isolated Wasm compilation, record limits, commit/push, and drain owned jobs.

## Contract

Wire shape is `Process { settings: { palette, recipe: RecipeV1 } }`.
The recipe owns output dimensions; the enclosing declared output must agree before execution.
Identity reuses the established native tuple `("process", palette, recipe)` and the matching-space semantic tag.
Tests obtain expected output from the frozen native Process adapter, not hand-written pixel bytes.
Same-browser conformance and retained cross-target diagnostics remain the later integration's obligation.
No tolerance changes, frozen changes, production calls, measurement workers, or PR/issue writes occur here.

## Cleanup

Retain source and evidence. Clean rebuildable compiler output after the completed handoff once no active slice owns it.
Coordinate ownership with root before cleanup; the current subtask has not completed the S30 PR.
