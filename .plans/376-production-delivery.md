# #376 Direct production website delivery

Deploy verified main commits independently of npm release eligibility. Parent owns merges and deployment.

## Acceptance criteria

- Ordinary main commits deploy after successful CI and conformance at the exact SHA.
- Failed validation, PR runs and superseded commits cannot deploy.
- Production credentials, migrations, serialization and manual recovery remain available.
- npm authorization, provenance, size gates and S41 holds remain unchanged.

## TODOs

- [x] Add main delivery, supersession guards and executable workflow tests.
- [~] Document delivery and recovery; run release tooling and focused lint.
- [ ] Review every changed file, rebase, push and file the non-draft PR.

## Notes

- Clean supplied branch rebased onto origin/main at 2dac2a2e.
- Website delivery lives in CI. Release keeps npm only, avoiding duplicate deployments and version tags for ordinary commits.
- Fresh reusable release validation runs on the exact main SHA before website delivery.
- Parent approved release-validation.yml concurrency ownership. Include caller workflow in the key to keep CI and Release validation independent while preserving latest-head PR cancellation.
- No relevant GitHub Project exists. Issue uses the sibling v1 milestone.
- Workflow tests pass 21/21, including caller isolation. Actionlint 1.7.12, focused Prettier and ESLint pass.
