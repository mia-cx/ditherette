# S34 threaded initialization reuse inventory

Issue #76 depends on validated S33. This inventory prepares its handoff, not implementation availability.
Read the approved execution contract, S34 slice, and frozen `spec/contract/thread_pool.md`, `thread_pool.rs`, and lifecycle initialization rules before implementation.

## Existing initialization

The package factory validates `threads`, `memoryLimitBytes`, and custom `wasm` inputs already.
`disabled` and `preferred` currently use scalar; `required` returns a capability error.
The root import remains inert. `createScalar` loads its glue factory lazily and creates one independent closure per instance.
Response and Request inputs are cloned. Buffer views preserve their offsets and lengths.
Keep these ownership guarantees when sharing processor wrapping between scalar and threaded bindings.

`crates/ditherette-wasm/scripts/scalar-factory.mjs` already parses pinned generated glue with TypeScript's AST.
It preserves static imports and wraps mutable binding state in a fresh closure. Unsupported export syntax fails generation.
Extend this existing generator where suitable rather than creating a second text-replacement glue transform.
The build script currently generates the factory only for scalar. Package staging already includes both artifact directories.

## Existing threads and missing ownership

The crate already enables Rayon and `wasm-bindgen-rayon/no-bundler` behind its threads feature.
It exports `init_thread_pool`; the existing build uses atomics, bulk memory, rebuilt standard library, and a 2 GiB memory maximum.
Threaded scheduling belongs to S35-S37. S34 establishes actual pool initialization and teardown without replacing scalar kernels.

Inspect the pinned `wasm-bindgen-rayon` 1.3.0 source in the local Cargo registry before designing its adapter.
Its no-bundler helper imports `builder.mainJS()`, initializes the module with shared memory, and starts the received Rayon worker.
The helper retains a module-global worker array only after all ready promises resolve.
It does not expose per-instance teardown or partial-startup error cleanup. Reusing it unchanged cannot satisfy the frozen ownership contract.
The package must own each worker immediately and clean partial starts before preferred fallback or required failure.
Keep builder lifetime valid until workers start or initialization fails; inspect the pinned Rust builder implementation before changing this boundary.

The pinned builder passes a pointer to its own channel receiver to each worker and uses `build_global()`.
Its Rust safety comment requires the builder to remain alive until all workers run. Cleanup must respect that lifetime.
The global Rayon pool is therefore per Wasm memory, not a reusable pool shared by separately budgeted processors.

`builder.mainJS()` resolves `import.meta.url` from generated bindings. A closure-factory file exports a factory, not the original default initializer.
Do not point the stock worker helper at that file and expect `pkg.default` to exist.
Use an explicit worker bootstrap that initializes the matching bindings with the supplied module and shared memory.
The retained S32 threaded glue already accepts object-form `module_or_path`, `memory`, and `thread_stack_size`.

Each processor needs independent glue state and module memory. A shared compiled `WebAssembly.Module` is code, not shared instance state.
Its pool workers share only that processor's memory. Do not place independently budgeted processors in one shared memory accidentally.
Private processor state uses `thread_local!`, and the busy slot releases its mutable borrow before JavaScript calls.
Preserve S33 callback and disposal guards while adding pool cleanup to the same instance lifetime.

## Required evidence

Prove disabled imports fetch no threaded assets, preferred fallback releases partial workers first, and required failures remain structured.
Test two instances, custom Wasm inputs, startup failure, idempotent disposal, and host processing-worker termination.
Observe actual browser worker cleanup, not merely a mocked worker counter. No external consumer project is required.
Use the frozen thread-pool model as the naive contract before implementing production ownership.
Keep generated assets reproducible, and include startup measurements only after root quiet clearance.
No release, deployment, or rollout activation belongs to this slice.
