# #76 Installed threaded lifecycle tests

## Scope

Use the installed public factory and browser Worker boundary. Observe real worker lifetimes through held Web Locks.
The observer only changes served test responses. Package artifacts and public interfaces stay unchanged.

Base is validated S33 `06d9ad0730669dac3008baf848b5eb463689c584`.
Runtime implementation belongs to `v1-s34-threads`. This branch owns new browser fixtures and required installed-driver support.

## TODOs

- [x] Validate the real-worker lifetime observer in Chromium, Firefox, and WebKit.
- [ ] Add scalar selection, capability fallback, required errors, and custom Wasm checks.
- [ ] Add real pool ownership, partial startup cleanup, disposal, and host termination checks.
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
