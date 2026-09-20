# Nearest-quarter scalar reproduction

On 2026-09-20, isolated builds reproduced all three measured scalar Wasm binaries byte-for-byte.
No timings ran. The historical sample counts, results, and release limitations remain unchanged.

## Source retention

Use the full commit IDs below, not mutable branch tips, when reproducing these builds.
The `evidence/nearest-quarter-accepted`, `evidence/nearest-quarter-global`, and
`evidence/nearest-quarter-selected` branches keep their Git objects reachable.

| Trial | Source commit | Measured and rebuilt scalar Wasm SHA-256 |
|---|---|---|
| Accepted | [29cfdb8cc6c4e6ed5a17647bd299591a346a6bf1](https://github.com/mia-cx/ditherette/commit/29cfdb8cc6c4e6ed5a17647bd299591a346a6bf1) | `6df1333251b763fe60cd955f20749ba888340ff7aee898540b7df4c3d6eff9ce` |
| Global cutoff | [520f2008d852eee6e06d8314393272f1cb8b6602](https://github.com/mia-cx/ditherette/commit/520f2008d852eee6e06d8314393272f1cb8b6602) | `3aa32efcc784f4025b08d0fc096da84126c121504aaac65e851e704bed7522b3` |
| Selected policy | [b72238d414395c4d1df06d70bbfeb78b7d9c415b](https://github.com/mia-cx/ditherette/commit/b72238d414395c4d1df06d70bbfeb78b7d9c415b) | `1e3646b8e49e1aac8f39d41d4e6e38b2259a1cf92dbdc872fb59028ee33edf99` |

The retained package files match their archived manifests. The coordinator independently rehashed all three retained Wasm binaries.
The three sources have identical `packages/ditherette/src` trees.

## Build and comparison

Each checkout was clean and detached at its recorded source commit. Tools were Node 24.19.0,
pnpm 11.13.0, wasm-pack 0.15.0, and Rust 1.97.0 (`2d8144b78`, 2026-07-07).
Each build used its source-pinned release profile and flags, with `CARGO_INCREMENTAL=0`.
Target, Rust flags, compiler-wrapper, toolchain, and debug-profile environment overrides were removed.

Install the locked dependencies in each isolated checkout:

```sh
pnpm install --frozen-lockfile --ignore-scripts
```

The accepted checkout used `pnpm --filter ditherette build`.
The global and selected checkouts used `pnpm --filter ditherette-wasm build:scalar`.
All three commands exited successfully. Compare this generated file against the table:

```sh
sha256sum crates/ditherette-wasm/dist/scalar/ditherette_wasm_bg.wasm
```

Each comparison also checked the scalar JS factory, generated JS, and three helper snippets against the archived file hashes.
All six scalar files matched in each trial. The rebuilt Wasm hashes above were captured explicitly.
The five matching glue/snippet hashes were compared but not separately printed before build-output cleanup.

The archived worker calls `createDitherette({ threads: 'disabled' })`.
Its manifest hashes `wasm/scalar/ditherette_wasm_bg.wasm`, so these are the binaries used by the historical trials.

## Scope

This is a scalar reproduction record, not a full-package or threaded attestation.
The accepted package rebuild matched 19 of 23 manifest files. Four unused threaded files differed:
the threaded Wasm, generated JS, factory, and copy helper. Their original build difference remains unexplained.
Global and selected package-wrapper sources are unchanged from accepted; no separate wrapper rebuild is claimed for them.

The original package directories and reproduction logs remain outside tracked source.
No compiled package, fixture, or new evidence archive was added to Git.
This record restores the source-to-scalar-binary checks needed to inspect the historical cutoff comparison.
It does not satisfy the broader performance release gate or lift any release hold.
