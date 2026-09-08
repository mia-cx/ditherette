# #81 Website cancellation and faithful fallback

## Scope and ancestry

Worktree `v1-s39-website`, branch `impl/v1-s39-website`.
Join `6230326d98af9c881e243a10258d389152d44000` explicitly merges:

- S34 `29bccaa5f60d5aa0172453e0dce9ec2a5ead0c9d`.
- S38 `0305456bc25259a92d46ded245ae09aaf407be07`.

Own the website client, processing worker, fallback bridge, and focused tests.
Preserve the UI, crop adapter, current strength scaling, and disabled development flag.
Package, frozen references, and kernels remain unchanged.

## TODOs

- [x] Reject stale events immediately and replace active workers after the existing debounce. Preserve the last preview.
- [ ] Forward package progress and contain initialization-only fallback at the page-session boundary.
- [ ] Define faithful TypeScript support from existing semantics and verify fallback and visible processing failures.
- [ ] Join final S34 artifact/report ancestry and validate against its retained package without rebuilding Rust.

## Validation boundaries

Test the public scheduling functions with real stores and a controlled browser Worker boundary.
Test the worker pipeline and transport with existing fixtures. Use the installed package for final browser checks.
Mock timers and external load failures only where needed. Record red before green for each behavior change.

S34's WebKit atomic.wait cleanup failure remains an engine-specific release gate, recorded in issue #83.
Held Web Locks observe JS worker lifetime, not proof that a parked Wasm stack can never resume.
Do not weaken those observers or add capability probes, blacklists, or shutdown redesigns.

No Rust compiler outputs or measurements belong to this worktree. Drain all jobs for the coordinator's exclusive startup trial.

## Scheduling checkpoint

The new client fixture fails before implementation on stale progress and malformed old response IDs.
After the fix, all 4 client tests and 12 existing store tests pass.
The host invalidates request authority before waiting for the debounce. It terminates unfinished work when replacement starts.
Old worker errors and malformed responses cannot alter current progress or error state.
Explicit cancellation keeps the previous preview and clears the worker once.

Command: `pnpm exec vitest run --project server src/lib/processing/client.spec.ts src/lib/stores/app.spec.ts`.
Fallback and progress forwarding remain pending. Nearest fallback needs request-specific center-tie evidence, including the historical 2-to-49 mismatch.
