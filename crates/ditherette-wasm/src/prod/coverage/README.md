# Coverage resize

Recipe v2 scans the verified post-effects RGBA8 snapshot once. Nearest and opaque
inputs stay on the existing packed production resize path. Translucent filtered
inputs use premultiplied `f32` source and output carriers, exact per-axis tap
plans, scalar reference-order accumulation, and coverage-only alpha clamping for
bicubic and Lanczos.

All carrier and tap-plan reservations are fallible. Their actual capacities are
charged as working memory before downstream preparation. The final RGBA8 pixels
reuse the pipeline's existing resize scratch buffer.

## Measurement

Measured on 2026-09-27 with an AMD Ryzen 9 7950X, Linux 6.12.95, and the Cargo
release profile. The fixture was the repository's 800×800 RGB image decoded to
RGBA8. It is a larger stand-in for the planned 512×512 sprite sheet. The
benchmark calls the coverage stage directly, so it measures the same carrier
and kernel work while excluding opacity dispatch, dither, quantization, and
package-boundary copies.

```text
DITHERETTE_BENCH_QUIET=1 cargo run --release --locked \
  --manifest-path crates/ditherette-bench/Cargo.toml -- perf resize \
  --subjects prod:coverage:bilinear:scalar,prod:coverage:lanczos3:scale-aware \
  --fixtures Celeste_Insta_selfie.png --scales 0.5,2 \
  --sample-size 8 --measurement-time 2s --warm-up-time 300ms \
  --preheat-time 0ms --process-priority normal --correctness exact --baseline none
```

| Resize | Median | Coverage-owned peak capacity |
| --- | ---: | ---: |
| 800×800 → 400×400, bilinear | 5.55 ms | 12.28 MiB |
| 800×800 → 400×400, Lanczos3 scale-aware | 33.97 ms | 12.38 MiB |
| 800×800 → 1600×1600, bilinear | 57.26 ms | 49.04 MiB |
| 800×800 → 1600×1600, Lanczos3 scale-aware | 215.36 ms | 49.23 MiB |

The registered production subjects have exact reference subjects, and the
integration matrix separately checks every output byte. Per-axis preparation is
included in each timing. No separable reordering, sparse coverage, bands, SIMD,
or threaded coverage is used.
