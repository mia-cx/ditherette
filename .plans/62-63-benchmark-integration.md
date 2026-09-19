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
- [x] Register public area/bilinear and convolution benchmark recipes using actual TypeScript and frozen Rust.
- [x] Join validated slice implementations and verify public ownership, memory failures, and installed package behavior.
- [x] Prepare immutable accepted/candidate artifacts and record a bounded filter-specific measurement budget.
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
S22 public integration follows the S21 join. The completed measurement record is in [63-measurement.md](63-measurement.md).

S22 public checkpoint `f0976601` and diagnostic fixes join at measured candidate `1761705e2c6935544b0232427d48129059d89615`.
The combined core passes 296 native tests and the trusted frozen guard.
The public build includes scalar and threaded artifacts. The installed benchmark-adapter fixtures pass all three engines.
The tarball SHA-256 is `91729b064dc856fa3cac256369e73f98a41b847aed95f568cf32e2a4a544ac5e`.
Native accepted revision `e64ee3f43d547edd2424c34392e01990c2442c5d` contains restored production unchanged from `467542f4`.
Its only changes are native benchmark identity validation, shared with the candidate. Both executables were freshly built.
Immutable native pairs and six browser snapshots are under `target/resize-trial-01`.

Read-only review found two benchmark defects, both fixed before measurement.
Native support variants now have separate semantic identities and use existing registry oracle mappings.
An unstable diagnostic result retains both actual images and fails before publishing a trial result.
Seven library/binary/example tests, nineteen paired/worker integration tests, and thirteen JS fixtures pass.

## Website semantic differences

The untimed adapter checks exposed existing differences, not new production approximations.
For opaque red values `[0, 64, 128, 192]` reduced from four pixels to three:

- Website area returns `[21, 96, 171]`. Its inclusive box uses equal weights; the package overlap filter returns `[16, 96, 176]`.
- Website bilinear returns `[11, 96, 181]`. Its two-tap reduction differs from the package's widened triangle result `[17, 96, 175]`.

Website area also switches to bilinear when enlarging both axes. Keep these differences visible in diagnostic artifacts.
These public comparisons cannot prove equivalent-output speedups or faithful fallback. No website or production kernel was changed.
The adapter test correction and this evidence follow artifact preparation; the measured runtime code stays at `1761705e`.

## Constraints

No frozen spec/image/policy edits. No replacement or repeated optimization of landed kernels.
Preserve established exact or bounded production behavior. New non-exact changes require Mia's approval.
Reference drift must remain visible in benchmark artifacts; timing evidence never grants visual acceptance.
An explicit developer-only `measure_nonexact` browser flag may collect timing despite reference drift.
It preserves the actual differing output and the incorrect conformance gate. It does not approve an approximation.
Existing landed-output conformance remains separately checked against the original production entrypoints.
No measurements during implementation. Never run more than one ditherette-bench process.
Leave all PRs unmerged. No publishing, tags, deployment, or rollout.

## Measurement outcome

All 304 workers launched and reaped, retaining 5,760 samples across eight complete reports.
Native production pairs preserve landed bytes exactly. Existing frozen differences keep strict incorrect gates visible.
Native overhead stays below 10%; no kernel retuning is justified by this trial.
Public diagnostics expose complete-call browser costs and non-equivalent TypeScript output, especially poor Firefox performance.
S41 retains that work. No new approximation is accepted and no release readiness is claimed.
Implementation resumed only after the last worker exited. Separate S21/S22 PR filing remains the final TODO.
