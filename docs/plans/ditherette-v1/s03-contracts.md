# S03 reference contracts

Implements [issue 44](https://github.com/mia-cx/ditherette/issues/44).
Base and dependency SHA: `a213effed4b426c5c432c9ccc7062b7016dd5c1b`.
Base PR: [75](https://github.com/mia-cx/ditherette/pull/75).
Branch: `impl/v1-s03-contracts`. PR base: `impl/v1-s01-anchor`.

## Work

- [x] Define shared result storage and typed reference requests with validation fixtures.
- [x] Specify lifecycle and progress transitions with focused tests.
- [ ] Inventory every mode, export, and adapter; record validation and file the unmerged PR.

## Scope

This slice specifies requests and control behavior. S07 through S17 complete the processing references before S18 freezes them.
S09 owns palette normalization and alpha behavior. Production dispatch remains unchanged.

## Evidence

`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_contract` passes all 8 request/storage fixtures.
`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_contract_lifecycle` passes all 6 control fixtures.
No benchmark measurements belong to this slice.
