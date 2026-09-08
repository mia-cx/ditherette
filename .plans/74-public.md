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

- [x] Share the existing standalone installed browser setup without changing its ownership checks.
- [x] Add callback behavior across five methods, cold/repeated calls, caught failures,
  output durability, reentry/disposal, and observable completion/copy ordering.
- [x] Check syntax and run an explicitly failing S32 callback probe, then hand off
  the test-only checkpoint for actual S33 candidate validation.

No compiler targets or build ownership. No measurements, PR creation, or public comments.
Baseline tarball lives at the S32 worktree's
`target/s32-candidate-d638f87c-public/ditherette.tgz`, SHA-256
`8a51e04d08cfdf73bd022ccef1167fed36c74f8267ea1615e19e46df7636fc80`.

The extracted `installed-browser.mjs` keeps tarball digest checks, offline install,
restricted asset serving, three engines, and cleanup. Unchanged stage ownership
assertions pass all four tests on the retained S32 tarball in 2.51 seconds.
Local dependency installation used the frozen lockfile, offline mode, and disabled scripts.

`progress-browser-fixture.mjs` covers five methods with cold and repeated requests.
It checks typed stages, optional integer counts, count bounds and monotonicity,
completion as the final synchronous callback, and unchanged result bytes/metadata.
A frozen browser clock checks that stage transitions do not wait for 50 ms.
A second quantize fixture advances the clock by 20 ms per read and checks observed
same-stage spacing without imposing a row-band schedule or requiring a slow call.

Each method gets intermediate, cold-completion, and warm-completion callback
failures, followed by ordinary recovery. One callback catches attempts to invoke
every method and disposal recursively, while a separate instance remains usable.
Final RGBA/indexed copy failures must precede completion. Earlier results survive
later callbacks and disposal. The broad tarball runner also registers this fixture.

Syntax checks pass for the new fixture/runner and shared helper. Prettier and
`git diff --check` pass. Against the digest-bound S32 tarball, the new standalone
test fails in all three engines at the first callback-enabled resize with
`Progress callbacks are not implemented in this package checkpoint.` This is the
expected red baseline, not S33 validation. The test process exits 1, with four
failed tests including the parent; no further callback assertions execute on S32.

The runtime owner must join the fixture commits, build its actual S33 package,
and run `node --test packages/ditherette/tests/progress-browser.test.mjs` with
`DITHERETTE_TEST_TARBALL`, its SHA-256, and the documented WebKit executable alias.
Then run the broad installed suite. Native/private tests must independently prove
real counts and zero new cache publication on callback failure. This worktree has
no compiler outputs or running jobs; only ignored package dependencies remain.
