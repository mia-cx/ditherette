# ditherette-wasm

Rust/Wasm scratch package for porting Ditherette's image-processing core.

## Current API

- `hello(name)` — scaffold export.
- `resize_rgba_nearest(source_rgba, source_width, source_height, output_width, output_height)` — resizes tightly packed browser-order RGBA bytes with deterministic nearest-neighbor sampling.
- `resize_rgba_nearest_into(source_rgba, source_width, source_height, output_width, output_height, output_rgba)` — allocation-free form that writes into a caller-owned RGBA buffer.

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

Run the Rust kernel benchmark:

```sh
pnpm bench:rust-resize
```

Pass Criterion options when you want a longer or shorter run:

```sh
pnpm bench:rust-resize --measurement-time 30 --sample-size 100 --warm-up-time 5
```

The npm script clears Criterion's saved benchmark state and disables plots so the output keeps Criterion's normal statistics without confusing deltas against previous runs or stale plot artifacts. After Criterion finishes, it prints a percentile summary from Criterion's raw samples with mean-vs-baseline speedup factors. Before sampling each scale, the benchmark checks every variant's full byte output against the baseline so correctness failures do not affect measured timings.

Run the real-browser nearest-neighbor resize smoke benchmark:

```sh
pnpm bench:wasm-resize
```

Both benchmarks use `benchmark-fixtures/Celeste_box_art_full.png` by default at 0.95x, 0.75x, 0.5x, 0.25x, and 0.125x. They compare the simple reference baseline against benchmark-only variants and the production nearest paths. The Rust benchmark times kernels over decoded RGBA memory; the browser benchmark includes the generated Wasm package and reports browser decode/normalization context.
