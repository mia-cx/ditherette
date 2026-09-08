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
- [x] Forward package progress and contain initialization-only fallback at the page-session boundary.
- [x] Define faithful TypeScript support from existing semantics and verify fallback and visible processing failures.
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
This historical scheduling checkpoint leaves fallback and progress forwarding to the later checkpoints below.

## Progress checkpoint

The worker pipeline forwards package stages and optional completed/total counts without adding skipped stages.
The existing progress display uses the current stage's measured fraction, not a whole-call time estimate.
The transport validates counts before updating the existing store. No UI components change.
The new package-progress fixture fails before wiring; all 54 focused pipeline/client/schema/store tests pass afterward.

## Faithful fallback checkpoint

The main-thread client retains the fallback decision for its page session and includes it in replacement-worker requests.
Only package loading or typed initialization/capability failures request fallback. Typed memory, callback, validation, and runtime failures remain errors.
A processing-time error never requests fallback, even when its code is `initialization`.
Existing stale-request guards reject old fallback messages before changing the session decision.
Activation uses the existing progress text and a durable output warning. No backend selector or development-flag activation is added.

Admission is deliberately restricted to nearest resize, sRGB matching, preserve alpha, and no dithering.
Every nearest axis index must agree with the current website floating center formula and the package integer center formula.
The normalized palette must have at most one distinct visible RGB, or every non-thresholded cropped input RGB must appear exactly in that palette.
This avoids unproven f32-versus-f64 matching ties. Area, bilinear, other matching/dither/alpha modes, and unmatched multicolor requests remain visibly unsupported.
Crop packing, palette normalization, and existing TypeScript execution reuse the landed helpers. No resampling or matching kernel changes.

The new client and pipeline fallback tests fail before wiring and pass afterward.
Focused server validation passes 67 tests; the later module-load boundary test passes with all 5 fallback-admission tests.
Six Chromium browser tests pass, including existing S38 integration and six admitted fallback cases compared against the actual package.
These cover crop offsets, ordered duplicate colors, 256-entry truncation, transparent-only palettes, and f64 preserve-alpha thresholds.
The browser test also reproduces the real 2-to-49 center-tie output mismatch at x=24 and verifies rejection.

Actual package input is S34 `2afd1802c3249948d53b2c2bc69e287c959a30fe`'s retained tarball.
SHA-256 is `01ad17dcf087d1debd7564de9d34acff94805f2034168687ab24e1431412955d`.
Only its generated `package/dist` was extracted into this worktree. No Rust build ran.
`svelte-check` reports zero errors and one existing missing `worker-configuration.d.ts` warning.
Final S34 report/artifact ancestry and the coordinator's final validation handoff remain pending.
