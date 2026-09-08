# S35 resize and color row bands

## Scope and ancestry

Issue #77. Branch `impl/v1-s35-resize`, worktree `v1-s35-resize`.
Initial parent is S34 capability checkpoint `6296c66babdbef8967f768108988899994396f03`.
The coordinator explicitly authorizes this bounded start while S34 finishes loader error handling and cleanup.
Join its final validated head before S35 delivery or artifact preparation.
Join `92768069fe1a4cbdecdfd79795f12cd08b930a11` includes validated S34 delivery `3863cadce82b4d272a1730f3ebad828b18ac4434`.
Its report-only PR head remains pending.

Own production resize/color row adapters and policies, focused native tests, and S35 benchmark subjects.
Preserve scalar kernels, packed nearest, exact integer area, accepted fractional arithmetic, and shared convolution and mip plans.
Public color continues to use packed triplets with byte alpha. Direct quantization gains no full-image color plane.

## TODOs

- [x] Expose allocation-free area/bilinear caller-scratch row adapters. Check exact split output and reject insufficient scratch before writes.
- [x] Extend shared convolution with caller-owned support-range scratch, including overlapping support across bands.
- [ ] Integrate complete worker/support capacity preflight and disjoint execution through existing tiling models.
- [ ] Extend benchmark subjects for the budgeted path and retain caller-thread progress.
- [ ] Join final S34 and validate actual installed scalar/threaded calls before exclusive crossover evidence.
- [ ] Select only freshly measured exact configurations, or retain scalar, and prepare the unmerged PR.

## Test seams and evidence

The first confirmed seam is the existing production plan-plus-output row adapter, extended with caller-owned scratch.
Compare split execution exactly with the landed full-call kernel, including identity axes, integer scales, and fractional support.
Budget tests use the existing fallible `CapacityBudget` and check output preservation on rejection.
The next seam is the production budgeted executor, not a test-only scheduling implementation.

No policy threshold or threaded default is chosen during this checkpoint.
Callbacks remain on the caller. Diffusion remains scalar. Trilinear retains one shared mip chain.
S34 requires a blocking-wait-capable processing host for threads. Its WebKit cleanup limitation remains a recorded release gate.

Only worktree-local `target/compiler` belongs to this task. No symlinked targets, Wasm builds, or measurements are authorized.
Drain every owned job when the coordinator requests the S34 quiet phase.

## First bounded checkpoint

Area and bilinear reuse their existing absolute-output row loops and fast paths.
The new adapters accept already-reserved f32 slices. They reject undersized scratch before changing output.
No current public dispatch selects these new adapters yet. Full worker-capacity planning remains pending.

Each new adapter fixture first fails to compile because the required caller-scratch entrypoint is absent.
After implementation, two new tests pass with physical allocation counters around execution.
They compare exact landed scalar bytes across integer scales, fractional scales, identity axes, and three representative anchors.
Area tests use band heights 1, 2, 4, and the complete output height. Bilinear tests include an uneven final band.
Caller scratch is reserved through `CapacityBudget`; every tested execution allocates zero heap blocks.
Short scratch returns `MemoryLimit` and preserves sentinel output bytes.

Focused native validation passes 15 tests across `prod_resize_row_scratch`, `prod_resize_area`, `prod_resize_bilinear`,
`prod_resize_preparation`, `prod_process`, and `prod_progress`.
Command uses `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --target-dir target/compiler` with those six test targets.
No Wasm builds, measurements, threshold selection, frozen changes, or color-plane changes occur in this checkpoint.

## Shared convolution checkpoint

The existing x-then-y row kernel accepts caller-owned f64 scratch. Its source-support calculation also supplies preflight lengths.
Bicubic and Lanczos expose that shared adapter; their scalar dispatch and arithmetic remain unchanged.
The new fixture first fails on absent row-scratch APIs. It then passes bicubic/Lanczos2/Lanczos3, both support policies,
three anchors, identity axes, direct filtering, and large x-then-y downscales with band heights 1, 3, and full output.
It proves overlapping bands need more aggregate support scratch than one full-call owner.
Physical allocation counters stay unchanged during execution; short scratch leaves output sentinels intact.
