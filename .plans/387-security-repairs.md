# #387 Hide benchmark errors and pin Pullfrog

## Summary

Keep benchmark exceptions in local stderr. Pin Pullfrog's audited upstream `v0` action commit without changing its contract.

## Acceptance criteria

- [ ] Malformed URLs and asset read errors return the same generic HTTP 500 body and log diagnostics locally.
- [ ] Real HTTP tests cover successful assets and shutdown without executing benchmark workloads.
- [ ] Pullfrog uses a verified full commit with a `v0` comment; inputs, secrets, and permissions stay unchanged.
- [ ] Focused tests, formatting/lint, and action validation pass; every diff hunk is reviewed.

## TODOs

- [ ] Make the CLI importable, repair HTTP errors, and add request-level tests.
- [ ] Pin the audited Pullfrog commit and record the upstream execution path.
- [ ] Run final checks, inspect every hunk, rebase, push, and file a linked non-draft PR closing only #387.

## Notes

- Worktree and branch stay as supplied. Parent owns integration, CI review, alert disposition, merge, and deployment.
- Initial fetch/rebase onto `origin/main` succeeded with a clean tree.
- Upstream `refs/tags/v0` resolves to commit `ad4283a862b8377cb525a0f5bf30f17c023b18ea` through the GitHub API.
