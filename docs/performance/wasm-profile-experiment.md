# Scalar Wasm compiler profile experiment

Mia approved comparing Cargo `opt-level=3` against frozen `opt-level=s` on 2026-09-11.
The default manifest remains `s`. Spec, filter behavior, wrapper, and wasm-opt settings remain unchanged.
The experiment improves several expensive paths, but does not meet the full JS target in issue #149.

## Size

| Artifact | Size profile | Speed profile | Increase |
|---|---:|---:|---:|
| Scalar Wasm | 386,643 B | 472,462 B | 22.2% |
| Scalar gzip, level 9 | 171,005 B | 192,339 B | 12.5% |
| Scalar Brotli | 138,729 B | 152,000 B | 9.6% |
| Complete npm archive | 420,195 B | 441,431 B | 5.1% |

Both npm archives contain the same 569,876-byte threaded Wasm. This measures a scalar-only profile change.
Local `npm pack --ignore-scripts` creates the archives. Nothing is published.

## Browser measurements

Celeste input is 2600 × 4168. Each profile runs 11 recipes across Chromium, optimizing Firefox, and WebKit.
Each state has six samples. Cold means a fresh worker and processor, excluding imports and creation.
Warm means a primed processor recomputing the target dimensions, not a final-result cache hit.
E2E dispatch and canvas submission remain separately recorded. They do not measure actual presentation.
Historical JS revision is `a895267baea624a6e89bfcef6c5147f170e8a8f7`.

Selected warm public-call medians, in milliseconds:

| Browser and recipe | Size Wasm | Speed Wasm | JS in speed trial |
|---|---:|---:|---:|
| Chromium area 5% resize | 42.70 | 24.30 | 44.00 |
| Chromium Lanczos3 50% process | 357.55 | 273.90 | 323.20 |
| Chromium Oklch 50% process | 82.35 | 97.90 | 86.50 |
| Chromium Floyd 50% process | 234.70 | 198.15 | 325.50 |
| Firefox area 5% resize | 44.00 | 28.00 | 48.00 |
| Firefox Lanczos3 50% process | 358.00 | 261.50 | 263.50 |
| Firefox Oklch 50% process | 81.00 | 78.50 | 120.00 |
| Firefox Floyd 50% process | 240.00 | 214.00 | 482.50 |
| WebKit area 5% resize | 37.00 | 24.00 | 40.50 |
| WebKit Lanczos3 50% process | 345.00 | 265.00 | 215.00 |
| WebKit Oklch 50% process | 97.00 | 103.00 | 136.00 |
| WebKit Floyd 50% process | 578.50 | 543.50 | 387.50 |

A fresh Chromium Oklch repeat reverses profile order. Warm medians are 83.85 ms for size and 88.45 ms for speed.
Cold medians are 89.70 and 98.70 ms. Palette-edit medians are 81.10 and 83.40 ms.
This repeat suggests a smaller regression than the initial warm result. It does not establish the cause.
Six-sample trials are diagnostic evidence, not confidence-bound release qualification.

All 33 first-output Wasm PNGs match exactly between profiles. The repeated Oklch PNG also matches.
Within-backend repeated-output checks pass. These are profile-to-profile checks, not a claim of universal JS output equivalence.
The speed build passes all 21 private Wasm checks.

Mia keeps anti-aliased bilinear unchanged. Historical JS bilinear uses four samples.
Downscale timings remain visible as filter-quality mismatches, excluded from like-for-like JS target counts.

## Provenance and retained evidence

The size scalar was compiled at runtime revision `753bfebe338a48e9204ff8e47194afe89a6eb775`.
The speed scalar was compiled at `c92e486945fce05bc6777100a593a5582354ad76`.
The intervening changes are tests and documentation; runtime sources match.
The original size trial's `candidateRevision` says `c92e4869`, the source-equivalent delivery revision, not its compilation revision.
Use the compiled revision above and the artifact SHA when reproducing it.

- Size scalar SHA256 `be2e868f9404a2bd1ad47420052b914526c296488d3539499dcae6029024078f`.
- Speed scalar SHA256 `6ac7685f16204d86a49d71b0bf3286a63927b6785a4cd4b1ef10cb1ed256bafe`.

Evidence lives under `.worktrees/celeste-browser-e2e/benchmark-results/js-performance/`:

- `profile-size-qualify/` and `profile-speed-qualify/` retain raw samples, manifests, and PNGs.
- `profile-size-oklch-repeat/` and `profile-speed-oklch-repeat/` retain the targeted repeat.
- `profile-build-comparison.json` and `profile-package-comparison.json` retain sizes and hashes.
- `profile-size-package/`, `profile-speed-package/`, and `profile-pack-{size,speed}/` retain compiled assets and archives.
- `profile-speed-build.log`, `profile-speed-private.log`, and `profile-images.json` retain validation evidence.

Selecting a new default build profile remains a separate decision. No kernels were changed for this experiment.
