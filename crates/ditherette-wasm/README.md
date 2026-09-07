# ditherette-wasm

Internal MIT-licensed Rust/Wasm processing crate. It owns compilation and Rust/Wasm test commands. The public browser package lives in `packages/ditherette`; this crate is not published.

## Build and test

Install workspace dependencies with the repository's pinned Node and pnpm versions, then run these commands from this directory:

- `pnpm build` builds scalar and threaded variants into ignored `dist/scalar` and `dist/threads` directories.
- `pnpm test` runs the native Rust correctness suite.
- `pnpm test:wasm` and `pnpm test:browser` invoke wasm-pack's Node and headless Chrome runners.
- `pnpm check` checks formatting and compilation for the Wasm target.

The existing Rust tests use native `#[test]` attributes. The Wasm runner currently compiles them but discovers no browser/Node Wasm tests; later conformance slices must add actual Wasm coverage.

Scalar compilation uses the root Rust pin. Threaded compilation uses `rust-toolchain-threads.toml` and requires that toolchain's `rust-src` component. Both variants declare a 2 GiB memory maximum and use separate Cargo target directories.

Root `wasm:build` commands stage generated artifacts into the website's existing `static/wasm` URLs. The public package's build stages both variants and worker files into its own distribution. Generated files remain private and ignored.

`packages/ditherette/package.json` owns the release version. Its build and check commands verify that this crate's version agrees.
