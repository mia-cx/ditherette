# ditherette-wasm

Internal MIT-licensed Rust/Wasm processing crate. It owns compilation and Rust/Wasm test commands. The public browser package lives in `packages/ditherette`; this crate is not published.

## Build and test

Install workspace dependencies with Node 24.19.0 and pnpm 11.13.1, then run these commands from this directory:

- `pnpm build` builds scalar and threaded variants into ignored `dist/scalar` and `dist/threads` directories.
- `pnpm test` runs the native Rust correctness suite.
- `pnpm test:wasm` and `pnpm test:browser` invoke wasm-pack's Node and headless Chrome runners.
- `pnpm check` checks formatting and compilation for the Wasm target.

The current suite has 102 native tests and two Node Wasm tests for storage-overflow errors. Most Rust tests still use native `#[test]` attributes; broader browser/Wasm conformance remains for later slices.

Scalar compilation uses the root Rust pin. Threaded compilation uses `rust-toolchain-threads.toml` and requires that toolchain's `rust-src` component. Both variants declare a 2 GiB memory maximum and use separate Cargo target directories.

Root `wasm:build` commands stage generated artifacts into the website's existing `static/wasm` URLs. The public package's build stages both variants and worker files into its own distribution. Generated files remain private and ignored.

## GPU backend

The `gpu` feature adds `prod::gpu`, a `wgpu` backend with WGSL kernels and the CPU path as fallback. Default and threaded builds leave it out, so their Wasm bytes do not change. It needs the root Rust pin, so it does not combine with `threads` yet.

- `cargo test --features gpu` runs the fallback tests. GPU tests accept software adapters and skip when there is none.
- `pnpm gpu:parity [WIDTH HEIGHT [RAW_RGBA_FILE]]` runs the channel-lookup stage natively on both backends and prints the largest byte difference and how many pixels differ.
- `pnpm gpu:parity:browser` does the same in headless Chromium with WebGPU, uploading through `copyExternalImageToTexture`. `--no-webgpu` shows the fallback.

`packages/ditherette/package.json` owns the release version. Its build and check commands verify that this crate's version agrees.
