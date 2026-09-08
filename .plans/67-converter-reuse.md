# S26 converter reuse candidate

The complete literal public baseline is `089251287e387cb575e22e8993d8989a371a089d`.
Typed native component and complete-call adapters are registered at `cdbd6e07e98fceb77ebea8bd7b791159b430227d`.
The baseline rebuilds the existing byte conversion tables for each source or neighbor read.
This candidate reuses the existing Converter within a real perturb call and within each standalone contrast calculation.
It changes no conversion equations, lookup table contents, neighborhood order, reconstruction, field sequence, or matching implementation.

## TODOs

- [x] Share one call-owned converter between adaptive placement and source conversion in the production field loop.
- [x] Preserve single-pixel conversion as an unchanged construction-inclusive benchmark control.
- [x] Verify frozen bytes, masks, alpha, row bands, complete composition, and exact/one-under memory budgets.
- [ ] Join identical public benchmark adapters into both roles and validate the actual installed packages before timing.
- [ ] Run one fresh 208-worker comparison under the declared S26 matrix, after all agents/builds/tests drain.
- [ ] Select only exact output and a passing required-case regression/noise gate; otherwise retain the baseline.

## Target and limits

The target is at least 20% lower median latency for one adaptive or nontrivial-color complete call.
Removing repeated table construction is expected to matter most for eight-neighbor placement; the target is not a measured claim.
Other cases need not improve, but every required case must satisfy the existing 10% regression and pair-noise gate.
Inverse, field-threshold, and single-pixel conversion controls keep their actual production implementations and timing scopes.
Do not add a benchmark-only shortcut to make those controls faster.

The [S26 benchmark plan](67-benchmark.md) owns the 64x48 fixtures, nine complete recipes, sixteen components, sample settings, and caps.
Use one candidate revision and one complete comparison for this optimization. Retain failed or noisy evidence without automatic retries.
The accepted and candidate artifacts must come from verified fresh native/public builds and compatible protocol revisions.
No measurements begin before the public protocol and fixtures finish.

The existing working-capacity charge already includes one temporary Converter.
The candidate must not retain another converter alongside it or change that bound without updating actual capacity accounting.
Frozen spec, image storage, landed resize/shared helpers, and quantizer remain unchanged.

## Candidate validation

The four-file candidate passes 13 native field/processor tests and 11 benchmark adapter/metric tests.
All 24 interface/type tests and 11 private ABI tests pass, including repeated failure and reentry capacity checks.
Both Wasm variants build. Installed-package checks pass Chromium, Firefox, and WebKit.
Each engine checks 91 frozen field vectors and 1,365 compositions.
The trusted frozen guard retains digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Formatting and diff checks pass. An independent read-only review found no actionable issues.
These checks establish correctness only. The candidate remains unselected until the declared comparison passes.
