# #45 Exclusive benchmark execution

Base: `a213effed4b426c5c432c9ccc7062b7016dd5c1b`, S01 PR #75.
Branch: `impl/v1-s04-bench-lock`. PR base: `impl/v1-s01-anchor`.

## Acceptance criteria

- [x] Host-wide OS lease covers each benchmark and its transport lifetime.
- [x] Contention fails clearly; errors and interruption clean up owned children.
- [x] Quiet-phase protocol drains implementation work before measurement.
- [x] Short subprocess fixtures verify contention and cleanup without measurements.
- [x] Open an unmerged, non-draft PR with validation and dependency evidence.

## TODOs

- [x] Add the shared lease and external coordinator, with focused lifecycle tests.
- [x] Guard all measuring entry points and close transport resources on every exit.
- [x] Document the quiet-phase protocol and record final validation.

## Decisions

Unix measurement support uses libc and fails closed on unsupported platforms.
The lease path is fixed across worktrees and ignores temporary-directory overrides.
An inherited lease does not replace the separate exclusive execution guard.
The coordinator attests quietness; the lock cannot detect unrelated host work.
No benchmark timings run in this slice.

First TODO validation: `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --test lease` passed the process lifecycle fixture. It covers contention from another working directory, inherited execution rejection, sequential reuse, failed spawn, and SIGTERM cleanup. The ignored test is a subprocess fixture, not a skipped assertion.

Second TODO validation: `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --test lease` passed. The Rust parser fixture proves malformed JSONL reaps its child. Three Node fixtures prove cleanup after success, browser launch failure, and runtime failure. A detached fake browser exits before the interrupted owner releases its lease. Review caught stale child-handle ownership; the A-wait/B-spawn/A-drop fixture now protects it. Unsupported platforms return invocation errors without blocking compilation.

Final checks at code head `7e501bf`: the focused tests passed without warnings; `cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --benches`, Rust formatting, Node syntax checks for all three changed scripts, and `git diff --check` passed. No benchmark measurements or real browser launches ran. `crates/ditherette-bench/EXECUTION.md` owns the quiet-phase protocol and S06 coordinator API.

Delivered PR: https://github.com/mia-cx/ditherette/pull/90, open and non-draft,
with auto-merge disabled. Base remains `impl/v1-s01-anchor` at
`a213effed4b426c5c432c9ccc7062b7016dd5c1b`. Rebase was up to date.
Head `15ac134` includes documentation and formatting after the validated code
checkpoint. This final entry records delivery only. Issue #45 remains open.

## Restack and amendment verification, 2026-09-12

The historical entries above describe the original `impl/v1-s01-anchor` base.
The branch was restacked onto merged main `cc08a0cc` through merge commit
`523070ca`, which resolved a single conflict in
`crates/ditherette-bench/src/wasm_resize.rs` by retaining both test modules.

At `523070ca`, `cargo +1.97.0 test --locked --manifest-path
crates/ditherette-bench/Cargo.toml --bins --test lease -- --test-threads=1`
passes 7 binary tests, the cross-process lease lifecycle test, and the 3 Node
transport fixtures it invokes under the inherited lease. `cargo +1.97.0 check
--locked --manifest-path crates/ditherette-bench/Cargo.toml --benches` and
`cargo +1.97.0 fmt --manifest-path crates/ditherette-bench/Cargo.toml --check`
pass.

The subsequent amendment adds two verified corrections. First, `Lease::spawn`
now serializes check/launch/register under `SPAWN_LOCK`; the shared-reference
regression `concurrent_spawn_keeps_one_child` in `tests/lease.rs` failed before
the fix with two admitted children and passes afterward inside the existing
lifecycle test. Second, `bench:prepare` stages `ditherette-bench`,
`ditherette-bench-lease`, and `crit_spec_nearest` into
`target/prepared/`; every `bench:*` measurement alias now launches the prepared
helper with `--quiet` and never builds. The new Node regression
`measurement aliases launch prepared executables under the coordinator` failed
on the old cargo aliases and passes on the updated manifest. `node --test
scripts/benchmark-wasm-resize.test.mjs` passes all 16 deterministic tests.
`node --check` passes for `prepare-benchmarks.mjs` and
`benchmark-wasm-resize.mjs`; `node scripts/prepare-benchmarks.mjs` built and
staged all three executables; `git diff --check` is clean.

A third correction was found during this verification: forwarding `--help`
through the harness reached the Node transport's stdout, which the JSONL reader
rejected with exit code 5. The new fixture `tests/help.rs`
(`wasm_help_uses_owned_text_transport_without_measurement`) failed with that
exact JSONL error before the fix. `wasm_resize_command` now intercepts
`--help`, spawns the harness through the owned lease with inherited stdio, and
returns its exit status; `run()` in `main.rs` exempts `wasm-resize --help` from
`require_quiet` while every other measurement command keeps it. After the fix
the test passes, `pnpm bench:resize:wasm --help` and
`pnpm bench:color:wasm --help` both exit 0 and print the full help including
the Preparation block, and `cargo +1.97.0 fmt --manifest-path
crates/ditherette-bench/Cargo.toml --check` and `git diff --check` remain
clean. All results describe the uncommitted working tree after `523070ca`. No
benchmark timing workload or real browser launch ran.
