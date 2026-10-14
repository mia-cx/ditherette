# #380 Responsive indexed PNG export

## Acceptance criteria
- Preserve exact indexed pixels, PLTE/tRNS, alpha and deterministic PNG output.
- Keep displayed indices attached; encode away from the main thread.
- Cancel stale exports and terminate workers on completion, failure and teardown.
- Keep current controls and filenames.

## TODOs
- [x] Replace row/zlib/chunk staging with one IDAT buffer; add cancellable dedicated worker.
- [x] Integrate export identity/teardown and verify real PNG decoding and lifecycle.
- [ ] Capture browser flow evidence and prepare exclusive before/after measurements; file the child issue PR.

## Notes
- Current origin/main rebased before reading code; supplied branch retained.
- Parent owns preview/persistence, integration, production and DIRECTION.md.
- Child issue tracks this slice only; umbrella remains open.
- Measurements require parent scheduling, the build lock and host benchmark leases.
- Focused server tests passed, 28 tests. Workspace scalar/threaded package build passed.
- `pnpm check` passed with zero errors and warnings. Focused ESLint passed.
