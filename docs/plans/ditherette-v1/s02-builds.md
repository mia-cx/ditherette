# S02 package and build ownership

Issue [#43](https://github.com/mia-cx/ditherette/issues/43). Decision [#36](https://github.com/mia-cx/ditherette/issues/36) fixes package ownership and toolchain pins.

- Branch: `impl/v1-s02-builds`, based on `impl/v1-s01-anchor` at `a213effed4b426c5c432c9ccc7062b7016dd5c1b`.
- Ownership: manifests, build scripts, and inert npm scaffold. Semantic kernels and the benchmark transport remain unchanged.
- PR and validated delivery head: pending.

## Work

1. [x] Establish pinned workspaces, crate-owned builds/tests, inert package distribution, and legacy URL staging.
2. [ ] Build both variants; verify memory maxima, package imports, version alignment, and focused correctness checks.
3. [ ] Record evidence and file the unmerged PR against the anchor.

The website stays in the root workspace. Generated crate and package `dist/` output remains ignored. The package manifest owns version `0.1.0`; its build and check commands reject a mismatched Cargo version. Publication remains held through `private: true` while the package is a scaffold.

The pnpm 11 migration preserves the four previously approved build scripts and adds the pinned wasm-pack install script. This script installs wasm-pack's platform binary; no floating package invocation remains.

## Validation

Both crate variants and the complete package build pass. Frozen pnpm install and package version/type checks pass. Detailed artifact and correctness evidence follows in the validation commit. No performance measurements belong to this slice.
