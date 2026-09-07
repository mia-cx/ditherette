# S19 prerequisite preparation

Issue [#60](https://github.com/mia-cx/ditherette/issues/60) requires S02, S06, and S18.
This file records integration preparation only. Production implementation waits for the validated S18 freeze.

## Join

The pre-existing `impl/v1-s19-base` branch pointed to S02 when its worktree was materialized.
The coordinator merged S17 and S06 without conflicts, producing `ad2410b481d710fb2d739695ec1637d6d08eab79`.

| Prerequisite | Included delivered commit |
| --- | --- |
| S02 package/build ownership | `bc110d91d441ec3069d128e3cc7d39b3b439a1f4` |
| S06 paired benchmark tooling | `a65f53e24b880932021b39e601b5682b1111bc54` |
| S17 complete reference, pending S18 freeze | `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b` |

Each commit passes `git merge-base --is-ancestor` against the join.
`spec/`, `image/`, the Wasm Cargo lockfile, and the benchmark Cargo lockfile are unchanged from S17.
Resolved serde_json features remain alloc/default/std for the core test graph and default/std for the benchmark graph.
Neither graph enables preserve_order.

## Validation

All commands ran in this isolated worktree at the joined code checkpoint.
No actual benchmark measurement ran.

- `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --features bench-subjects` passed.
- `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown --features bench-subjects` passed.
- `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` passed.
- `cargo test --manifest-path crates/ditherette-bench/Cargo.toml --locked --bins --test reference_subjects --test verification --test verification_adapters --test lease --test paired` passed.
- The combined benchmark run includes 4 binary, 4 paired, 5 reference-subject, 8 verification, and 3 adapter tests. The lease lifecycle fixture passes, including its three owned Node transport checks.
- `cargo check --manifest-path crates/ditherette-bench/Cargo.toml --locked --benches` passed.
- `cargo fmt --manifest-path crates/ditherette-bench/Cargo.toml --check` passed.

## Remaining prerequisite

- [ ] Merge the delivered S18 head into this join.
- [ ] Run its trusted freeze guard against this resolved dependency/build graph.
- [ ] Record all prerequisite SHAs, remove only satisfied native blocking edges, and start the S19 implementation worktree.

All PRs stay unmerged. No production copy or optimization has started.
