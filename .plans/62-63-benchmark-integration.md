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

## Fixed measurement budget

`resize_integration_plan` declares two alternating pairs, 20 samples, 50 ms warmup,
a 250 ms measurement cap, and a 2 ms throughput target. Its fixture test verifies the budget.
Run each matrix once. Keep incomplete or inconclusive evidence; do not keep retrying for a pass.

- S21 native has four cases. Public has eight cases per engine, including first enlargement and unequal axes.
- S22 native and public each have twelve cases, covering all six filter/support combinations in latency and throughput.
- Native comparisons measure the existing one-shot entrypoint against fallible plan/scratch construction plus execution.
- Public comparisons use actual website TypeScript. Bicubic uses an explicit package/package control because the website lacks bicubic.
- The full budget is 304 serial workers across native and Chromium, Firefox, and WebKit.

Opaque performance fixtures avoid comparing different alpha conventions. Dedicated untimed tests retain transparent coverage.
Reference differences remain incorrect diagnostic evidence, even when timings are retained.
Byte-exact comparisons against the landed entrypoints separately establish preservation.
Judge integration overhead within each filter and support policy, never against nearest-neighbor throughput.
Only a measured integration bottleneck can justify one further bounded candidate comparison. Retain the original result too.

## Joined validation

S21 public code `35169fc0`, evidence `056a1324`, joins at `b52d1c8b`.
Its owner reports 287 native, 14 interface, six private ABI, and all three installed-package engines passing.
Both scalar/threaded builds and the trusted frozen guard pass. Independent allocation/dispatch review found no defects.
S22 native support `53eaf013` passes nine focused tests, including 378 byte-exact landed-output comparisons.
S22 public integration follows the S21 join. Actual measurements remain pending.

## Constraints

No frozen spec/image/policy edits. No replacement or repeated optimization of landed kernels.
Preserve established exact or bounded production behavior. New non-exact changes require Mia's approval.
Reference drift must remain visible in benchmark artifacts; timing evidence never grants visual acceptance.
An explicit developer-only `measure_nonexact` browser flag may collect timing despite reference drift.
It preserves the actual differing output and the incorrect conformance gate. It does not approve an approximation.
Existing landed-output conformance remains separately checked against the original production entrypoints.
No measurements during implementation. Never run more than one ditherette-bench process.
Leave all PRs unmerged. No publishing, tags, deployment, or rollout.
