# #76 Installed threaded lifecycle tests

## Scope

Use the installed public factory and browser Worker boundary. Observe real worker lifetimes through held Web Locks.
The observer only changes served test responses. Package artifacts and public interfaces stay unchanged.

Base is validated S33 `06d9ad0730669dac3008baf848b5eb463689c584`.
Runtime implementation belongs to `v1-s34-threads`. This branch owns new browser fixtures and required installed-driver support.

## TODOs

- [x] Validate the real-worker lifetime observer in Chromium, Firefox, and WebKit.
- [x] Add scalar selection, capability fallback, required errors, and custom Wasm checks.
- [x] Add real pool ownership, partial startup cleanup, disposal, and host termination checks.
- [ ] Run syntax checks and baseline checks, then hand off a clean fixture checkpoint for candidate validation.

## Evidence rules

- Require lock acquisition before testing release. Wait for observable conditions with bounded deadlines.
- Successful initialization and processing use the real installed package. Only browser capability and startup failure boundaries are injected.
- Preserve existing progress, reentry, output ownership, and disposal tests. Reuse their fixtures on the threaded artifact when possible.
- Baseline failures prove missing threaded behavior only. Candidate pass claims require the actual S34 tarball.
- No compiler outputs or benchmark measurements belong to this worktree.

## Validation

`thread-observer-browser.test.mjs` passes 4 tests across Chromium 147, Firefox 148, and WebKit 26.4.
It observes both parent and child locks before termination, then waits for zero held locks.
Firefox bypasses browser routing for nested worker requests, so the server serves temporary instrumented copies.
The prelude buffers early startup messages while acquiring the lock, then delivers them to the original handlers.
Installed S33 tarball SHA-256 is `f6e62526cc290d8ca1f9fdcb7fcfc9de39790c1982dab7118adb30a4a84173d2`.

The scalar selection test passes 4 tests against S33. Threaded ownership fails on S33 in every engine as expected.
The error is its unimplemented required-thread capability path, before any pool assertion.

Development S34 package source is `a5915dda32ac875557272a6ca67b4f86b965b558`.
Its packed SHA-256 is `1c33e616ee5712ea382ce15ddde27c708792ceeb1ce77524e2885f5a34574a48`.
This uses the runtime owner's existing generated dist, not a fresh revision-bound acceptance build.

- Chromium and Firefox pass threaded ownership, all five methods, ten callback failures, independent memories/budgets, and nine custom Wasm inputs.
- Partial startup cleanup passes all three engines after observing two acquired workers for each selection mode.
- Chromium and Firefox pass processing-host termination during a synchronous callback and during partial startup.
- WebKit 26.4 fails actual Rayon worker release after disposal and host termination. All four pair locks remain after first disposal.
- The WebKit processing-host witness retains the host lock and its two pool locks. Ordinary nested-worker termination passes.

The coordinator and runtime owner are investigating the WebKit engine behavior. Keep these assertions intact pending resolution.

The opt-in `thread-atomic-wait.diagnostic.mjs` reproduces the same failure without Ditherette or Rayon imports.
It uses the exact upstream WebKit bug 289686 module bytes. Chromium and Firefox release its lock; WebKit retains it.
The installed runner closes each browser in `finally`, including diagnostic failures.
