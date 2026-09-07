# S26 native literal field baseline

Issue #67. Base `0085972a05a3dbdbbef6d47351d6e37bdd8625d2` is the validated all-mode S25 baseline, not its dispatch candidate.

## Bounded deliverable

Copy missing inverse, field, placement, and RGBA8 reconstruction semantics before any optimization.
Keep landed `prod/color.rs`, packed forward conversions, tables, row bands, and all existing pixel kernels.
Only module declarations and a WorkingSpace-to-packed-converter adapter may extend landed color wiring.

The seven original f32 inverse exports and wide f64 field reconstruction remain separate recipes.
Fields keep the global pixel index, original neighborhood, fixed ranges, arithmetic order, clipping, rounding, and byte alpha.
The row callback has no palette parameter. Quantization consumes its completed RGBA8 output.

## TODOs

- [x] Copy and verify the literal missing closure, record fragment hashes, and test inverse/field/placement/composition output against frozen references.
- [x] Record native/Wasm/format/guard evidence for the literal checkpoint.
- [x] Push the clean literal baseline and return ownership before any expanded implementation.

Literal baseline commit: `e156cfbfe0d3dfb598e2f5746bca1ddc46a5f69a`.
That commit is pushed and keeps the S25 validated baseline in its ancestry.

## Copy boundary

Inverse functions and their alpha-copy image adapters come from the seven mirrored `spec/color` modules.
The inverse-only common fragment contains the existing transfer function and D65 constants.
Copy `spec/color/reconstruct.rs` and `spec/dither/placement.rs` with mechanical spec-to-prod imports.
Copy Bayer size/rank/threshold and random index/mixer fragments without unrelated legacy indexed kernels.
Copy `perturb_by_field_rows_into` and FIELD_SCALE without the allocating wrappers or BlueNoise dispatcher.
Reuse `prod::tiling::RowBand`. Forward conversion calls the existing packed Converter, with no copied forward equations.

The [literal manifest](67-literal-manifest.json) records whole-source SHA256 and exact retained-fragment SHA256 values.
Baseline function bodies retain frozen arithmetic and iteration order. New imports and module wiring are not new recipes.

## Deferred scope

Bounded native allocation/preparation, request-level field dispatch, public Processor/Wasm/package integration, and benchmark registration remain unfinished.
No BlueNoise production module or public enablement belongs to this checkpoint; S27 owns that mode.
No optimization or measurement is authorized during this baseline phase.

## Validation

The initial implementation paused for the coordinator's S24 exclusive measurement and resumed only after every worker exited.
All 24 retained fragments match their frozen source verbatim, including constants, f32 inverse dispatch, and field composition.
Both complete copied modules match after only the manifest's listed import replacements.
Verification reads source bytes directly from frozen commit `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, independently of the worktree copy.

`CARGO_TARGET_DIR=/home/mia/mia-cx/ditherette/.worktrees/v1-s24-quantize/crates/ditherette-wasm/target cargo test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test prod_fields` passes all six tests.
The same target passes `cargo check --locked --manifest-path crates/ditherette-wasm/Cargo.toml --target wasm32-unknown-unknown`.
`cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --all --check` and `git diff --check` pass.
The separate trusted S18 guard passes native/Wasm isolation, including threaded production, with the unchanged frozen digest:
`sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

Command: `node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-s26-fields --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze`.

The native fixtures already cover seven inverse/image adapters, wide reconstruction, all Bayer sizes, wrapping random indices,
945 field/space/strength/placement/shape combinations with reversed row scheduling, and 630 complete quantize compositions.
No benchmark, public integration, allocation wrapper, or optimization ran during this phase.

## Proposed next ownership, awaiting assignment

1. Add bounded `prod/pipeline/perturb.rs` and separable composition beside existing quantize ownership.
   Reuse `Allocator`, allocation-free `Failure`, the current field row callback, and `PreparedQuantizer`.
   Preflight source/output capacities for perturb; include the RGBA8 intermediate, indices, palette preparation, and owned records for separable quantization.
   Reserve every required buffer before input copy. Preserve the full original source for adaptive neighborhoods.
   Quantization reads only the clipped, rounded RGBA8 intermediate. Keep cache/reuse optimization out of the first bounded wrapper.
2. Extend the shared Processor lifecycle and add private Wasm perturb/separable adapters in new modules.
   Reuse caught input/result helpers and the private result sink; extract shared quantize parsing only when both paths need it.
   Append stable error-path IDs without renumbering existing IDs. Keep source/field/working-space validation allocation-free before reservations.
   Expose Bayer/random only. Reject BlueNoise until S27 rather than adding a stub kernel.
3. Extend package types, validation, scalar dispatch, and focused private/interface/browser fixtures under one owner.
   Validate exact/one-under budgets, each reservation failure, caught copy/result failures, recovery, reentry, disposal, and durable results.
   The coordinator owns common registries, benchmark registration, measurements, and integration with the selected S25 checkpoint.

No expanded implementation starts until the coordinator assigns these overlapping files and the current integration base.
