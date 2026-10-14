# #380 Responsive indexed PNG export

## Acceptance criteria
- Preserve exact indexed pixels, PLTE/tRNS, alpha and deterministic PNG output.
- Keep displayed indices attached; encode away from the main thread.
- Cancel stale exports and terminate workers on completion, failure and teardown.
- Keep current controls and filenames.

## TODOs
- [x] Replace row/zlib/chunk staging with one IDAT buffer; add cancellable dedicated worker.
- [x] Integrate export identity/teardown and verify real PNG decoding and lifecycle.
- [x] Capture browser flow evidence, prepare the benchmark pair and track deferred timings separately.

## Notes
- Current origin/main rebased before reading code; supplied branch retained.
- Parent owns preview/persistence, integration, production and DIRECTION.md.
- Child issue tracks this slice only; umbrella remains open.
- Measurements require parent scheduling, the build lock and host benchmark leases.
- Focused server tests passed, 28 tests. Workspace scalar/threaded package build passed.
- `pnpm check` passed with zero errors and warnings. Focused ESLint passed.
- Rebased onto e35d4bbb, frozen-installed its patched dependency graph, then reran 28 server tests, 22 browser tests, `pnpm check` and full `pnpm build`. All passed. Fresh logs are under `png-worker/revalidated/` in the task artifact directory.
- Removed the unused synchronous download entry point after moving its only caller to the worker path.

## Preserved evidence
- Historical prepared baseline: e4bffdd449c53194f5c181fd4e4c34d873914bc6. Candidate: fe06ae3b490b838eb86ed2bcfb5f53b9e4fadc6d. Preparation and source fingerprints remain in `png-worker/prepared/identity.json`.
- [Historical preflight](https://i.mia.cx/file/2026/10/results-d04d9e.json) independently verifies exact indices/alpha and byte-identical baseline/candidate PNGs.
- [Before browser flow](https://i.mia.cx/file/2026/10/evidence-9a2f63.json) and [after browser flow](https://i.mia.cx/file/2026/10/evidence-314bb3.json) verify a 3 MP source upload and the existing Download PNG control. Candidate frames continue, its worker terminates and displayed indices remain attached. Captures cover 1280×900 and 375×812.
- Quantitative timings are **unmeasured**, deferred to #394. [Recorded host contention](https://i.mia.cx/file/2026/10/host-window-3f3684.json) prevented five seconds of sampled quiet. No timing command or benchmark/build lease ran during that attempt; all owned servers/browsers stopped.
- Historical artifacts keep their original identities. They are not results from the later dependency revalidation. Preview memory/persistence and release performance gates remain outside this slice.
