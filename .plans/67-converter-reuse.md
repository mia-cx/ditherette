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
- [x] Join identical public benchmark adapters into both roles and validate the actual installed packages before timing.
- [x] Run one fresh 208-worker comparison under the declared S26 matrix, after all agents/builds/tests drain.
- [x] Select only exact output and a passing required-case regression/noise gate; otherwise retain the baseline.

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

## Outcome

The candidate remains on `impl/v1-s26-converter` at `b237b7468fa5fc349760bc0086bd1748113b6d82`.
It exceeds the complete-call target, but required verification and noise gates are incomplete.
The accepted branch retains the original field implementation. No retry runs.
See the [measurement record](67-measurement.md) for every median, artifact identity, and limitation.
