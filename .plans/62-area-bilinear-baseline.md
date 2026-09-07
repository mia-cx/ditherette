# S21 literal area and bilinear baseline

Historical evidence only. Mia explicitly supersedes this replacement approach in #108.
Restoration `23f6e4f5b9bb6cc1110322b83b8538ffd6dd4508` returns every crate file to the pre-baseline bytes.
Keep the landed area/bilinear kernels and shared helpers in production. Their package integration remains S21 work.
The copy manifest below describes the historical baseline commit, not the current source tree.

Base is `711c7aec61587b45a91c2e404583161edb1e0de9`, containing S19 `7de86d799a25a132c8de41ee54696bd8e54bdf76` and S20 PR107.
This bounded phase establishes the copied baseline only. Main owns full S21 acceptance and its PR.

## TODOs

- [x] Preserve inherited candidates, copy frozen kernels/helpers, and verify exact conformance in a separate baseline commit.
- [x] Record copy hashes, validation, and shared integration fragments.
- [x] Push the clean checkpoint and drain.

Literal baseline commit `88b3162900f547837a596b49f0b0199c32b42cda` is pushed to `impl/v1-s21-area-bilinear`.
All owned validation and push processes exited. No implementation continues during the coordinator's exclusive diagnostic.

## Boundaries

Copy `spec/resize/scalar/{area,bilinear}.rs` to mirrored prod paths using only `spec::resize` to `prod::resize` import changes.
Copy common coordinates/sample without changing any byte. Leave common alignment untouched.
Preserve inherited implementations under area_candidate/bilinear_candidate and redirect legacy callers mechanically.
Keep copied prod and legacy candidate subject identities separate. Run no measurements or timing tests.

The copied kernels retain per-pixel allocations. Fallible public execution, wrapper exposure, benchmark integration,
and later measured improvements remain coordinator-owned follow-up work after this verified baseline.

## Copy evidence

`62-literal-copy.json` records every source and destination SHA256, including the twelve unchanged candidate files.
The copy check compares each source against frozen checkpoint `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
It permits only the recorded kernel import substitutions. Both common helpers remain byte-identical.
Candidate checks compare each moved file against the base revision above.

## Validation

Seven focused native tests pass with `bench-subjects` enabled. Three new tests check the copied kernels exactly.
They cover eleven shapes, nine bilinear anchors, independent strides, RGBA8, palette indices, and packed f32 coordinates.
RGBA fixtures include transparent hidden RGB and fractional alpha. Float comparisons use exact bits.
Independent average fixtures verify byte rounding. Four inherited candidate tests also pass without changing their historical tolerances.

`cargo check --locked` passes for both crates, including the Wasm crate's `bench-subjects` feature.
`cargo fmt --check` passes for both crates. No benchmark or timing test ran.
The separate S18 checkout's trusted guard passes with frozen digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

## Integration handoff

Main reconciles shared edits in the scalar/common barrels, `wasm.rs`, `bench_subjects.rs`, and `tiling_sweep.rs` with S22.
Legacy Wasm imports and row-band sweeps explicitly select candidate modules.
Canonical `prod:resize:{area,bilinear}:scalar` subjects call the literal copies.
New `candidate:resize:{area,bilinear}:legacy` subjects retain inherited implementations without promotion.
Row-band sweeps reject canonical copied IDs because those copies have no row-band adapter.

Public processor/package support, fallible allocation, measured candidate selection, and full S21 benchmark acceptance remain open.
This checkpoint is a literal baseline, not a claim that S21 is complete.
