# Remaining resize integration

Continue S21/S22 from restored production `467542f49ce3f600e5b03aeef574a97be554ae15`, delivered in unmerged PR109.
Existing kernels and helpers stay production. Only allocation/public integration and missing benchmark registration change.

## Ownership

- S21 owns area/bilinear, the shared allocation helper, and the first public resize extension.
- S22 owns convolution/bicubic/Lanczos allocation entrypoints, then stacks its public extension on S21.
- S24 independently owns packed color/palette/direct matching and its native tests.
- The coordinator owns this branch, benchmark protocol/adapters, joins, progress, measurements, and PR filing.

## TODOs

- [x] Verify restored ancestry and green restoration checks; dispatch isolated owners.
- [ ] Register public area/bilinear and convolution benchmark recipes using actual TypeScript and frozen Rust.
- [ ] Join validated slice implementations and verify public ownership, memory failures, and installed package behavior.
- [ ] Prepare immutable accepted/candidate artifacts and record a bounded filter-specific measurement budget.
- [ ] Drain agents/builds/tests, run exclusive measurements, retain all results, and file unmerged slice PRs.

## Constraints

No frozen spec/image/policy edits. No replacement or repeated optimization of landed kernels.
Preserve established exact or bounded production behavior. New non-exact changes require Mia's approval.
Reference drift must remain visible in benchmark artifacts; timing evidence never grants visual acceptance.
No measurements during implementation. Never run more than one ditherette-bench process.
Leave all PRs unmerged. No publishing, tags, deployment, or rollout.
