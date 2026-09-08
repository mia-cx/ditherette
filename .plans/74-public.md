# S33 installed callback fixtures

Issue #74. Branch `impl/v1-s33-public` starts at validated S32
`59b5a004c98c4cff255b30f51025d8ec8143786d`.
This worktree owns installed browser callback fixtures and their serving glue.
The runtime owner owns package source, private/native checks, and candidate builds.

## Contract and seam

Read the approved S33 slice, execution contract, resolved issue #24, and coordinator
`.plans/74-reuse-inventory.md`. Test ordinary methods of the installed tarball.
The frozen `spec/contract/lifecycle.rs` defines stages, optional measurable counts,
immediate stage transitions, the 50 ms gate, and completion/failure ordering.

Callback errors use `callback`, `onProgress`, and `Progress callback threw.`.
Reentry and disposal use existing `reentrant-call` / `instance` errors.
Control only the browser clock and final typed-array copy boundary in focused probes.
Keep runtime stages and work-band counts flexible. Public equality and recovery do
not prove private cache publication; the runtime owner supplies that evidence.

## Work

- [ ] Share the existing standalone installed browser setup without changing its ownership checks.
- [ ] Add callback behavior across five methods, cold/repeated calls, caught failures,
  output durability, reentry/disposal, and observable completion/copy ordering.
- [ ] Check syntax and run an explicitly failing S32 callback probe, then hand off
  the test-only checkpoint for actual S33 candidate validation.

No compiler targets or build ownership. No measurements, PR creation, or public comments.
Baseline tarball lives at the S32 worktree's
`target/s32-candidate-d638f87c-public/ditherette.tgz`, SHA-256
`8a51e04d08cfdf73bd022ccef1167fed36c74f8267ea1615e19e46df7636fc80`.
