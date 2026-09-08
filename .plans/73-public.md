# S32 installed-package ownership fixtures

Issue #73. Test-only branch `impl/v1-s32-public` starts at S31
`a3c9629f35280c36e838faa00e9b664b23abcb53` and joins the runtime owner's `impl/v1-s32-stages` branch.

## Scope and ownership

This worktree owns package test fixtures, installed-runner integration, and this plan only.
The runtime owner implements stage caching and native hit/accounting/publication proof.
The benchmark owner implements measured protocol support. No public cache controls are added.
Public equality can pass on uncached S31 and does not prove reuse or atomic publication.

## TODOs

- [x] Add bounded actual-package checks for mutable inputs/results, cross-method composition, settings changes, disposal, and copy-failure recovery.
- [x] Run the fixture against the retained installed S31 package in Chromium, Firefox, and WebKit; record its digest.
- [x] Prepare the clean test-only checkpoint for the runtime owner; require candidate validation after the S32 package exists.

Reuse the existing frozen field vectors, actual public request shapes, browser asset server, and installed tarball runner.
The baseline tarball is `v1-s31-preparation/target/s31-candidate-972d4e9a-public/ditherette.tgz`.
Expected SHA-256 is `ce556c8dc5a7f747ebdd572d84d0185a9d2db2796cd9d3af085905fa3d9bb4a0`.
No compiler cache belongs to this worktree. No compilation or performance measurements are needed for baseline checks.
Retain any target-local fixture evidence; drain browser jobs before returning the checkpoint.

## Baseline validation

The focused installed-package runner passes all four tests in 2,920.836058 ms.
Engines are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
The runner verified the expected tarball digest before installing it offline into a temporary consumer.
Each engine exercises all five methods, 18 cross-method comparisons, four settings transitions,
one final-copy rejection/recovery sequence, and two separately initialized instances.
The existing full installed-tarball runner now invokes the same fixture; that full suite was not rerun here.
Syntax checks and focused Prettier checks pass. No Rust compilation or measurements ran.

```sh
DITHERETTE_TEST_TARBALL=/home/mia/mia-cx/ditherette/.worktrees/v1-s31-preparation/target/s31-candidate-972d4e9a-public/ditherette.tgz \
DITHERETTE_TEST_TARBALL_SHA256=ce556c8dc5a7f747ebdd572d84d0185a9d2db2796cd9d3af085905fa3d9bb4a0 \
DITHERETTE_TEST_WEBKIT_EXECUTABLE=/home/mia/mia-cx/ditherette/.worktrees/v1-s20-worker/target/s20-webkit-alias/webkit \
node --test packages/ditherette/tests/stage-cache-browser.test.mjs
```

After an S32 artifact exists, run this command with its tarball path and actual SHA-256.
Then run the ordinary full installed-package suite in the joined runtime worktree.
The runtime owner confirms separate native hit and atomic-publication assertions are in scope.
The public failure fixture proves caught final-copy failure and recovery, not invisible pending-entry state.
