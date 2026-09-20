# S25 benchmark integration

Base: `5b697193205b9dd7a4d15c9d1f13ddebf2d58344`.
This subtask implements issue #66 measurement adapters, not production arithmetic.
Use the coordinator's `.plans/66-measurement-budget.md` for measurement authorization and limits.

## TODOs

- [x] Extend typed quantize bridges to all 15 policies and packed forward controls to seven spaces.
- [x] Add seven typed score-batch controls with exact f32-bit verification and preallocated native execution.
- [x] Declare the 276-worker experiment matrix and update executable inventory and conformance fixtures.
- [x] Run controlled tests, record evidence, and push clean checkpoints.

## Constraints

Production, package, Wasm ABI, frozen spec, image, and policy files remain unchanged.
No measurements or artifact builds run in this subtask.
Tests reuse `v1-resize-bench-subjects/crates/ditherette-bench/target` exclusively.
Native full quantize includes owned result destruction and uses fixed deterministic endpoint conformance.
Public calls retain the existing every-output stability checks.

## Validation

`cargo test --manifest-path crates/ditherette-bench/Cargo.toml --test quantize_adapters --test paired_quantize` passes four tests.
The adapters check 135 quantize combinations plus transparent-only fallback and seven packed conversions against frozen outputs.
The installed-package fixture now requires 47 cases covering all 15 modes. Browser execution awaits fresh coordinator artifacts.

Score controls compare each row-major source pixel against its cyclic successor, including last-to-first.
The recipe `metric-cyclic-successor-frozen-forward`, version 1, binds this pairing and frozen conversion.
Metric family and coordinate space enter normalized settings. The complete RGBA fixture enters the source digest.
Euclidean and weighted RGB use sRGB, chord/arc use Oklch, and CIEDE2000 uses CIELAB.
Alpha bytes remain identity-bound but do not enter the metric.
Native metric timing includes one batch loop and direct scalar function calls over preconverted pairs.
Pair conversion, output allocation, and exact verification occur outside timing. Every iteration has input/output black-box barriers.
Scores serialize as f32 bits and retain their own error report. Count and finite-value checks reject malformed evidence.
These unchanged metric functions provide component coverage, not an acceleration claim.

The four focused integration suites now pass 15 tests, including signed-zero score differences and finite/count validation.

## Final handoff

The typed generator and inventory are documented in [66-executable-inventory.md](66-executable-inventory.md).
Full CIEDE2000 alone uses the coordinator-approved 10,000 ms cap. Other cases retain 250 ms, including metric-only CIEDE2000.
The generator asserts the fixed 80 + 196 worker split and the exact cap exception.

Final controlled validation uses the assigned benchmark target:

- Eight integration suites pass 42 tests: `browser_worker`, `metric_scores`, `paired`, `paired_browser`, `paired_quantize`, `quantize_adapters`, `reference_subjects`, and `verification`.
- `--bin ditherette-bench paired_native::tests` passes its native scope/callable guard test without timing.
- `--example matching_integration_plan --example quantize_integration_plan` passes both budget tests.
- The explicitly selected ignored S24 compatibility test passes against retained `s24-quantize-attempt-02` artifacts, checking 20 real request/result round trips.
- `node --test scripts/benchmark-public-timing.test.mjs scripts/benchmark-public-browser.test.mjs` passes 26 fake-operation and transport fixtures.
- Rust format, JavaScript syntax, and the separately trusted frozen-content guard pass.

The initial wider check found the old reference registration count of 12. It now expects 19 after adding seven score oracles.
No production/package/ABI, frozen spec/image/policy, or measured artifacts changed.
Actual 47-fixture browser conformance, release artifact builds, and fresh measurements remain with the coordinator.
Existing S24 artifact reuse is compatible. No historical timing sample enters the new experiment declarations.
