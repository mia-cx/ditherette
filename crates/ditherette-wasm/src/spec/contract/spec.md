# Processing contract spec

## Scope and composition

This domain defines the five request types, stable errors, and synchronous instance control rules.
It validates borrowed packed RGBA8 sources and describes output layouts without allocating result images.
It does not dispatch production kernels, load Wasm, run callbacks, or implement caches.

- [request.md](request.md) specifies typed settings, validation, and dimension limits.
- [error.md](error.md) specifies error codes and field paths.
- [lifecycle.md](lifecycle.md) specifies initialization, progress, disposal, and publication rules.
- [inventory.md](inventory.md) maps processing modes and adapters to their references and assigned slices.

Shared result types live in `crate::image::contracts`.
S09 owns palette normalization and alpha behavior. S07 through S17 complete the processing references.

## Invariants and production obligations

Source buffers remain borrowed and unchanged during validation. Successful calls return independent owned results.
Failures expose a structured error, never a partial image or newly published cache entry.
Explicit output settings and source-derived output dimensions retain distinct error categories.
Processing recipes exclude thread selection, cache identity controls, callbacks, and frontend identity.
Production may optimize allocation and execution while preserving these observable rules.

The contract accepts transparent-only and oversized palettes for later normalization.
It preserves the website's sRGB diffusion feedback with perceptual matching.
Threading and memory policy belong to initialization, not algorithm recipes.
