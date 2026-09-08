# S32 installed-package ownership fixtures

Issue #73. Test-only branch `impl/v1-s32-public` starts at S31
`a3c9629f35280c36e838faa00e9b664b23abcb53` and joins the runtime owner's `impl/v1-s32-stages` branch.

## Scope and ownership

This worktree owns package test fixtures, installed-runner integration, and this plan only.
The runtime owner implements stage caching and native hit/accounting/publication proof.
The benchmark owner implements measured protocol support. No public cache controls are added.
Public equality can pass on uncached S31 and does not prove reuse or atomic publication.

## TODOs

- [ ] Add bounded actual-package checks for mutable inputs/results, cross-method composition, settings changes, disposal, and copy-failure recovery.
- [ ] Run the fixture against the retained installed S31 package in Chromium, Firefox, and WebKit; record its digest.
- [ ] Commit a clean test-only checkpoint for the runtime owner; require candidate validation after the S32 package exists.

Reuse the existing frozen field vectors, actual public request shapes, browser asset server, and installed tarball runner.
The baseline tarball is `v1-s31-preparation/target/s31-candidate-972d4e9a-public/ditherette.tgz`.
Expected SHA-256 is `ce556c8dc5a7f747ebdd572d84d0185a9d2db2796cd9d3af085905fa3d9bb4a0`.
No compiler cache belongs to this worktree. No compilation or performance measurements are needed for baseline checks.
Retain any target-local fixture evidence; drain browser jobs before returning the checkpoint.
