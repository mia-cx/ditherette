# S19 literal nearest baseline

Historical checkpoint only. [The restoration](108-restore-landed.md) reverses this replacement of already-landed production.
The contract copies remain; missing implementations still follow spec-first development.

Issue [60](https://github.com/mia-cx/ditherette/issues/60). This checkpoint completes only the literal-copy prerequisite.
The processor, fallible allocations, package boundary, and optimization evidence remain pending.

## Copy provenance

[The manifest](60-literal-nearest-baseline.json) records both SHA-256 values for each copied file.
The frozen source revision is `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
Its complete content digest is `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
The copy source base is `5e45b7b6677d1fe4241a8109c39818a36c44b61d`.

Four files match their reference bytes exactly. Nearest changes only its alignment import from `spec::` to `prod::`.
The copies retain reference docstrings and generic/strided behavior because this checkpoint permits no semantic edits.
The new contract barrel exports the copied request, error, and lifecycle modules.

The inherited five-file nearest directory moves to `prod/resize/scalar/nearest_candidate/`.
Each relocated file matches its base-revision bytes exactly, checked against `git show`.
Legacy Wasm exports, inherited nearest tests, and row-band sweep imports explicitly use that candidate.
The copied kernel calls neither the reference nor the candidate.

## Executable identities

| Subject | Implementation |
|---|---|
| `spec:resize:nearest:scalar` | Frozen direct nearest loop |
| `prod:resize:nearest:scalar` | Copied direct nearest loop |
| `candidate:resize:nearest:legacy` | Inherited packed/plan kernel, unpromoted |

Each production/candidate descriptor names the frozen nearest oracle.
The registry test calls all three subjects and checks concrete source paths and expected bytes.
Legacy row-band sweeps select the candidate explicitly and reject copied/reference nearest IDs.
Other inherited filter registrations retain their previous status. This checkpoint makes no claim about their acceptance.

## Validation

The baseline copy test checks exact source text after the single declared import substitution.
Nearest conformance covers 1,764 RGBA8 cases across nine anchors, 49 shape pairs, and four source/output stride combinations.
Fixtures include single rows/columns, identity, enlargement, reduction, unequal axes, unchanged padding, alpha, and hidden RGB.
Additional cases compare float payload bits, negative zero, infinities, and palette indices.
Contract cases compare all five request variants, errors, wire tags, and numeric limits.
Lifecycle cases compare initialization choices, preflight boundaries, callback throttling/failure, publication permission, reentry, and disposal.

Validation commands:

```sh
cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects
cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects
cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --test reference_subjects --test verification --test verification_adapters --test lease --test paired
cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --benches
cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --all --check
cargo fmt --manifest-path crates/ditherette-bench/Cargo.toml --all --check
```

Core native validation passes 280 tests, including seven focused baseline tests.
Wasm compilation, formatting, and benchmark-target compilation pass.
The selected benchmark adapter suite passes 26 top-level Rust tests, with one controlled child fixture ignored at top level.
Its two child invocations and three transport cleanup fixtures also pass. None measures image-processing performance.
The trusted freeze guard passes using the separate `v1-s19-base` checkout's validated pre-fixture S18 policy.
The initial attempt against the newer S18 policy rejected the expected policy-file mismatch before checking semantics.
The coordinator will join that policy-only fixture correction after this baseline commit.

After a later optimization, the copy-text assertion belongs to this historical baseline checkpoint.
Keep output conformance active; preserve this manifest and commit as the literal-copy evidence.
