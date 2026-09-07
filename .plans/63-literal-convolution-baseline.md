# S22 literal convolution baseline

Issue #63 starts from `711c7aec61587b45a91c2e404583161edb1e0de9` on `impl/v1-s22-convolution`.
The immediate PR base is `impl/v1-s20-browser-bench`; the coordinator owns PR filing and aggregate progress.

## Bounded deliverable

Copy frozen bicubic, Lanczos, and convolution into canonical production paths with mechanical import substitutions only.
Preserve inherited optimizations under explicit candidate paths. Keep their tests and legacy callers pointed at those candidates.
Shared coordinates/sample copies must remain byte-identical to the frozen files. Existing common alignment stays unchanged.

## TODOs

- [x] Commit literal kernels, preserved candidates, mechanical caller redirects, content manifests, and exact native conformance.
- [x] Record final Wasm compilation, formatting, and trusted freeze validation; push the baseline and drain all owned processes.

## Validation scope

Compare independently executed frozen and production functions for both support policies, all nine anchors, odd and single-axis shapes,
downsampling, padded source/output storage, integer and floating formats, negative lobes, clipping, and rounding.
Conformance must not invoke benchmark timing code. A recorded literal commit is historical evidence, not a permanent ban on later production optimization.

Public processor/package integration, benchmark registration of canonical baselines, bounded optimization, native/public timing,
and the complete S22 PR remain coordinator-owned follow-up work. No optimization or measurement is authorized in this phase.

## Literal checkpoint evidence

The JSON manifest records full hashes for six frozen semantic files, including unchanged common alignment, and ten inherited candidate files.
Validation reads frozen source directly from `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
Each canonical target equals those bytes after only `spec::resize` to `prod::resize` import substitution.
Each inherited candidate equals the source-base bytes after convolution-module redirection and the candidate-status doc line only.
No kernel arithmetic changes. The reference's per-pixel accumulation allocation remains part of this baseline.

`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects --test prod_convolution_baseline --test prod_resize_convolution` passes eight tests.
The new tests execute 9,720 exact comparisons across five storage formats, both support policies, nine anchors, nine shape pairs, and four stride combinations.
Float comparisons include raw bit equality and negative values. A hand-calculated Catmull-Rom step checks negative/overshooting lobes, clipping, and byte rounding.
Source buffers and output padding remain unchanged. Four inherited candidate conformance tests remain separate from baseline acceptance.
An initial generic test compilation required an explicit `ImageFormat + Copy` bound; the corrected test passes without changing kernels.

Mechanical integration redirects affect `bench_subjects.rs`, `wasm.rs`, the retained candidate tests, `tiling_sweep.rs`, and `ditherette-bench.toml`.
Legacy cubic/Lanczos subject prefixes change from `prod:resize:` to `candidate:resize:` with accurate candidate source paths.
Canonical baseline registry/public adapters remain follow-up work. The legacy Wasm exports still call the explicitly preserved candidates.
Sibling S21 adds the same byte-identical coordinates/sample files and common module declarations; its shared registry fragments concern only area/bilinear.

## Final validation

Literal code checkpoint: `97ca0996d67dc0a20a8058cc35bbe1e05515c8a3`.
The following checks pass without running any benchmark or browser:

- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects`.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects`.
- The eight integration tests above, plus `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --lib prod::resize::scalar::lanczos::tests::lobes_follow_known_sine_values_and_finite_support`.
- `cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins` compiles redirected legacy callers without executing them.
- Both crate `cargo fmt --check` commands and `git diff --check`.

`node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-s22-convolution --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze` passes native/Wasm isolation and content checks.
It reports frozen revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` with artifact `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
All owned build/test sessions exit before handoff. This completes the bounded baseline phase, not the full S22 acceptance criteria.
