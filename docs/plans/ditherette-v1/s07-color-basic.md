# S07 ordinary color references

Issue [#48](https://github.com/mia-cx/ditherette/issues/48). Original base and dependency: S03 at `fa3007fffc9e4ca9a85c19c4d6e06ebedb41bd06`, branch `impl/v1-s03-contracts`, PR #89.

Branch: `impl/v1-s07-color-basic`. Ownership is limited to the sRGB, linear RGB, and YCbCr reference modules/docs and their tests. Production code, common helpers, and shared module exports remain unchanged.

1. [x] Define and test per-pixel forward/inverse equations and RGBA8 reconstruction with unchanged alpha.
2. [x] Validate inherited color tests, full native tests, Wasm-target compilation, and formatting; record exact evidence.
3. [x] File an unmerged stacked PR against S03.

The per-space pixel helpers accept RGB byte triplets or f32 working triplets. Image inverse adapters accept the working image, corresponding RGBA8 alpha source, and mutable RGBA8 output. Final encoded RGB clips to `[0,1]`; byte ties round upward. Canonical coordinate domains remain unchanged. S08 uses the same reconstruction convention for perceptual spaces; S10 can join dispatch after both dependencies are available.

The [space references](../../../crates/ditherette-wasm/src/spec/color/) record primary formula sources and numerical conventions. This slice runs correctness checks only; no benchmarks or optimization candidates.

Original validated implementation: `0a1ff0f84d599c4f4dbadf9b43e235262319518f`.

| Command | Result |
| --- | --- |
| `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked --test spec_color_basic --test spec_color_spaces` | Ten new and six inherited color tests pass. |
| `cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --locked` | Historical report of 134; superseded by the verified restack count below. |
| `cargo check --manifest-path crates/ditherette-wasm/Cargo.toml --locked --target wasm32-unknown-unknown` | Passed. |
| `cargo fmt --manifest-path crates/ditherette-wasm/Cargo.toml --check` | Passed. |
| `git diff --check` | Passed. |

Tests include primary/neutral vectors, both transfer thresholds, exact byte ties, clipping of out-of-gamut reconstruction, all 256 neutral/single-channel byte values, and 1331 mixed colors. Strided image tests use independent source, alpha, working-color, and destination strides; verify byte-alpha preservation including zero; preserve hidden RGB; and check input/padding integrity.

The f32 references use direct arithmetic and powers. No LUT, approximate conversion, production call, or benchmark process was introduced. Native export smoke tests do not provide performance evidence. Wasm-target compilation is not browser execution; later conformance slices own that coverage.

PR [#91](https://github.com/mia-cx/ditherette/pull/91) was originally filed non-draft against S03. Evidence checkpoint: `1ba15ec9d01a0f1ef7d1c8b71124842841a2a439`. These describe the initial delivery; the PR now targets main after S03 merged.

## Stack-collapse validation

Merge `0eee8606` restacks S07 onto main `fd035567` without conflicts or changes
to its color implementations. The current native suite passes 128 tests, including
ten ordinary-color tests, six inherited color tests, and sixteen contract tests.
Wasm-target compilation and rustfmt also pass. The older report above is not the
current test count. No standalone benchmark workload or browser run occurred.
