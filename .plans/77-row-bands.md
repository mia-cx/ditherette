# S35 resize and color row bands

## Scope and ancestry

Issue #77. Branch `impl/v1-s35-resize`, worktree `v1-s35-resize`.
Initial parent is S34 capability checkpoint `6296c66babdbef8967f768108988899994396f03`.
The coordinator explicitly authorizes this bounded start while S34 finishes loader error handling and cleanup.
Join its final validated head before S35 delivery or artifact preparation.

Own production resize/color row adapters and policies, focused native tests, and S35 benchmark subjects.
Preserve scalar kernels, packed nearest, exact integer area, accepted fractional arithmetic, and shared convolution and mip plans.
Public color continues to use packed triplets with byte alpha. Direct quantization gains no full-image color plane.

## TODOs

- [ ] Expose allocation-free caller-scratch row adapters. Check exact split output and reject insufficient scratch before writes.
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
