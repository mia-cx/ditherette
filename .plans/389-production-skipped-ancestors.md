# #389 Production jobs after skipped ancestors

Main CI succeeds but production jobs skip after the PR-only Changeset job skips.
Both guards must override Actions' implicit success() while requiring successful
CI/conformance, a trusted repository, and a main push or manual dispatch.

## TODOs

- [x] Reproduce skipped-ancestor suppression and repair both production guards.
- [x] Run release tests, focused lint and workflow validation.

## Post-merge verification

The parent verifies the new main run starts conformance and delivers its exact
source after the required gates pass. This check needs the merged workflow.

## Evidence

Run38641062557 at6c1ab880 reports successful CI result and two skipped production
jobs with no steps. The deployment API returns no deployment for that SHA.
Independent review and read-only delivery verification agree on this failure.
GitHub documents the default success() condition and recommends !cancelled().
The focused regression fails on the original workflow at production_validation
for a main push with skipped ancestors. Both guards now include !cancelled().
All55 release tests, focused Prettier/ESLint, actionlint1.7.12 and diff checks pass.
