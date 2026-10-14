# #387 Hide benchmark errors and pin Pullfrog

## Summary

Keep benchmark exceptions in local stderr. Pin Pullfrog's audited upstream `v0` action commit without changing its contract.

## Acceptance criteria

- [x] Malformed URLs and asset read errors return the same generic HTTP 500 body and log diagnostics locally.
- [x] Real HTTP tests cover successful assets and shutdown without executing benchmark workloads.
- [ ] Pullfrog uses a verified full commit with a `v0` comment; inputs, secrets, and permissions stay unchanged.
- [ ] Focused tests, formatting/lint, and action validation pass; every diff hunk is reviewed.

## TODOs

- [x] Make the CLI importable, repair HTTP errors, and add request-level tests.
- [ ] Pin the audited Pullfrog commit and record the upstream execution path.
- [ ] Run final checks, inspect every hunk, rebase, push, and file a linked non-draft PR closing only #387.

## Notes

- Worktree and branch stay as supplied. Parent owns integration, CI review, alert disposition, merge, and deployment.
- Initial fetch/rebase onto `origin/main` succeeded with a clean tree.
- Upstream `refs/tags/v0` resolves to commit `ad4283a862b8377cb525a0f5bf30f17c023b18ea` through the GitHub API.
- Export only the existing server. Guard CLI execution with `import.meta.main` and load Playwright inside the CLI, so the HTTP test needs no browser or built Wasm.
- The real HTTP regression first failed because `/pkg/%ZZ` returned a URIError stack with this worktree's path. After repair, malformed escapes and missing assets return `Internal server error`, while stderr retains URIError/ENOENT, stack frames, and the missing path.
- `TMPDIR=/home/mia/.cache/ditherette-tmp/autonomy-2026-10-14/security-repairs/tmp node --test scripts/benchmark-wasm-resize-server.test.mjs` passes. The test also checks package/fixture assets, MIME/isolation headers, and listener shutdown.
- `pnpm exec eslint scripts/benchmark-wasm-resize.mjs scripts/benchmark-wasm-resize-server.test.mjs` passes.
- The existing, unchanged broader tests have four failures: `benchmark-wasm-resize.test.mjs` treats the `bench:pnpm:entry` discovery utility as a measurement alias; `benchmark-transport.test.mjs` invokes the transport without its required lease environment. Their source and the manifest already contain these mismatches on `origin/main`. Keep these separate from the security repair.
