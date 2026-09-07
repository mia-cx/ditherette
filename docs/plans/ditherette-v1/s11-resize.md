# S11 reference resize

Implements [issue 52](https://github.com/mia-cx/ditherette/issues/52) on `impl/v1-s11-resize`.
The dependency is S03 PR89 at `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`.
The PR base is `impl/v1-s03-contracts`.

## Work

- [x] Repair naive trilinear mip construction for padded input views, with a failing then passing fixture.
- [ ] Add the complete typed reference resize call and independently calculated recipe fixtures.
- [ ] Update resize oracle coverage, run focused and integrated checks, and file the unmerged PR.

Only reference resize code, its tests, and its inventory change. No production code or benchmark timings belong here.

## Findings

Trilinear copies the complete backing slice into packed mip storage, including padding. Copy logical rows instead.
Bilinear is the existing scale-aware triangle filter, including within trilinear. Its reference is not a two-tap minification shortcut.

## Evidence

The padded-input fixture first panicked with `BufferLengthMismatch { len: 36, expected: 32 }`.
After copying logical rows, `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_resize_contract` passes.
