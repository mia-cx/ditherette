# #86 Held Wasm-default rollout

## Scope and acceptance

Prepare scalar Wasm as the website default on the verified S43 integration tip. Keep the existing developer override and faithful initialization fallback. Production exposes no backend selector. This branch remains unmerged until Mia accepts stability and the outstanding release gates clear.

Parent is S43 PR132, `impl/v1-s43-integration` at `15300c0dc461265fcef2bd72096de5202836706b`. Its [release-readiness report](85-release-readiness.md) owns inherited evidence and blockers.

## TODOs

- [x] Change only the website gate and prove default, developer override, fallback/error boundaries, and cancellation with focused tests.
- [ ] Document rollback, exact evidence reuse, and the held activation state.
- [ ] Rebase onto the latest parent with merge ancestry preserved, rerun affected checks, and file a non-draft unmerged PR.

## Notes

Use the existing `VITE_DITHERETTE_WASM_PROCESS` flag. Only an explicit `false` in development selects the legacy path. Production ignores that override. Package initialization still requests scalar defaults; threading remains separate.

No kernel, frozen reference, image, guard, package, or public-control changes. No Rust build or performance measurement is needed for this gate-only change. Root owns global tracking and GitHub dependency updates.

The production override test first fails with TypeScript output, then passes after the one-line gate change. Focused validation passes 70 server checks and six Chromium checks using the existing S43 tarball. The full focused runs have no skips. Prettier and ESLint pass for all four changed TypeScript files.
