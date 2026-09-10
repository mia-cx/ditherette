# Dependency caching without image hashing

Identity resize was spending more time hashing unchanged pixels than copying them.
Mia approved source revisions and staged dependency caching on 2026-09-10.

The runtime now verifies source bytes against its owned snapshot and reuses the associated revision.
Changed inputs receive a fresh revision. Resize, perturbation and indexed keys depend on their parent and settings.
No source or intermediate image is hashed, including on first-image calls.
The existing small settings and palette records still use canonical SHA-256 lookup keys.

Identity resize passes the source through internally; public results retain independent ownership.
Final cache hits skip intermediate lookups. Perturbation hits skip resize materialization.
Explicit staged calls can reuse retained RGBA outputs after exact-byte and dimension verification.
Failed calls publish neither a new snapshot nor new stages. Revisions survive failures and scratch eviction without recycling.
LRU limits, capacity accounting, frozen specifications and landed kernels remain intact.

## Fresh browser comparison

Input is the supplied Celeste art at 2600 x 4168. Scalar nearest-center resize, Wplace palette, sRGB Euclidean matching, preserved alpha, no dithering.

Three-sample medians in milliseconds. End-to-end timing includes the worker request, public processing, result transfer, palette expansion and canvas write.
Image decode, module loading and processor creation are outside the timer.
Cold means the first processing call on a fresh processor, not a fresh browser engine.
Warm recompute primes the same source at width +1, +2 or +3 before measuring the requested dimensions. Final results are not cached.
Historical JS retains its within-call RGB cache; optional website cross-call caches stay disabled.

| Browser | Scale | Wasm cold before | Wasm cold after | Wasm warm before | Wasm warm after | JS cold | JS warm |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Chromium | 10% | 180.5 | 34.0 | 11.7 | 11.5 | 13.0 | 12.2 |
| Chromium | 25% | 174.1 | 44.5 | 32.0 | 24.2 | 32.7 | 33.5 |
| Chromium | 50% | 237.2 | 66.7 | 96.5 | 61.1 | 92.8 | 90.0 |
| Chromium | 100% | 482.3 | 179.6 | 334.9 | 173.5 | 219.1 | 224.8 |
| Firefox | 10% | 1208 | 45 | 56 | 44 | 14 | 12 |
| Firefox | 25% | 1350 | 136 | 205 | 134 | 37 | 38 |
| Firefox | 50% | 1768 | 356 | 628 | 348 | 101 | 100 |
| Firefox | 100% | 3208 | 928 | 2107 | 918 | 268 | 281 |
| WebKit | 10% | 176 | 31 | 13 | 11 | 12 | 16 |
| WebKit | 25% | 210 | 40 | 33 | 25 | 33 | 28 |
| WebKit | 50% | 268 | 83 | 100 | 62 | 70 | 73 |
| WebKit | 100% | 513 | 193 | 321 | 154 | 177 | 179 |

JS columns are contemporaneous controls from the candidate run. Raw evidence also retains the fresh baseline's JS timings.
Browser versions are Chromium 147.0.7727.15, Firefox 148.0.2 and Linux WebKit 26.4.

At 100%, warm public-call time excluding canvas falls from 269.2 to 108.5 ms in Chromium, 1991 to 851 ms in Firefox, and 284 to 119 ms in WebKit.
Firefox's remaining quantization path is still slow. These cache changes do not claim to fix that separate execution cost.
Cold Wasm still loses to JS at small outputs. First-source copying and cold allocation remain measurable.

Identical-request cache hits are mostly unchanged. The raw ranges include small regressions and noise, including Chromium 25% end-to-end repeats at 10.7 to 12.1 ms.
That case's public-call median is 6.6 to 6.4 ms. No repeat-hit result is hidden or relabeled as a kernel improvement.
These bounded measurements are not confidence-gated release qualification. Existing release holds remain unchanged.

## Verification and provenance

- 442 native tests pass, plus 61 library tests with benchmark subjects enabled.
- 45 package interface tests pass, including the minimal identity budget and output ownership.
- Source verification, stage ownership and progress pass in all three browser engines. Threaded ownership passes Chromium and Firefox.
- Both ordinary package builds and the trusted frozen-reference guard pass.
- The red identity-storage test measured 115120 bytes versus 98496 for direct quantize before the fix. It passes after identity materialization is removed.
- Waterfall tests cover source, resize, perturbation and palette changes against fresh computations. Existing tests retain mutation, failure, eviction and cross-method coverage.

The exclusive lease covers 432 timed calls and 144 untimed primes. No build, test or implementation work runs concurrently with timings.
All calls complete without errors. Repeated outputs match their backend's cold output.
All 24 before/after PNG pairs are byte-identical, including 12 Wasm pairs and 12 JS controls. This proves rendered RGBA equality, not independently measured index equality.
JS and Wasm retain their pre-existing small quantization differences. No visual-difference acceptance is requested.

Baseline package source is `60b47c264af16842894352ee0fecd00e65594356` from PR142.
Candidate source is `1762aaa03d5c966f5bb3d7fba17e8bfe4290f98d`; runtime commit is `00c9de70`.
Baseline scalar Wasm SHA-256 is `44fb3bf8cabf6f1e7d43c074bd86de2f82990b22e8dc786f85f61c42357d1c89`.
Candidate scalar Wasm SHA-256 is `bb6e0293e115d4f78746d7f5d665ede691ef1d27cdf459583a8e0d199d4ee02d`.
The validated candidate tarball SHA-256 is `a6b10590029ebe4848cfe6c3495ff4517702cd69b351489d50d27a87243e6911`.

Adjacent `tiered-caching-evidence.tar.gz` contains both raw runs, manifests, the summary audit and the runner scripts.
The local full evidence directory also retains PNGs and runtime assets at `benchmark-results/tiered-caching-2026-09-10` in the main checkout.

Reproduce from the implementation worktree with `benchmark-results/celeste/runtime/ditherette-bench-lease --quiet -- node benchmark-results/celeste-scales/run.mjs NEW_TRIAL`.
Set `CELESTE_CASES=scale-260,scale-650,scale-1300,scale-2600` and point `CELESTE_CANDIDATE` at the intended package's dist directory.
Set `CELESTE_CANDIDATE_REVISION` to its actual source revision. Use a new trial name and run packages serially.
