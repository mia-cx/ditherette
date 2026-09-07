# S20 typed browser protocol

This subtask extends the existing fresh-pair protocol from base `86692c27fd525d2d439aefb96b277dfc6d511680`.
The coordinator owns the aggregate S20 plan and measurement clearance.

## TODOs

- [x] Add typed browser operations, complete artifact/runtime identity, and validated coordinator dispatch while preserving native JSON.
- [x] Prove exact comparison, absent/tampered identity rejection, zero/coarse timer behavior, and native compatibility with controlled fixtures.

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

## Final control validation

Browser workers attach `TrialRequest.reference_output` before Node starts any timing.
A preflight mismatch returns exact public output with `timing_skipped: "reference-mismatch"`, empty samples, and zero work counts.
The worker preserves mismatch evidence through S05 and rejects it as timed evidence.
The coordinator never supplies this reference itself. Native workers reject supplied reference claims.

The fake Node worker fixture checks browser dispatch and validation before and after every child.
It covers failures before spawn, after reap, and during child execution. Each failure releases the lease.
Its intentionally absent browser proof cannot pass comparison. No actual browser or benchmark timing runs.
Mixed fast/slow pairs remain inconclusive without incorrectly blaming timer resolution.

Validation commands all pass:

- `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --test paired --test paired_browser --test verification --test verification_adapters` passes 28 tests.
- `cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --examples`.
- `cargo fmt --manifest-path crates/ditherette-bench/Cargo.toml --all --check` and `git diff --check`.
- `node /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze/tools/spec-freeze/guard.mjs --root /home/mia/mia-cx/ditherette/.worktrees/v1-s20-protocol --trusted-root /home/mia/mia-cx/ditherette/.worktrees/v1-s18-freeze` passes all native/Wasm isolation checks.

The guard reports frozen revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` and artifact `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Asset filesystem validation, actual Node transport integration, and authorized live trials remain with their assigned owners.
