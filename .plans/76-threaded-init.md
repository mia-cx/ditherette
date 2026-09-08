# S34 optional threaded initialization and teardown

Issue #76. Worktree `.worktrees/v1-s34-threads`, branch `impl/v1-s34-threads`,
starts from validated S33 `06d9ad0730669dac3008baf848b5eb463689c584`.
Read the approved execution contract, S34 slice, coordinator reuse inventory,
decisions 24/36/37, and frozen lifecycle/thread-pool contracts before implementation.

## Reuse and ownership

Keep the existing five-method processor wrapper, callback guards, kernels, and
shared storage. S34 selects isolated module memory and owns pool resources.
S35-S37 select parallel work later; initializing a pool does not change image math.
The existing AST glue factory isolates mutable bindings. Extend that generator
for threaded bindings and replace only the pinned worker-start import with an
instance-owned callback. Worker bootstrap explicitly creates matching bindings;
it must not call a nonexistent default export on the factory module.

Pinned wasm-bindgen-rayon 1.3.0 passes its builder's receiver address to workers.
Keep the builder alive while any worker can use that address. Rayon 1.13.0's
`build_global` waits for worker priming before returning. Failed partial startup
must terminate every acquired worker before dropping pool ownership or falling back.
Each processor gets fresh glue state and shared memory. Compiled code may be reused,
but separately budgeted processors never share memory, caches, or global Rayon state.
Thread stacks and runtime bookkeeping are fixed module overhead; processing capacity
continues through the existing budget. Do not add public pool sizing or scheduling controls.

Custom inputs retain Response/Request cloning and byte-view offsets. A supplied
artifact must match the selected variant. Preferred fallback retries scalar loading
with the supplied input; incompatible input remains a structured initialization error.
Root imports stay inert. Disabled initialization does not import threaded assets.
Disposal passes the existing idle guard, then releases the owned pool once.
Host termination uses the browser's dedicated-worker ownership tree; installed
fixtures must observe actual descendant cleanup rather than infer it from counters.

This worktree owns package source, private runtime, glue generator/build, native/private
and unit tests, and this plan. The public-fixture agent owns installed test servers
and runners. Root owns startup experiments, issue/stack records, and final acceptance.
Fresh compiler outputs are only `target/compiler`,
`crates/ditherette-wasm/target/scalar`, and `crates/ditherette-wasm/target/threads`.
All three paths are absent at handoff. No old caches or symlinks are reused.

## Atomic steps

1. [x] Copy the missing frozen thread-pool contract into production and verify its baseline.
2. [x] Extend the existing AST factory and add isolated pool/bootstrap ownership.
   Prove failed startup releases acquired workers and retains valid builder lifetime.
3. [x] Wire typed capability selection, custom inputs, fallback, and guarded teardown.
   Validate real isolated threaded instances while preserving scalar behavior.
4. [x] Join installed fixtures and S33 report ancestry. Build exact artifacts once coordinated.
   Record browser cleanup, errors, imports, custom inputs, and callback recovery results.
   Acceptance retains the failures below; completing evidence collection does not waive them.
5. [ ] Record exclusive startup evidence and file the unmerged child PR after root handoff.

No measurements run during implementation. Routine reviews remain deferred until
the complete stack exists. Frozen specs and landed image arithmetic stay unchanged.

The literal production thread-pool model and its mechanically redirected frozen
fixtures pass 6/6 each under the declared local native target. Dependency installation
uses the frozen lockfile. The pool-size calculation will call the existing
`WorkerBudget::from_available_parallelism`, preserving its `clamp(cpus / 2, 1, 8)` policy.

The existing generator now emits threaded factories with a per-instance worker-start
callback instead of the stock helper import. The original generated glue remains intact.
Fresh local scalar and threads builds pass. Generator and built-binding fixtures prove
independent shared memories, callback ownership, and the existing pool-size calculation.

Workers initialize matching factory bindings before receiving the receiver pointer.
Successful Rayon priming permits freeing the builder. If dispatch or build throws,
cleanup terminates acquired workers and a private Rust export forgets the builder.
Its allocation then belongs to the discarded module memory. This avoids a JS finalizer
freeing a receiver while a terminating worker can still borrow it. No failed attempt
reuses that memory. Failures before receiver dispatch free the builder normally.

Twelve factory/pool tests pass, including constructor, readiness, dispatch, and build
failure ordering. The public interface/type suite passes 35/35. An untimed Chromium
check creates two actual required-thread pools, processes exact nearest output,
disposes one twice, and verifies the second remains usable. Installed cross-engine
cleanup evidence is still assigned to the public-fixture worktree.

Final S33 PR #123 at `b2ca677ed9927165a1010f5c52646a989d8a02ca` joins at
`591d4e5b16eae2f85fbe742344b81a2887718c13`. The full native `--locked --tests`
suite passes after this join. Existing private Wasm and package staging tests pass
19/19. Pool unit tests and built threaded binding checks now belong to the existing
interface/glue test commands. Package initialization documentation describes the
selected artifact requirement, isolation, worker CSP, pool sizing, and scalar math.

## Retained engine release gate

The installed fixtures at `1b4bc28cd185b4987e1b251c1ac72dcbdbc16837` and startup
protocol `bf7912db096810bf63ab3cfa39aba20f1d20291d` are joined. Scalar selection
and partial startup cleanup pass in Chromium, Firefox, and WebKit. The partial
failure test observes zero live worker locks before the scalar fallback fetch.
Chromium and Firefox also pass independent pools, five-method scalar equality,
callbacks, nine custom inputs, disposal, and processing-host termination.

WebKit 26.4 retains parked Rayon worker locks after disposal and host termination.
The fixture owner reproduces this with the upstream tiny Wasm atomic-wait module,
without any Ditherette or Rayon import. See [WebKit bug 289686](https://bugs.webkit.org/show_bug.cgi?id=289686)
and its [upstream fix](https://github.com/WebKit/WebKit/commit/03e836de2f7bd5627a95f60357633d59fb6bb18d).
These failures remain an unresolved engine-specific release gate for S41.
The coordinator permits continuing the reviewable implementation stack, not claiming
that WebKit cleanup acceptance passed. Preserve both failing assertions.

Two bounded capability experiments were rejected. A surviving waiter count and
post-wake Wasm sentinel both classify working Chromium like WebKit, despite their
different observed JS worker lifetimes. They do not justify changing the existing
capability contract. No probe, browser blacklist, test skip, or scheduler rewrite
enters production. Experimental browser jobs have drained.

The trusted guard requires private functions under the existing Wasm adapter tree.
Sizing and abandoned-builder functions now live in `wasm/threads.rs`, without
unnecessary crate-root reexports. Frozen source and guard policy remain unchanged.

## Candidate artifact and validation handoff

The trusted guard passes `29bccaa5f60d5aa0172453e0dce9ec2a5ead0c9d` with frozen
checkpoint `cef2b60a` and digest `17ba3be3` unchanged. Interface/type and pool tests
pass 40/40. Generator and built-binding tests pass 7/7. Focused native threaded
contract/progress tests pass 7/7. Subsequent changes are benchmark protocol,
fixture diagnostics, and documentation only.

Official native and public preparation both exit 0 at clean artifact source
`1d1cba8950ab3ffc4064a0a920343e6f92e6a901`. Outputs remain under this worktree:

- `target/s34-candidate-1d1cba89-native`
- `target/s34-candidate-1d1cba89-public`

The preparers verify unchanged source throughout each build. A separate check
verifies all 2,471 recorded input/output hashes before this plan-only update.
The native worker SHA-256 is `36c612d00f3d9636a07a617a88fb50093d80762fac36404239c5cb5e562848fa`.
The native pair executable SHA-256 is `f9689938f14bd4e28d1b783783b25508ea47ea69aa2cde86e3984a8d982ee4c0`.
The public tarball SHA-256 is `01ad17dcf087d1debd7564de9d34acff94805f2034168687ab24e1431412955d`.
The corrected browser transport SHA-256 is `db43e10f603da2cae6a4761b82e96c80f8ebd83e882027a2013ecd1c569d0772`.

The package is byte-identical to the prior official `2afd1802` tarball. The new
source joins benchmark CSP fix `f55f100f` and labels each custom input failure.
The CSP fix permits the existing blob worker bootstrap. Its owner proves actual
required startup fails before the fix and passes afterward against the installed
package. No runtime code changes between these two artifact sources.

Installed validation uses the exact tarball and SHA above, the official public
oracle, and generated frozen Yliluoma vectors. Broad, progress, and stage-cache
suites pass 12/12 on Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Each broad engine check includes 367 frozen vectors and 734 untimed adapter calls.

The final threaded suite reports 24 tests, 19 passes, and 5 failures, including
two failed parent tests. Three engine checks fail:

- WebKit disposal retains all four observed pool locks instead of the remaining two.
- WebKit processing-host termination retains its host lock and both pool locks.
- Chromium rejects initialization during one of nine custom input forms. The
  error is `initialization` at `threads`, with message "Required threaded
  initialization is unavailable." The original fixture does not retain the form.

The isolated Chromium ownership/custom-input diagnostic passes all nine forms
against the same installed package. It logs input constructors and raw failures
through a served-only diagnostic substitution; no runtime or artifact file changes.
It does not reproduce a raw failure or establish a cause. The earlier rejection
remains unexplained, not diagnosed or waived. The final source labels future
custom-input failures with their form, code, and path while rethrowing the original
structured error. Syntax and diff checks pass for this test-only change.

Scalar selection, disabled imports, partial startup cleanup, and ordinary nested
worker cleanup pass in all three engines. Firefox passes ownership and custom
inputs. Chromium and Firefox pass processing-host termination. Both known WebKit
assertions remain enabled. Package-byte equality carries these results to the
replacement artifact; no further broad rerun follows the CSP/test-only join.

All owned builders and browser jobs are drained. Root owns the five immutable
snapshots and exclusive startup trial. No measurements run in this worktree.
Artifact source remains `1d1cba89`; this plan-only handoff is a separate delivery
commit. Keep step 5 pending until root returns measurement/report ownership.
