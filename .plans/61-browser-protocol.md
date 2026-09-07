# S20 typed browser protocol

This subtask extends the existing fresh-pair protocol from base `86692c27fd525d2d439aefb96b277dfc6d511680`.
The coordinator owns the aggregate S20 plan and measurement clearance.

## TODOs

- [x] Add typed browser operations, complete artifact/runtime identity, and validated coordinator dispatch while preserving native JSON.
- [~] Prove exact comparison, absent/tampered identity rejection, zero/coarse timer behavior, and native compatibility with controlled fixtures.

## Agreed seams

`paired/browser.rs` owns the single Rust protocol shared by the worker and Node transport.
The initial nearest operation uses the existing serializable production Anchor type.
Browser cases declare package/TypeScript roles, fresh/primed instance preparation, or initialization from bytes/precompiled modules.
S19 reports no application cache. Cold/warm claims, fresh-instance throughput, and TypeScript initialization are rejected.
Later method/cache slices extend explicit tags instead of embedding untyped settings.

Asset and runtime identities include complete sorted file inventories, entrypoints, binary/tool versions, launch options, and isolation.
Absolute snapshot locations are not content identities. Output records bind the worker, served closure, and runtime together.
The worker adds frozen-reference output through the existing S05 verifier. There is no second comparison engine.
Browser latency retains genuine zero timings. Timer-quantum uncertainty must resolve the regression threshold before a pass or regression is claimed.

The artifact owner supplies filesystem validation through `coordinator::run_with_browser`.
The callback runs before and after every browser child. Legacy `run` rejects browser preparations, so it cannot skip validation.
Native optional browser fields default to absent during deserialization.

No real measurements run during this subtask. All builds and controlled tests must exit before the coordinator's quiet phase.

## First checkpoint

`cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --lib` compiles the shared types.
`cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --test paired --test paired_browser` passes ten tests.
The browser fixtures reject missing runtime/assets, unsupported TypeScript requests, fake cache claims, invalid entrypoints, and native/browser confusion.
Zero and coarse positive timer medians remain inconclusive without changing raw samples.
The historical native JSON omits the additive fields and still compares successfully.
The native fixture continues checking alternating lease-owned children, malformed output, failures, and artifact tampering.

The native control-plan compatibility line comes from the artifact owner's `2f667661` commit.
The protocol agent changes only native case/result defaults and explicit rejection of browser requests in the native worker.
