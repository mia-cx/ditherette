# #87 Historical TypeScript benchmark provider

## Scope

Preserve the S41 TypeScript provider while S45 removes live website processing.
This work owns benchmark preparation only. Root owns website retirement and the held PR.
Base is S44 PR #133 at `76bf1f8938c14a1a4cde290d45c61a280f7434d2`.

## TODO

- [x] Compile the pinned Git closure, distinguish provider/package provenance, remove the live adapter, and verify offline behavior.

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
Public `build-provenance.json` keeps its current package `source_revision` and adds that object as `typescript_provider`.
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

No Rust build, browser, or benchmark runs. Tests remove their temporary Git repositories and emitted modules.
No `target/` tree exists. Formatting preparation installs ignored dependencies and runs SvelteKit sync;
these generated directories are removed before handoff. All owned processes exit.
Root still needs to join this commit with website retirement and validate the combined diff.
