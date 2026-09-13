# S11 reference resize

Implements [issue 52](https://github.com/mia-cx/ditherette/issues/52) on `impl/v1-s11-resize`.
The original dependency is S03 PR89 at `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`.
The current PR base is `main`. [PR93](https://github.com/mia-cx/ditherette/pull/93) remains unmerged.
The original validated implementation checkpoint is `7f0436658967c45bf36edd32a2e5681cc39bd118`.

## Work

- [x] Repair naive trilinear mip construction for padded input views, with a failing then passing fixture.
- [x] Add the complete typed reference resize call and independently calculated recipe fixtures.
- [x] Update resize oracle coverage, run focused and integrated checks, and file the unmerged PR.

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

Original native validation reported 136 tests, including 11 complete resize fixtures and the Lanczos kernel fixture; the current verified count is below.
Wasm-target compilation, rustfmt, and `git diff --check` pass. No production files changed and no benchmark ran.
The oracle inventory and resize/trilinear documentation now describe the complete request and its stored-intermediate rounding.

## Stack-collapse validation

Merge `4327a75f` restacks S11 onto main `8547e29e` without conflicts or changes
to its resize implementation or numerical vectors. All 174 native tests pass,
including eleven complete resize fixtures and the Lanczos kernel fixture.
Benchmark-feature compilation, Wasm-target compilation, and rustfmt pass.
Anti-aliased bilinear, the original trilinear storage rules, and correct optimized
kernels remain unchanged. No standalone timing workload, browser run, or production
change occurred during collapse. The documentation amendment below changes no runtime behavior.

## Exact frozen-document amendment

`6e01ab32` corrects the S11 audit reference in
`crates/ditherette-wasm/src/spec/contract/inventory.md` from
`tests/spec_resize_contract.rs` to `../tests/spec_resize_contract.rs`.
The inventory declares paths relative to the crate's `src/` directory; the old
path does not exist and the corrected path names the actual test file.

The same typo exists at frozen checkpoint `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
At S18, preserve that original identity and add this exact one-line documentation
correction to the already approved S03 request/four-document amendment. Validate
only those named differences and keep the freeze guard strict. No kernel, formula,
vector, or runtime behavior changes.
