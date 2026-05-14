# ditherette-wasm

Rust/Wasm scratch package for porting Ditherette's image-processing core.

## Current API

- `hello(name)` — scaffold export.
- `resize_rgba_nearest(source_rgba, source_width, source_height, output_width, output_height)` — resizes tightly packed browser-order RGBA bytes with deterministic nearest-neighbor sampling.
- `resize_rgba_nearest_into(source_rgba, source_width, source_height, output_width, output_height, output_rgba)` — allocation-free nearest-neighbor form that writes into a caller-owned RGBA buffer.
- `resize_rgba_bilinear(source_rgba, source_width, source_height, output_width, output_height)` — resizes tightly packed browser-order RGBA bytes with deterministic center-aligned bilinear interpolation.
- `resize_rgba_bilinear_into(source_rgba, source_width, source_height, output_width, output_height, output_rgba)` — allocation-free bilinear form that writes into a caller-owned RGBA buffer.
- Reference resamplers: `resize_rgba_trilinear`, `resize_rgba_bicubic`, `resize_rgba_lanczos2`, `resize_rgba_lanczos3`, `resize_rgba_lanczos2_scale_aware`, `resize_rgba_lanczos3_scale_aware`, `resize_rgba_area`, and `resize_rgba_box`.
- `antialias_rgba_box3(source_rgba, width, height)` — applies a simple post-resize 3x3 box antialiasing pass for comparison.

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

Run the Rust nearest-neighbor kernel benchmark:

```sh
pnpm bench:resize
```

Run the Rust bilinear kernel benchmark:

```sh
pnpm bench:resize:bilinear-criterion
```

Run all canonical Rust resize filters in Criterion. Before sampling, the bench reports byte equality for local `<filter>` vs `<filter>_reference` and, where useful, for `<filter>_reference` vs matching `image` crate output; mismatches are reported but do not stop the benchmark.

```sh
pnpm bench:resize:filters
```

Run one filter at a time:

```sh
pnpm bench:resize:nearest
pnpm bench:resize:nearest:aa
pnpm bench:resize:bilinear
pnpm bench:resize:trilinear
pnpm bench:resize:bicubic
pnpm bench:resize:lanczos2
pnpm bench:resize:lanczos2-scale-aware
pnpm bench:resize:lanczos3
pnpm bench:resize:lanczos3-scale-aware
pnpm bench:resize:area
pnpm bench:resize:box
pnpm bench:resize:antialias
```

Pass Criterion options when you want a longer or shorter run:

```sh
pnpm bench:resize --measurement-time 30 --sample-size 100 --warm-up-time 5
pnpm bench:resize:bilinear-criterion --measurement-time 30 --sample-size 100 --warm-up-time 5
pnpm bench:resize:filters --measurement-time 30 --sample-size 100 --warm-up-time 5
```

The npm script preserves Criterion's saved benchmark state so `--save-baseline` and `--baseline` can show regressions and speedups. After Criterion finishes, it prints a percentile summary from Criterion's raw samples. Before sampling each scale, the benchmark checks selected canonical filters against their reference implementations.

Run the real-browser nearest-neighbor resize smoke benchmark:

```sh
pnpm bench:resize:wasm
```

Both benchmarks use `benchmark-fixtures/Celeste_box_art_full.png` by default at 2x, 0.95x, 0.75x, 0.5x, 0.25x, and 0.125x. Criterion measures canonical production kernels only; reference implementations run in preflight correctness checks. The Rust benchmark times kernels over decoded RGBA memory; the browser benchmark includes the generated Wasm package and reports browser decode/normalization context.
