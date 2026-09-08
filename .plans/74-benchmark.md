# S33 public progress overhead

Issue #74. Start from validated S32 `59b5a004c98c4cff255b30f51025d8ec8143786d` in `impl/v1-s33-bench`.
Read the approved execution contract and S33 slice before implementation.
Runtime ownership belongs to `impl/v1-s33-progress`; installed callback fixtures belong to `impl/v1-s33-public`.
This worktree owns development benchmark protocol, adapters, generator, focused protocol tests, and this plan.

## Reuse and comparison

Use the existing complete-call browser adapter, fresh-instance lifecycle, target-local frozen oracle, and paired coordinator.
The public request already declares `onProgress`. Functions cannot appear in benchmark JSON.
Add explicit optional development metadata that selects callback-disabled or callback-enabled execution for each role.
Historical records omit that metadata and keep their existing behavior. It is not a public backend or cache option.

Run two comparisons for all five methods in Chromium, Firefox, and WebKit:

1. Fresh S32 versus S33, both callbacks disabled. This measures the cost of adding progress support.
2. The same fresh S33 artifact in both roles, callbacks disabled versus enabled. This measures callback delivery overhead.

Reuse S32's cold Lanczos3 resize, Lab76 quantize, separable Bayer4 fused call, and resized diffusion Process workloads.
Add standalone perturb using the exact separable workload's field, input, and dimensions.
Use two alternating role pairs, 20 single-call samples, 50 ms warmup, and a 10-second cap.
The fixed matrix totals 120 serial workers. No warm-cache timing is added; functional fixtures cover callback behavior on hits.

## Callback observation

The enabled callback does constant bounded work, such as updating a count and last event.
Create it before timing. Actual callback dispatch and this ordinary callback work stay inside the timed public method.
Reset observation before each measured call without leaking state from warmup or preflight.
Check event validity and final completion outside timing. Preserve concrete failure evidence if any call violates the protocol.
Keep all existing output, input-identity, and frozen-reference verification outside timers.
Both roles must return identical output bytes and metadata.

The untimed staged Process comparison must not accidentally change the declared callback role or compare combined progress sequences.
Inspect that adapter branch explicitly. Keep callback evidence distinct from output semantic identity.

## Atomic steps

- [ ] Extend typed development metadata and adapter wiring. Prove omitted historical metadata retains existing behavior.
- [x] Add the constant-storage callback observer and focused reset/invalid-event checks.
- [ ] Add focused fake-clock/full-trial tests for actual enabled callbacks, observation reset, and thrown callback propagation.
- [ ] Generate the fixed matrix from reused fixtures. Validate identities and exact frozen outputs without timing.
- [ ] Join final runtime and public fixtures, then build fresh accepted/candidate artifacts with the official preparers.
- [ ] After root quiet clearance, run the fixed comparison and record every per-case gate.

Only root runs measurements. All agents, builds, and tests must be drained first.
One worker and its owned transport may run at a time under the existing shared lease.
Confirmed regressions above 10% stay release blockers; inconclusive results stay incomplete.
Do not retune landed kernels, change frozen files, retry undeclared cases, or run routine PR reviews.
After PR handoff, return compiler ownership for cleanup while retaining artifacts and results.

The initial observer passes four Node tests. It retains only counters and scalar fields, never an event array.
Its bounded callback checks are part of enabled-call timing. Reset and verification belong outside timers.
Full adapter integration and measurements remain pending.
