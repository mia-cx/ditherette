# S30 frozen-only Process oracle

Issue #71. This bounded subtask adds the complete frozen Process request to the existing independent Wasm oracle.
Production process, public/native benchmark registration, and timed comparisons remain separate work.

## Ownership and base

Branch `impl/v1-s30-oracle` starts from validated S30 join `22b6dd78a6e552596c34aa9e693ad74850426b23`.
Only the oracle library, its focused integration tests, and this plan change.
The oracle continues to import frozen spec/image modules and shared identity code, never production semantics.
All builds use this worktree's ordinary `target/compiler` directory. No sibling or shared compiler cache is used.

## TODOs

- [x] Add failing Process wire/identity tests against the existing frozen native reference, then add typed frozen-only dispatch.
- [x] Verify complete recipe identities, changed output dimensions, indexed metadata, invalid inputs, and rejection of caller reference overrides.
- [ ] Run focused native tests and isolated Wasm compilation, record limits, commit/push, and drain owned jobs.

## Contract

Wire shape is `Process { settings: { palette, recipe: RecipeV1 } }`.
The recipe owns output dimensions; the enclosing declared output must agree before execution.
Identity reuses the established native tuple `("process", palette, recipe)` and the matching-space semantic tag.
Tests obtain expected output from the frozen native Process adapter, not hand-written pixel bytes.
Same-browser conformance and retained cross-target diagnostics remain the later integration's obligation.
No tolerance changes, frozen changes, production calls, measurement workers, or PR/issue writes occur here.

The first regression fails on the absent `process` enum variant, then passes with typed dispatch.
The second fails because envelope output was ignored, then passes with a Process-only dimensions check.
All five focused oracle tests pass after these changes. Every new expected output comes from the existing native frozen adapter.

The expanded suite passes all eight tests. It includes 130 combinations of ten resize recipes and thirteen dither recipes.
Those cover every resize algorithm, both convolution supports, all separable fields, four diffusion kernels with both feedback modes, and Yliluoma.
Alpha modes rotate across the matrix; separate 257-entry palettes preserve truncation, transparent-only, and fallback metadata.
Identity witnesses cover all Process recipe groups, f32 control normalization, f64 alpha precision, complete palette tails, source bytes/dimensions, and output dimensions.
Invalid settings/source failures retain native frozen error ordering. Caller-supplied reference outputs are rejected, never used as an oracle override.

## Cleanup

Retain source and evidence. Clean rebuildable compiler output after the completed handoff once no active slice owns it.
Coordinate ownership with root before cleanup; the current subtask has not completed the S30 PR.
