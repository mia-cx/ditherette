# S09 palette and alpha reference

Issue [50](https://github.com/mia-cx/ditherette/issues/50).
Branch `impl/v1-s09-palette`, originally based on S03 `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`.
Current PR base is `main`.
Review [PR 94](https://github.com/mia-cx/ditherette/pull/94), open and non-draft with auto-merge disabled.

## TODOs

- [x] Implement ordered palette preparation, byte-alpha rules, metadata ownership, and focused semantic fixtures.
- [x] Validate the complete native suite and Wasm compilation; record the evidence.
- [x] Rebase onto the latest S03 branch and file an unmerged, non-draft PR.

## Semantic evidence

The reference retains order and duplicates, truncates before preparation, excludes transparent entries from matching,
and carries original indices into normalized output. Transparent-only palettes short-circuit every alpha policy.
Darkest fallback uses integer RGB sum and first ties. Warnings retain website wording and order.

Alpha compositing uses f64 before byte reconstruction. The preserve threshold also uses f64.
The exact counterexample `127.9999999` shows why S03's f32 threshold changes alpha-128 classification.
No working-color precision or other request field changes.

`PaletteEntry::Transparent {}` replaces the unit variant so Serde rejects extra fields as the tagged boundary promises.
The JSON representation remains `{"kind":"transparent"}`. Downstream Rust constructors and patterns need the empty braces.
The coordinator approved both shared contract corrections before freeze.

The package accepts concrete matte RGB. Website matte-key resolution and its selection warnings remain outside this contract.
Standalone perturb preserves alpha and processes hidden RGB before indexed preparation.
Executable field composition belongs to the perturb slices; S07/S08 inverse adapters already carry byte alpha.

Normative details and TypeScript source locations live in `crates/ditherette-wasm/src/spec/palette/README.md`.
No production code, benchmarks, aggregate ledger, or root user files change.

## Original validation

`cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_palette --test spec_contract`
passes all ten palette tests and eight contract tests.
The first run exposed the transparent unit variant's extra-field acceptance; the strict empty-struct variant fixes it.

Implementation commit `841334cb` passes:

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked`: historically reported 134 native tests; superseded by current validation below.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown`: passes.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --all -- --check`: passes.
- `git diff --check`: passes.

These are correctness and compilation checks. No benchmark process or browser timing run was started.

The final fetch/rebase found S03 unchanged at the recorded dependency SHA.
Focused tests passed again after rebase. Validated code and evidence head was `33c192d49531adc9e66bebcb1211a19221d8ce36`.
The final bookkeeping commit changes only this PR record; its exact SHA appears in the coordinator's stack ledger.

## Stack-collapse validation

Merge `a9af3f5b` restacks S09 onto main `8c19f361` without conflicts or changes
to its palette implementation. The merged request validator retains S03's corrected
implicit-source `InvalidImage` classification. All 148 native tests pass, including
ten palette tests and sixteen contract tests. Benchmark-feature compilation,
Wasm-target compilation, and rustfmt pass. No standalone timing workload, browser
run, production change, or new reference amendment occurred during collapse.
