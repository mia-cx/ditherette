# #232 Resize kernels bleed hidden colour into transparent areas

## Summary

Every filtering resize kernel averages channels independently, so the hidden RGB under transparent pixels bleeds into edges, and Lanczos/bicubic ring in alpha, which alpha thresholds then treat as opaque. Fix all kernels except nearest.

## Acceptance criteria

- [x] Hidden colour under fully transparent pixels never reaches the output of any filtering kernel.
- [x] Ringing kernels leave fully transparent neighbourhoods transparent.
- [x] Nearest and fully opaque images keep their exact bytes and speed.
- [x] The website output for transparent sprites shows no black halos or ghost bands.

## TODOs

- [x] Spec: coverage-weighted resize as a freeze extension; recipe v2 `process` resizes through it before frozen v1 `process`.
- [x] Prod: match the new v2 reference byte for byte, with charged, fallible allocations and measured cost.
- [x] Website: always send recipe v2 so every resize is coverage-weighted.
- [x] Validate: cargo tests, freeze guard, package and website tests, render a transparent sprite in debug Chromium.

## Notes

- The frozen v1 resize stays unchanged: the benchmark oracle and conformance identity pin v1. Recipe v2 is unreleased, so its resize can change.
- The coverage path reuses the v1 spec kernels on a premultiplied `f32` carrier. Alpha clamps to the main-lobe range only for kernels with negative lobes.
- Validation: `cargo test --locked` (all crate tests, including a 2880-case prod-vs-spec v2 matrix), `cargo fmt --check`, `node tools/spec-freeze/guard.mjs`, package interface tests (60), website `vitest run` (151), and a 24x12 transparent sprite upscaled 20x with Lanczos3 and bilinear in debug Chromium: no halos, blobs, or bands.
- Prod coverage timings: Lanczos3 800x800 to 400x400 in 34 ms, bilinear in 5.6 ms (`src/prod/coverage/README.md`). The prod path was implemented by gpt-5.6-sol from a reviewed plan.
