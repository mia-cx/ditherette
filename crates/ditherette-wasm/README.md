# ditherette-wasm

Rust/Wasm scratch package for porting Ditherette's image-processing core.

## Current API

- `hello(name)` — scaffold export.
- `resize_rgba_nearest(source_rgba, source_width, source_height, output_width, output_height)` — resizes tightly packed browser-order RGBA bytes with deterministic nearest-neighbor sampling.

## Prerequisites

Use Rust through `rustup`; the repo-level `rust-toolchain.toml` requests the `wasm32-unknown-unknown` target for `wasm-pack` builds.

## Build

```sh
pnpm wasm:build
```

Or, with `wasm-pack` installed locally/globally:

```sh
wasm-pack build crates/ditherette-wasm --target web
```

From inside this directory:

```sh
wasm-pack build --target web
```

## Test

```sh
cargo test --manifest-path crates/ditherette-wasm/Cargo.toml
pnpm wasm:test
pnpm wasm:test:browser
```

## Benchmark

Run the real-browser nearest-neighbor resize smoke benchmark:

```sh
pnpm bench:wasm-resize
```

The benchmark uses `benchmark-fixtures/Celeste_box_art_full.png` by default at 0.95x, 0.75x, 0.5x, 0.25x, and 0.125x. Each scale runs in both lanes: browser-decoded RGBA and predecoded RGBA reuse.
