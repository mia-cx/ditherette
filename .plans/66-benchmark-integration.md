# S25 benchmark integration

Base: `5b697193205b9dd7a4d15c9d1f13ddebf2d58344`.
This subtask implements issue #66 measurement adapters, not production arithmetic.
Use the coordinator's `.plans/66-measurement-budget.md` for measurement authorization and limits.

## TODOs

- [x] Extend typed quantize bridges to all 15 policies and packed forward controls to seven spaces.
- [ ] Add seven typed score-batch controls with exact f32-bit verification and preallocated native execution.
- [ ] Declare the 276-worker experiment matrix and update executable inventory and conformance fixtures.
- [ ] Run controlled tests, record evidence, and push clean checkpoints.

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
