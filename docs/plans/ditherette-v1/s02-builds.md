# S02 package and build ownership

Issue [#43](https://github.com/mia-cx/ditherette/issues/43). Decision [#36](https://github.com/mia-cx/ditherette/issues/36) fixes package ownership and toolchain pins.

- Branch: `impl/v1-s02-builds`, originally based on `impl/v1-s01-anchor` at `a213effed4b426c5c432c9ccc7062b7016dd5c1b`. Restacked onto merged root `db1c3f67` through merge `a53aef27`.
- Ownership: manifests, build scripts, and inert npm scaffold. Semantic kernels and the benchmark transport remain unchanged.
- PR: [#88](https://github.com/mia-cx/ditherette/pull/88), open and non-draft. Validated implementation `1cbe4999c29b9a5e4caf9cf0a4f4836b5e8f4f5c`; evidence `ba7dd082bc4016cead440a4b07e2f7f96c1d77f4`. The subsequent delivery commit only records this PR.

## Work

1. [x] Establish pinned workspaces, crate-owned builds/tests, inert package distribution, and legacy URL staging.
2. [x] Build both variants; verify memory maxima, package imports, version alignment, and focused correctness checks.
3. [x] Record evidence and file the unmerged PR against the anchor.

The website stays in the root workspace. Generated crate and package `dist/` output remains ignored. The package manifest owns version `0.1.0`; its build and check commands reject a mismatched Cargo version. Publication remains held through `private: true` while the package is a scaffold.

The pnpm 11 migration preserves the four previously approved build scripts and adds the pinned wasm-pack install script. This script installs wasm-pack's platform binary; no floating package invocation remains.

## Current build pin

Mia approved pnpm 11.13.1 for this slice and downstream conformance/publishing workflows. Both `packageManager` and `engines.pnpm` use that version. npm deprecated 11.13.0 because its native `@pnpm/exe` packages lack the executable. A cached JavaScript installation passing locally does not validate that native installation path.

The restacked baseline at `a53aef27` passed 102 native tests, two Node Wasm storage-overflow tests, 120 processing/wrapper tests across 16 files, and 15 deterministic benchmark Node tests. The historical validation section describes the original implementation checkpoint, not the current test counts.

### Native 11.13.1 validation

Validated on 2026-09-12 from `a53aef27` with the pnpm pin correction. A fresh isolated `@pnpm/exe@11.13.1` installation supplied pnpm, with Node 24.19.0 on PATH. No global installation or settings changed.

| Command | Result |
| --- | --- |
| `pnpm install --frozen-lockfile` | Passed for all three workspaces; lockfiles unchanged. |
| `pnpm package:build` | Scalar and threaded Wasm builds, TypeScript output, and package staging passed. |
| `pnpm package:check` | Package/Cargo version alignment and TypeScript checks passed. |
| `pnpm --filter ditherette-wasm check` | Rustfmt and Wasm-target compilation passed. |
| `cargo check --locked --manifest-path crates/ditherette-wasm/Cargo.toml` | Native compilation passed. |
| `pnpm wasm:test:native` | 102 native tests passed. |
| `pnpm wasm:test` | Two Node Wasm storage-overflow tests passed; other binaries contain no Wasm tests. |
| `pnpm exec vitest run --project server src/lib/processing src/lib/wasm/ditherette-wasm.spec.ts` | 120 tests across 16 files passed. |
| `node --test scripts/benchmark-wasm-resize.test.mjs` | 15 deterministic tests passed. |
| `git diff --check` and generated-output ignore checks | Passed. |

Rust 1.97.0, nightly-2024-08-02, wasm-pack 0.15.0, and `opt-level = "s"` remain unchanged. No standalone benchmark timing workload or publication ran. Native benchmark-export smoke tests are not performance evidence.

## Historical validation

Validated implementation checkpoint: `1cbe499`. All commands ran in the isolated S02 worktree.

| Command or assertion | Result |
| --- | --- |
| `pnpm install --frozen-lockfile` | Passed for all three workspaces. |
| `pnpm wasm:build` and `pnpm wasm:build:threads` | Passed; existing website URLs contain the generated variants. |
| `pnpm package:build` | Passed; wrapper JavaScript/declarations, both variants, worker helper, and MIT license staged. |
| `pnpm package:check` | Passed. A temporary Cargo version mismatch correctly failed; restoring `0.1.0` passed. |
| `pnpm --filter ditherette-wasm check` | Rust formatting and Wasm-target compilation passed. |
| `pnpm wasm:test:native` | 110 native tests passed. |
| `pnpm exec vitest run --project server src/lib/processing src/lib/wasm/ditherette-wasm.spec.ts` | 120 tests across 16 files passed. |
| `pnpm wasm:test` | Compilation and runner succeed, but discover zero Wasm tests. This inherited coverage gap remains explicit. |
| Direct package/artifact assertions | Inert empty public import with throwing fetch/worker guards; valid Wasm bytes; identical crate, distribution, and legacy binaries; threaded worker and matching license present. |
| Generated scalar export | `initSync` plus nearest resize of `[12,34,56,255]` from 1x1 to 2x2 returns four identical pixels. |
| `git check-ignore` and `git diff --check` | Generated crate/package output, copied package license, and legacy artifacts remain ignored; diff is clean. |

Using wasm-pack's cached `wasm-opt --print --all-features`, the scalar memory declaration is `(memory $0 17 32768)`. The threaded declaration is `(memory $mimport$0 18 32768 shared)`. Both maxima are 32768 64-KiB pages, or 2 GiB.

Artifact SHA-256 digests:

- Scalar: `4d0da1bb4ddeebcb27fef430e9fcbc25c5b02a63eb2770810c3bd8704800b307`.
- Threaded: `fcae89278846ef91da74a69dc6bc2701e260ed5b31a80165e248ed1bcf1293ac`.

Node 24.19.0, pnpm 11.13.0, Rust 1.97.0, nightly-2024-08-02, and wasm-pack 0.15.0 supplied these builds. Threaded compilation retains Rust's expected unstable-atomics warning. The script stages the root MIT license after wasm-pack's crate-local license warning.

The two former `latest` website dependencies now pin their prior lockfile versions, preserving existing resolution during workspace installation. No processing source or benchmark transport changed. No benchmark process ran; native export smoke tests do not provide performance evidence. Browser conformance, worker lifecycle, packed-tarball validation, and publication automation remain later slices.

Build flag behavior follows [wasm-pack's build documentation](https://drager.github.io/wasm-pack/book/commands/build.html); the installed 0.15.0 `test --help` establishes test argument forwarding.
