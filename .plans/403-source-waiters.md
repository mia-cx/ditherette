# #403 Keep processing waiters for the current source

## Summary
An old source's compile reply rejects processing waiting for the newly selected source.
Keep waiter validity tied to the selected source and settings, while accepting replies only for their matching source.

## Acceptance criteria
- [x] A stale success or failure leaves the current source/key waiter pending until its compile resolves.
- [x] Superseded source/settings waiters reject, and current compile failures still reach processing.
- [x] Retention, fit restoration, stale publication guards, scheduling and cleanup keep their existing behavior.

## TODOs
- [x] Fix waiter invalidation with focused behavioral regressions and a website patch changeset.
- [x] Prove the regressions fail on the base, run controller/caller checks, and review every diff hunk before publishing.

## Notes
- Supplied branch `t3/stale-source-waiter` advanced cleanly from `f420d494` to exact main `0d14783cb79fff2ac98601f50c18dd98b621b674`.
- Cooperative environment: source and settings changes race normal effects-worker replies.
- Trigger: `processCurrentImage` in `client.ts` waits through `compiledEffectsFor` after the source store changes, while `receive` can still receive the previous source's reply.
- No GitHub Project is attached to the issue or repository.
- Parent owns DIRECTION.md, merges and delivery. This slice changes only the controller, its server spec, this plan and a website patch changeset.
- Evidence directory: `/home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/source-waiter-403/`.
- Baseline: 98/98 server tests pass, including all 25 existing live-effects tests.
- Before the fix, both new stale-success/failure cases fail because B's promise rejects with `The effects changed.`
- After the fix: 101/101 tests pass (live-effects 28, client 8, worker-pipeline 17, package-adapter 48).
- Full `pnpm lint`, `pnpm test:release` (55 tests), focused Prettier/ESLint and `git diff --check` pass.
- Reviewed every hunk. Reply acceptance still requires the matching source; waiter retention uses the selected source and current key.
- Tests use a private Node Vitest config that resolves the package to its TypeScript entry point. Engine initialization and worker transport remain mocked. No browser, native build, full app typecheck, timing or GPU work ran.
