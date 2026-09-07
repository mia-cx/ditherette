# #45 Exclusive benchmark execution

Base: `a213effed4b426c5c432c9ccc7062b7016dd5c1b`, S01 PR #75.
Branch: `impl/v1-s04-bench-lock`. PR base: `impl/v1-s01-anchor`.

## Acceptance criteria

- [ ] Host-wide OS lease covers each benchmark and its transport lifetime.
- [ ] Contention fails clearly; errors and interruption clean up owned children.
- [ ] Quiet-phase protocol drains implementation work before measurement.
- [ ] Short subprocess fixtures verify contention and cleanup without measurements.
- [ ] Open an unmerged, non-draft PR with validation and dependency evidence.

## TODOs

- [x] Add the shared lease and external coordinator, with focused lifecycle tests.
- [x] Guard all measuring entry points and close transport resources on every exit.
- [ ] Document the quiet-phase protocol and record final validation.

## Decisions

Unix measurement support uses libc and fails closed on unsupported platforms.
The lease path is fixed across worktrees and ignores temporary-directory overrides.
An inherited lease does not replace the separate exclusive execution guard.
The coordinator attests quietness; the lock cannot detect unrelated host work.
No benchmark timings run in this slice.

First TODO validation: `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --test lease` passed the process lifecycle fixture. It covers contention from another working directory, inherited execution rejection, sequential reuse, failed spawn, and SIGTERM cleanup. The ignored test is a subprocess fixture, not a skipped assertion.

Second TODO validation: `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --test lease` passed. The Rust parser fixture proves malformed JSONL reaps its child. Three Node fixtures prove cleanup after success, browser launch failure, and runtime failure. A detached fake browser exits before the interrupted owner releases its lease. Review caught stale child-handle ownership; the A-wait/B-spawn/A-drop fixture now protects it. Unsupported platforms return invocation errors without blocking compilation.
