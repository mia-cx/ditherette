# Scalar quantizer alpha overhead

The selected change moves preserved-alpha dispatch out of the pixel loop.
Validated fractional thresholds become an exact byte cutoff once per row.
Transparent-only palettes fill the row directly. Matte and premultiplied paths keep their existing arithmetic.
Production changes total 23 added lines. Frozen specifications, resize kernels, shared image helpers, cache layout and build profiles stay unchanged.

## Fresh browser measurements

The supplied Celeste image decodes to 2600 x 4168 RGBA8 pixels.
Default settings use nearest-center resize, the Wplace palette, sRGB Euclidean matching, preserved alpha at 128, and no dithering.
Every trial runs serially through the exclusive ditherette-bench lease.

Cold means the first processing call on a fresh processor, excluding module loading and processor creation.
Warm means recomputation after priming at width +1, +2 or +3 outside the timer. It is not a final-result cache hit.
Public-call timing includes the ordinary package boundary. End-to-end timing also includes worker transfer, palette expansion and canvas writing.
PNG decoding stays outside both timers. Each value below is a three-sample median in milliseconds.

| Browser, full size | Cold before | Cold after | Warm before | Warm after | JS cold | JS warm |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Chromium | 123.2 | 89.4 | 107.7 | 82.7 | 146.7 | 151.0 |
| Firefox | 127 | 100 | 120 | 86 | 197 | 196 |
| WebKit | 144 | 105 | 119 | 83 | 132 | 136 |

Warm public processing improves 23%, 28% and 30% respectively in the final comparison.
The initial independent comparison measured 112.8 to 77.8, 113 to 90, and 126 to 85 ms.
Full-size warm end-to-end medians in the final run are 144.6/154/120 ms, versus JS 213.2/263/174 ms.
JS columns are contemporaneous controls from the selected candidate run, not historical best results.

The wider controls use 650 x 1042 output. These are warm public-call medians, before to after.

| Control | Chromium | Firefox | WebKit |
| --- | ---: | ---: | ---: |
| Celeste, sRGB | 19.3 to 17.2 | 23 to 23 | 22 to 19 |
| Deterministic high-entropy input | 69.6 to 68.7 | 71 to 72 | 79 to 76 |
| Oklab | 22.3 to 19.0 | 26 to 23 | 24 to 22 |
| Matte, all alpha bytes | 38.3 to 35.7 | 39 to 41 | 38 to 40 |
| Premultiplied, all alpha bytes | 35.2 to 34.3 | 37 to 37 | 37 to 42 |
| Bayer 4 | 51.2 to 48.8 | 53 to 51 | 54 to 52 |
| Floyd-Steinberg | 235.9 to 237.1 | 215 to 213 | 293 to 292 |

These bounded browser trials are diagnostic evidence, not release confidence gates.
The unchanged WebKit premultiplied control is 13.5% slower in this small sample; no improvement is claimed there.
Cold Chromium 25% results vary widely, including 151.4 ms before and 64.3 ms after in the final run.
Earlier runs are much lower. Those samples remain in the evidence; do not treat their difference as a kernel speedup.
Floyd-Steinberg still loses to JS on this fixture. Its loop is unchanged.

## Corrected Firefox preparation

Our Playwright 1.59.1 Firefox 148.0.2 creates a Debugger without `allowUnobservedWasm`.
That forces debuggable baseline Wasm and invalidates conclusions about ordinary Firefox performance.
All measurements in this report use a copied Firefox tree with that flag enabled.
The installed browser remains unchanged. Chromium is 147.0.7727.15; Linux WebKit is 26.4.
Every recorded Firefox run reports 148.0.2.

Before preparing future Firefox benchmark snapshots, run:

```sh
node scripts/prepare-firefox-benchmark.mjs FIREFOX_EXECUTABLE NEW_DIRECTORY
```

Use the returned executable when preparing the benchmark. The helper patches and verifies the copied Juggler archive.
It also pins application-update policy relative to the executable directory, so snapshots can relocate the browser.
It refuses unknown debugger constructor layouts and writes archive hashes in `benchmark-runtime.json`.
This helper does not silently change existing prepared artifacts or historical reports. Existing release gates need fresh measurements.

## Native selection and frozen spec

The paired native comparison covers all 15 matching modes and a one-pixel control.
It times complete borrowed-input quantize calls, including preparation, allocation and destruction.
Fixtures have 128 x 96 pixels and a 32-entry palette. The one-pixel control retains the same palette.

Fifteen selection cases pass. sRGB is inconclusive because alternating pair ratios disagree.
One bounded repeat remains inconclusive, with ratios 0.901 and 1.002. No confirmed native regression appears.
This is not an all-green native selection gate. The candidate is retained for PR review based on repeated browser gains and exact outputs.
Native promotion remains held; the inconclusive case is not relabeled as passing.

The separate spec-versus-production run times the actual frozen implementation, not an old production revision.

| Matching mode | Spec time / production time |
| --- | ---: |
| sRGB Euclidean | 2.18x |
| Linear RGB Euclidean | 3.21x |
| Oklab Euclidean | 2.55x |
| CIELAB Euclidean | 2.40x |
| YCbCr Euclidean | 2.19x |
| sRGB Compuphase | 4.36x |
| sRGB Rec601 | 1.83x |
| sRGB Rec709 | 1.77x |
| OKLCH Euclidean | 2.22x |
| OKLCH circular hue | 1.30x |
| OKLCH hue arc | 1.53x |
| CIELAB CIEDE2000 | 1.00x |
| CIELCH Euclidean | 2.04x |
| CIELCH circular hue | 1.31x |
| CIELCH hue arc | 1.49x |
| One-pixel sRGB | 0.36x |

All full-image spec comparisons pass the existing timing gate. The tiny-call setup regression remains visible.
These native ratios describe the entire production implementation, not the incremental alpha change.

## Rejected candidates

- Forced row-loop inlining (`4e83e12a`) adds no consistent browser gain and worsens some controls.
- Exact previous-pixel cache reuse (`3721a1ad`) slows full-size warm calls in all three browsers.

Both changes were removed. Their commits, binaries' identities and raw browser samples remain available.
There is no new cache or matcher optimization in the delivered runtime.

## Validation and retained evidence

- 444 native tests and 45 package interface tests pass.
- New frozen-output tests exhaust all alpha bytes, fractional cutoffs, fallback ordering, padded rows, cached/direct/banded calls and compositing rounding.
- Both scalar and threaded package builds pass. The final scalar bytes match the measured selected artifact exactly.
- The trusted frozen-reference guard passes, including isolated native/Wasm builds.
- Installed scalar source-ownership, stage-cache and progress checks pass in all three engines. Threaded ownership passes Chromium and Firefox.
- Three Firefox preparation tests pass. The prepared update-pinned Firefox also passes the installed package checks.
- The broader tarball oracle test stopped at setup because its separately prepared Yliluoma oracle was absent. It was not claimed as passed.

Native trials total 132 serial workers and 2,552 samples. Every native comparison is byte-exact against the frozen reference.
Eight browser trials total 1,620 timed calls and 540 untimed primes. All 1,440 repeated rendered outputs are exact.
The audit verifies 54 byte-identical before/after Wasm PNG pairs, including all 24 final control pairs.
This does not claim JS/Wasm equality; their previously documented palette-matching differences remain.

Baseline package source is `646de481a33ca72af08de5495a7ca59d5ad4c12a`.
The selected runtime is `a1e7389de43c9a6c066b8a8692bb3d94a1b73092`; later runtime candidate reversions reproduce it exactly.
Baseline scalar SHA-256 is `bb6e0293e115d4f78746d7f5d665ede691ef1d27cdf459583a8e0d199d4ee02d`.
Selected scalar SHA-256 is `001c7f8222bcb47b8c1e3dfbbd78ebbe789b067dfa17c6d65aa7d73951fa3cfc`.
Final installed tarball SHA-256 is `bc97f1301a9e800bf1b767cd201a816a1995c7274eed5061e65b57a447e1527f`.

The adjacent `quantize-hotpaths-evidence.tar.xz` retains native raw results, browser samples, scripts, build identities and test logs.
Its SHA-256 is `6374854176b234f866d6f5ba1d8ed8bf69f5f765546fe540e29a1c1af8ad3c3c`.
Original PNGs and package snapshots remain under the implementation worktree's `benchmark-results/quantize-hotpaths-browser` and `benchmark-results/quantize-hotpaths`.
Finished compiler targets and disposable Firefox copies are removed after handoff. Nothing is merged, published or activated.
