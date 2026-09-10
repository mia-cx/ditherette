# Cold processing and palette matching

2026-09-10. Issue #141, stacked on PR #140.

## Selected changes

Production now memoizes exact RGB-to-palette results in optional, bounded scratch.
Scalar calls use at most 2 MiB. Threaded calls use separate bounded worker tables.
The cache keys effective RGB bytes after alpha preparation, not rounded diffusion coordinates.
Collisions recompute the exact result. First-entry ties and f32 expressions stay unchanged.
Each call clears its table. Cached final outputs allocate no RGB table.
Insufficient spare capacity or allocation failure leaves the direct scan available.
Peak accounting includes actual allocation capacities and owning records.

Palette matching selects the metric before scanning candidates.
Wasm bulk image identities use RustCrypto's scalar SHA-256 compression with its existing digest buffering and padding.
Native production keeps hardware-accelerated hashing. Digests and key framing remain identical.
Channel-distance lookup tables are not selected in this change.

The frozen reference, landed resize kernels, shared image code, public JavaScript API, and release profile remain unchanged.
No PR is merged, package published, or rollout enabled.

## Main result

Celeste nearest resize plus sRGB quantization, output 650 × 1042, scalar execution.
Values are end-to-end milliseconds, three-sample medians from `cold-combined-01`.

| Engine | Cold before | Cold after | Cold JS | Changed before | Changed after | Changed JS |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chromium | 414.7 | 182.1 | 33.6 | 198.0 | 32.5 | 31.8 |
| Firefox | 2434 | 1345 | 39 | 1357 | 207 | 40 |
| WebKit | 482 | 210 | 32 | 258 | 33 | 36 |

Identical repeated calls remain 10.6/14/8 ms respectively.
Changed settings use new output widths, so these calls recompute pixels rather than retrieving identical final output.
Chromium and WebKit approach historical JavaScript for several changed-settings recipes.
Firefox remains slower. First-image calls still lose to JavaScript for this Celeste recipe.
This is an improvement, not zero overhead or completed release qualification.

## Scope and provenance

Baseline production is PR #140, `7a57caa5b0be886146322bb6caa9165bad5e0c81`.
Selected source is `60b47c264af16842894352ee0fecd00e65594356`.
Historical JavaScript is `a895267baea624a6e89bfcef6c5147f170e8a8f7`, with its original within-call RGB cache enabled.
Its optional website cross-call caches remain disabled, as in the preceding measurements.

The supplied Celeste PNG decodes to 2600 × 4168 RGBA8, or 43,347,200 bytes.
Decoded SHA-256 is `8c68cb3dbcb878d92389360e26c133ffef118df96a523cb97ab4d84b86c56d39`.
The manifests retain the actual decoded hash for each engine and recipe.
The two entropy controls use deterministic xorshift RGB bytes with opaque alpha, generated outside the timer.
Both backends receive identical bytes. These controls limit benefits from repeated source colors.

The browser timer includes request adaptation, synchronous processing, worker result transfer, palette expansion, and canvas writes.
Decode, worker initialization, and processor creation are outside that timer and recorded separately.
PNG encoding and comparisons are also outside it. Backends alternate order.
Each recipe has three fresh-processor calls, three identical repeats, and three changed-width calls per backend.
Cold here means the first processing call on a fresh processor, not a browser navigation including module download.

The SHA-only experiment completes 162 calls. The combined nine-recipe experiment completes 729 calls.
All 891 calls finish without errors. Expanded RGBA comparisons against baseline and repeat checks are exact.
Native comparisons also verify exact indices and metadata against the frozen reference.
Browser measurements are bounded diagnostics, not replacements for the existing release confidence gates.

Engines are Chromium 147.0.7727.15, Firefox 148.0.2, and Linux WebKit 26.4.
The package retains opt-level `s`, stable scalar Rust, and pinned threaded nightly.
Only one benchmark owns the lease at a time. Agents, builds, and tests stop during measurements.

## SHA-only isolation

This package changes SHA-256 only, before metric specialization and RGB memoization.
The fresh resize-only comparison isolates most bulk source hashing cost without palette matching.

| Engine | Cold resize before | SHA candidate | Change |
| --- | ---: | ---: | ---: |
| Chromium | 245.0 ms | 166.4 ms | 32.1% faster |
| Firefox | 1180 ms | 1227 ms | 4.0% slower |
| WebKit | 249 ms | 182 ms | 26.9% faster |

Firefox does not benefit from this SHA implementation. Its observed slowdown stays below the existing 10% regression limit.
The candidate is selected for the Chromium and WebKit improvements, not claimed as a Firefox hashing optimization.

## Native scalar quantization

Fresh `ditherette-bench` pairs include validation, preparation, scratch allocation, owned output allocation, and destruction.
They borrow the source. These are complete native quantize calls, not timed browser transfers.
The regular fixture is 128 × 96 with 32 palette entries and covers all fifteen metrics.
The same existing plan supplies diffusion, metric-score, and tiny-call controls.

Each comparison uses two alternating AB/BA pairs, 50 ms warmup, and 5–20 samples with a 250 ms target.
The audit verifies actual sample counts, pooled medians, pair ratios, source revisions, and serial process journals.
There are 192 workers and 3,744 samples across selection, spec comparison, and one bounded selection repeat.
Every worker is reaped. Every output comparison is exact.

| Matching | Speedup over previous prod | Speedup over spec |
| --- | ---: | ---: |
| sRGB Euclidean | 2.20× | 2.12× |
| Linear RGB Euclidean | 2.22× | 3.10× |
| Oklab Euclidean | 1.75× | 2.49× |
| CIELAB Euclidean | 1.73× | 2.41× |
| YCbCr Euclidean | 2.11× | 2.12× |
| CompuPhase | 4.18× | 4.24× |
| Rec. 601 | 1.87× | 1.80× |
| Rec. 709 | 1.86× | 1.77× |
| OKLCH Euclidean | 1.57× | 2.18× |
| OKLCH circular hue | 1.36× | 1.38× |
| OKLCH hue arc | 1.89× | 1.51× |
| CIEDE2000 | 0.98× | 1.00× |
| CIELCH Euclidean | 1.50× | 2.00× |
| CIELCH circular hue | 1.39× | 1.33× |
| CIELCH hue arc | 1.88× | 1.43× |

Selection initially has 30 passes and one inconclusive CIELCH Euclidean comparison.
Its single fresh repeat passes, with candidate/baseline pair ratios 0.6703 and 0.6634; the table uses that repeat.
Diffusion and score controls pass without a confirmed regression. Tiny quantize is about 4% faster than previous production.
CIEDE2000 is about 2% slower than previous production, within the existing threshold.

The separate spec comparison retains an inconclusive OKLCH circular-hue timing and an inherited tiny-call regression.
Tiny production quantize is about 2.9× slower than naive spec because setup dominates that fixture.
Those results are not relabeled as passes. They do not indicate a regression introduced by this change.

## Changed-settings browser matrix

Milliseconds from `cold-combined-01`. All recipes output approximately 650 × 1042.

| Engine | Recipe | Previous prod | Selected prod | Historical JS |
| --- | --- | ---: | ---: | ---: |
| Chromium | nearest + sRGB | 198.0 | 32.5 | 31.8 |
| Chromium | scale-aware Lanczos3 | 444.0 | 286.7 | 293.5 |
| Chromium | Oklab | 223.6 | 35.6 | 33.3 |
| Chromium | Rec. 709 | 197.1 | 35.9 | 34.5 |
| Chromium | Bayer 8 | 235.7 | 78.6 | 75.0 |
| Chromium | Floyd–Steinberg | 253.7 | 252.9 | 112.8 |
| Chromium | resize-only nearest | 22.5 | 18.1 | 5.2 |
| Chromium | entropy + sRGB | 216.3 | 81.0 | 154.5 |
| Chromium | entropy + Oklab | 246.8 | 94.5 | 163.3 |
| Firefox | nearest + sRGB | 1357 | 207 | 40 |
| Firefox | scale-aware Lanczos3 | 3523 | 2452 | 290 |
| Firefox | Oklab | 1473 | 230 | 47 |
| Firefox | Rec. 709 | 1474 | 218 | 39 |
| Firefox | Bayer 8 | 1550 | 599 | 104 |
| Firefox | Floyd–Steinberg | 1779 | 1732 | 173 |
| Firefox | resize-only nearest | 86 | 86 | 5 |
| Firefox | entropy + sRGB | 1371 | 756 | 170 |
| Firefox | entropy + Oklab | 1500 | 891 | 257 |
| WebKit | nearest + sRGB | 258 | 33 | 36 |
| WebKit | scale-aware Lanczos3 | 495 | 286 | 214 |
| WebKit | Oklab | 269 | 36 | 52 |
| WebKit | Rec. 709 | 256 | 37 | 33 |
| WebKit | Bayer 8 | 296 | 83 | 93 |
| WebKit | Floyd–Steinberg | 307 | 303 | 134 |
| WebKit | resize-only nearest | 24 | 20 | 6 |
| WebKit | entropy + sRGB | 271 | 91 | 135 |
| WebKit | entropy + Oklab | 285 | 106 | 340 |

## Validation and retained evidence

Both ordinary package builds pass. Final source passes 440 native tests, 44 interface tests, and the trusted frozen-reference guard.
Scalar source mutation and stage ownership pass all three engines. Threaded ownership passes Chromium and Firefox.
No new threaded WebKit support is claimed.

The adjacent `cold-processing-evidence.tar.gz` contains native sample records, process journals, browser results, manifests, and audit code.
It omits large pixel arrays while retaining each complete raw result's SHA-256 digest.
Full raw results, browser assets, and PNGs remain under `benchmark-results/wasm-cold-processing-2026-09-10` locally.
The full raw archive SHA-256 is `0f18f1ee259a3f2763d8eccf6d02f032470c7d40ec25e72260ec21dd2f28a126`.
The SHA-only scalar Wasm digest is `c31f6a0e5f7885fa6ffa03080418df0c31467f852c4e6e6cae9134af7ca01e25`.
The selected scalar Wasm digest is `44fb3bf8cabf6f1e7d43c074bd86de2f82990b22e8dc786f85f61c42357d1c89`.
Compiler targets are removed after preserving evidence. Rebuild them during review.

Remaining performance work is first-image hashing, Firefox execution cost, and diffusion's full-call cost.
Existing release holds remain in place.
