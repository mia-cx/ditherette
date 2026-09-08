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

1. [ ] Copy the missing frozen thread-pool contract into production and verify its baseline.
2. [ ] Extend the existing AST factory and add isolated pool/bootstrap ownership.
   Prove failed startup releases acquired workers and retains valid builder lifetime.
3. [ ] Wire typed capability selection, custom inputs, fallback, and guarded teardown.
   Validate real isolated threaded instances while preserving scalar behavior.
4. [ ] Join installed fixtures and S33 report ancestry. Build exact artifacts once coordinated.
   Verify actual browser cleanup, errors, imports, custom inputs, and callback recovery.
5. [ ] Record exclusive startup evidence and file the unmerged child PR after root handoff.

No measurements run during implementation. Routine reviews remain deferred until
the complete stack exists. Frozen specs and landed image arithmetic stay unchanged.
