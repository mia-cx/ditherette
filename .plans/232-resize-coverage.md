# #232 Resize kernels bleed hidden colour into transparent areas

## Summary

Every filtering resize kernel averages channels independently, so the hidden RGB under transparent pixels bleeds into edges, and Lanczos/bicubic ring in alpha, which alpha thresholds then treat as opaque. Fix all kernels except nearest.

## Acceptance criteria

- [ ] Hidden colour under fully transparent pixels never reaches the output of any filtering kernel.
- [ ] Ringing kernels leave fully transparent neighbourhoods transparent.
- [ ] Nearest and fully opaque images keep their exact bytes and speed.
- [ ] The website output for transparent sprites shows no black halos or ghost bands.

## TODOs

- [ ] Spec: coverage-weighted resize as a freeze extension; recipe v2 `process` resizes through it before frozen v1 `process`.
- [ ] Prod: match the new v2 reference byte for byte, with charged, fallible allocations and measured cost.
- [ ] Website: always send recipe v2 so every resize is coverage-weighted.
- [ ] Validate: cargo tests, freeze guard, package and website tests, render a transparent sprite in debug Chromium.

## Notes

- The frozen v1 resize stays unchanged: the benchmark oracle and conformance identity pin v1. Recipe v2 is unreleased, so its resize can change.
- The coverage path reuses the v1 spec kernels on a premultiplied `f32` carrier. Alpha clamps to the main-lobe range only for kernels with negative lobes.
