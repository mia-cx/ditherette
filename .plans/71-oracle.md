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
- [x] Run focused native tests and isolated Wasm compilation, record limits, commit/push, and drain owned jobs.

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

## Validation handoff

Run with `CARGO_TARGET_DIR=/home/mia/mia-cx/ditherette/.worktrees/v1-s30-oracle/target/compiler`:

- `cargo test --locked --manifest-path crates/ditherette-bench/Cargo.toml --test frozen_oracle`: eight passed.
- `cargo build --locked --manifest-path crates/ditherette-bench-oracle/Cargo.toml --lib --features frozen-build --target wasm32-unknown-unknown --release`: passed.
- The oracle's normal dependency tree contains no production/core crate dependency.
- Rustfmt checks of the two owned Rust files and `git diff --check` pass. Frozen spec/image and all production files match the base.

Compiler is rustc 1.97.0, commit `2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`, LLVM 22.1.6.
Retained raw Wasm is `target/oracle-validation/ditherette_bench_oracle.wasm`, SHA-256 `c725b73abe7ba17636da2c4b413979b04f01fe9d28abd78308f79764054556cd`.
Oracle library source SHA-256 is `dd8fcb8dc7674fb1593db2fbae7c97b5286e4711b87d07c130bb4936b50f04b2`.
This is a compilation artifact, not a fresh identified browser oracle bundle or measured role.
No same-browser Process executions, browser transport additions, production adapters, or cross-target equality claims occur in this subtask.
The later joined integration must preserve native diagnostics independently of same-target verification and run browser conformance before measurement.
All owned compiler/test processes drain before handoff. The private compiler directory contains about 1.3 GiB of rebuildable output.

## Cleanup

Retain source and evidence. Clean rebuildable compiler output after the completed handoff once no active slice owns it.
Coordinate ownership with root before cleanup; the current subtask has not completed the S30 PR.
