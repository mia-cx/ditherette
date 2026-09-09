# #87 Historical TypeScript benchmark provider

## Scope

Preserve the S41 TypeScript provider while S45 removes live website processing.
This work owns benchmark preparation only. Root owns website retirement and the held PR.
Base is S44 PR #133 at `76bf1f8938c14a1a4cde290d45c61a280f7434d2`.

## TODO

- [x] Compile the pinned Git closure, distinguish provider/package provenance, remove the live adapter, and verify offline behavior.
- [x] Restore the strict public provenance schema and verify the actual Rust decoder.

## Contract

Read every provider input from S41 commit `a895267baea624a6e89bfcef6c5147f170e8a8f7`.
Keep compiler options, module rewriting, emitted layout, input hashes, and recipe admission unchanged.
Missing Git objects fail explicitly. Preparation never fetches or substitutes current website files.
The historical provider is not the current website backend or the frozen Rust correctness reference.

Retirement activation still requires Mia's rollout acceptance and no blocking regressions.
No public API, runtime, Rust spec, image, or guard change belongs to this task.

## Provenance and validation

`compiler-inputs.json` records the current package checkout HEAD as `source_revision`.
Its `provider` object records `kind: historical-website-typescript` and the separate historical `source_revision`.
Public `build-provenance.json` keeps its current package `source_revision` and its existing strict schema.
Its TypeScript inventory hashes `compiler-inputs.json`, which contains the explicit historical provider identity.
No duplicate provider field belongs at the public provenance top level.
Existing compiler identity, options, input hashes, and built-file inventories remain intact.
Git replacement objects and automatic lazy fetching are disabled for provider reads.

The removed live `scripts/benchmark-typescript.ts` remains the emitted historical entry at `scripts/benchmark-typescript.js`.
Its pinned source SHA-256 is `0fea5893cbfaf1de96f67e2ba8898651c52411d9e7cd84e6cde9a425911abf04`.
No legacy implementation is copied into the current website or scripts tree.

`node --test scripts/prepare-benchmark-typescript.test.mjs scripts/prepare-public-benchmark.test.mjs` passes all eight tests without skips.
The provider test uses a Git checkout containing only `.git`, compares every input hash against committed S41 bytes,
compares the emitted adapter with the historical adapter, and exercises existing output ownership and admission checks.
A repository without S41 history fails explicitly before creating output.
Prettier, focused ESLint, and `git diff --check` pass.

The initial provider validation runs no Rust build, browser, or benchmark. Tests remove their temporary Git repositories and emitted modules.
No `target/` tree exists. Formatting preparation installs ignored dependencies and runs SvelteKit sync;
these generated directories are removed before handoff. All owned processes exit.
The later website join and its validation are recorded in [87-retirement.md](87-retirement.md).

## Post-join strict-schema correction

At joined base `401bc85d0aa6915f27b80008299e0e358041ac1f`, the added top-level `typescript_provider` breaks Rust's `BuildProvenance` decoder.
Its `deny_unknown_fields` contract already has the required TypeScript file inventory. Remove the duplicate field and unused assignment.
The initial eight Node checks do not cover this Rust boundary.

A temporary Node probe evaluates the actual producer's provenance object expression, selected through its TypeScript AST.
It uses the real historical compiler and file inventory. Non-TypeScript inventories are empty decoder fixtures, not build evidence.
A temporary native example passes these bytes to `serde_json::from_slice::<BuildProvenance>` from the actual benchmark crate.
The original producer fails with exit 1:

```text
unknown field `typescript_provider`, expected one of `schema`, `source_revision`, `build_mode`, `tools`, `inputs`, `package`, `typescript`, `scripts` at line 4 column 23
```

The corrected producer passes with schema 1, source `401bc85d0aa6915f27b80008299e0e358041ac1f`, and 25 TypeScript inventory files.
An exact comparison confirms removal of that field is the only fixture change.
The inventory includes 24 emitted modules and `compiler-inputs.json`; its recorded digest matches the actual manifest bytes.
The historical provider revision remains distinct from the current source revision.

| Proof bytes               | SHA-256                                                            |
| ------------------------- | ------------------------------------------------------------------ |
| Rejected fixture          | `c5232e88f10b872d1e625a3ce516928378b7f34a3299da2c8fb316ee4638a0ec` |
| Accepted fixture          | `036f40d8fc680acd5cc2004a8cd2a0bbc3a335e12dbb15af0ef51c7bb3a8a9b9` |
| Compiler manifest         | `63e5b82c6c3bb69b0f868d24e1a21bdce1564f73991a4f655be21afd1bd6595d` |
| Corrected public preparer | `c65f378584cce7501742d623d5b0523fb14a8a4f26b1ccad8530e51d33c1910f` |

The same eight Node preparation checks pass again. Existing Rust protocol coverage passes all ten checks:

```sh
cargo test --offline --locked --manifest-path crates/ditherette-bench/Cargo.toml --target-dir target/s45-provenance-proof/compiler --test browser_assets
```

Prettier, focused ESLint, and diff checks pass. This is a decoder/protocol proof, not complete fresh `prepare-browser` validation.
Only a native debug decoder/test build runs. No package/Wasm build, browser, benchmark, or measurement runs.
Cleanup removes the temporary example and entire owned `target/` tree after retaining this proof; the tree occupies 1,312,940 KiB.
All owned jobs exit. Existing S43 package and earlier website validation identities remain unchanged.
