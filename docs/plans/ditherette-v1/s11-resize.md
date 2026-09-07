# S11 reference resize

Implements [issue 52](https://github.com/mia-cx/ditherette/issues/52) on `impl/v1-s11-resize`.
The dependency is S03 PR89 at `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`.
The PR base is `impl/v1-s03-contracts`.

## Work

- [x] Repair naive trilinear mip construction for padded input views, with a failing then passing fixture.
- [x] Add the complete typed reference resize call and independently calculated recipe fixtures.
- [ ] Update resize oracle coverage, run focused and integrated checks, and file the unmerged PR.

Only reference resize code, its tests, and its inventory change. No production code or benchmark timings belong here.

## Findings

Trilinear copies the complete backing slice into packed mip storage, including padding. Copy logical rows instead.
Bilinear is the existing scale-aware triangle filter, including within trilinear. Its reference is not a two-tap minification shortcut.

## Evidence

The padded-input fixture first panicked with `BufferLengthMismatch { len: 36, expected: 32 }`.
After copying logical rows, `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_resize_contract` passes.

The reference call covers every resize recipe and support policy. Fixtures check all nine anchors, nonconstant identity, magnification,
symmetric averages, cubic support weights, fractional LOD, ceil-halved mips, staged byte rounding, anisotropic LOD, and float samples.
Lanczos kernel checks use independently known half-angle sine values.

Final native validation passes 136 tests, including 11 complete resize fixtures and the Lanczos kernel fixture.
Wasm-target compilation, rustfmt, and `git diff --check` pass. No production files changed and no benchmark ran.
The oracle inventory and resize/trilinear documentation now describe the complete request and its stored-intermediate rounding.
