# S03 reference contracts

Implements [issue 44](https://github.com/mia-cx/ditherette/issues/44).
Current base: main `9e99b5bbd6b4e52475ab703e94e0ebcbed22e6cc` after [PR 88](https://github.com/mia-cx/ditherette/pull/88) merged, restacked through merge `85eaad562fbab86260d9d0f1912d1bb03bbd8df8`.
Original base and dependency SHA: `a213effed4b426c5c432c9ccc7062b7016dd5c1b`; original base PR: [75](https://github.com/mia-cx/ditherette/pull/75) (`impl/v1-s01-anchor`).
Branch: `impl/v1-s03-contracts`.

## Work

- [x] Define shared result storage and typed reference requests with validation fixtures.
- [x] Specify lifecycle and progress transitions with focused tests.
- [x] Inventory every mode, export, and adapter; record validation for the unmerged PR.

## Scope

This slice specifies requests and control behavior. S07 through S17 complete the processing references before S18 freezes them.
S09 owns palette normalization and alpha behavior. Production dispatch remains unchanged.

Read [the contract inventory](../../../crates/ditherette-wasm/src/spec/contract/inventory.md) when implementing a request consumer or preparing the freeze.
It maps all supported modes, inherited exports, adapters, and remaining reference obligations.
S15/S28 must preserve sRGB feedback with perceptual matching and the separate adaptive-placement coordinates documented there.

## Historical evidence

`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_contract` passes all 8 request/storage fixtures.
`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_contract_lifecycle` passes all 6 control fixtures.
`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked` passes all 124 native tests and zero doctests.
`cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown` passes.
`cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` and `git diff --check` pass.
No benchmark measurements belong to this slice.

The S01 dependency is present in branch ancestry. The PR records the final head SHA and remains unmerged.

## Verified bug amendment, 2026-09-12

PR89 review comments 3949411282 and 3949411271 identify the source-size error category and missing contract documentation.
Mia authorized these verified corrections during stack collapse.
`Request::validate` now chooses `InvalidImage` for source-derived output limits and retains `InvalidSettings` for explicit outputs.
The focused proof covers both axes for all three implicit-output methods, the accepted boundary, and both explicit-output methods.
The four required documents are `spec/contract/spec.md`, `error.md`, `request.md`, and `lifecycle.md`.

The downstream S18 checkpoint currently binds revision `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`
and content digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
Within its frozen closure, this amendment changes only `src/spec/contract/request.rs` and adds those four Markdown files.
The regression tests and this plan sit outside that closure. S03 does not modify the later checkpoint or guard.
When S18 reaches the front, record the corrected reference identity and exact amendment without accepting unrelated content differences.
Carry the corrected classification into production integration where duplicated validation requires it.

Verification on 2026-09-12 with Rust 1.97.0 against pre-fix restacked baseline `85eaad56` and amended head `694365fa`:

- `cargo +1.97.0 test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test spec_contract implicit_output_limits_report_invalid_source_images` failed before the fix with `(InvalidSettings, "source.width")` where `InvalidImage` was expected, and passes after it.
- `cargo +1.97.0 test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test spec_contract explicit_output_limits_remain_invalid_settings` passes before and after the fix.
- `cargo +1.97.0 test --locked --manifest-path crates/ditherette-wasm/Cargo.toml --test spec_contract --test spec_contract_lifecycle` passes 10 request/storage and 6 control fixtures.
- `cargo +1.97.0 test --locked --manifest-path crates/ditherette-wasm/Cargo.toml` passes all 118 native tests and zero doctests.
- `cargo +1.97.0 check --locked --manifest-path crates/ditherette-wasm/Cargo.toml --target wasm32-unknown-unknown` passes.
- `cargo +1.97.0 fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` and `git diff --check` pass.
