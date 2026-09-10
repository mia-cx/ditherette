# Public Wasm source reuse

2026-09-10. Implements issue #139, stacked on PR #138 at `cef9a733`.

## Result

Exact-byte verified source reuse removes repeated source SHA-256 work.
Aligned views compare four bytes per iteration; unaligned views and trailing bytes compare individually.
The website adapter borrows contiguous source rows instead of copying them before Rust takes its snapshot.
Noncontiguous crops still pack their rows.

Mia explicitly approved this production cache-policy exception. The frozen reference remains unchanged.
Source dimensions, byte mutation detection, budget accounting, failure recovery, and durable outputs remain enforced.
The existing input scratch buffer owns the snapshot. No second full-source allocation exists.

For nearest resize plus sRGB quantization at 650 × 1042, repeated end-to-end medians fall:

| Engine | Previous | Selected | Speedup | Historical JS |
| --- | ---: | ---: | ---: | ---: |
| Chromium | 234.6 ms | 11.3 ms | 20.8× | 45.6 ms |
| Firefox | 1132 ms | 15 ms | 75.5× | 38 ms |
| WebKit | 229 ms | 8 ms | 28.6× | 36 ms |

Public-call medians for that repeat fall from 211.9/1109/210 ms to 6.6/10/5 ms respectively.
Candidate adapter medians are 0 ms at the browsers' timer precision.
Zero reported by a timer does not mean literally zero work.

## Scope and measurement

The input is the supplied Celeste box art, decoded to 2600 × 4168 RGBA8, or 43,347,200 bytes.
Decoded SHA-256 is `8c68cb3dbcb878d92389360e26c133ffef118df96a523cb97ab4d84b86c56d39` in all engines.
The baseline is the retained ordinary scalar package from `cef9a733`, not the rejected opt-level 3 experiment.
Historical JS comes from `a895267baea624a6e89bfcef6c5147f170e8a8f7` without rewritten algorithms.
Both selected Wasm builds retain the standard release profile, opt-level `s`.

Each run uses five recipes, three engines, and three backends: original Wasm, candidate Wasm, and historical JS.
Each backend receives three fresh-processor calls, three identical repeats, and three same-source settings changes.
Settings changes use output widths 651, 652, and 653 with height 1042.
These are cache misses with reusable source bytes, not repeated final-output hits.
The order alternates between backends. Each displayed value is a three-sample median.

End-to-end timing includes the adapter, synchronous public call, worker result transfer, indexed expansion, and canvas write.
Source decode, worker initialization, and processor construction occur outside this timer and are recorded separately.
PNG encoding, output comparison, and artifact writes also occur outside the timer.
The historical JS comparison does not include its old website's optional cross-call caches.

Runs `source-reuse-01` and `source-reuse-02` each complete 405 calls with no errors.
All baseline/candidate output comparisons and repeated-output checks are byte-exact.
The first run checks bytewise reuse; the second selects wider comparison plus the borrowed adapter view.
Both runs use fresh baseline measurements, with the benchmark lease held and other implementation/build/test work paused.
These are bounded diagnostic measurements, not replacements for the existing release gates or confidence analysis.

## Final matrix

All times below are end-to-end milliseconds from `source-reuse-02`.
The repeated columns compare identical requests. Changed columns compare the three new output widths.

| Engine | Recipe | Repeat before | Repeat after | Changed before | Changed after | Changed JS |
| --- | --- | ---: | ---: | ---: | ---: | ---: |
| Chromium | nearest + sRGB | 234.6 | 11.3 | 421.2 | 197.4 | 35.8 |
| Chromium | scale-aware Lanczos3 + sRGB | 231.7 | 10.6 | 670.6 | 452.6 | 294.7 |
| Chromium | nearest + Oklab | 236.2 | 10.5 | 448.7 | 227.8 | 35.1 |
| Chromium | nearest + Floyd–Steinberg | 234.7 | 10.6 | 481.1 | 253.5 | 112.1 |
| Chromium | nearest resize only | 229.6 | 7.7 | 248.2 | 22.5 | 4.5 |
| Firefox | nearest + sRGB | 1132 | 15 | 2437 | 1344 | 40 |
| Firefox | scale-aware Lanczos3 + sRGB | 1111 | 16 | 4595 | 3494 | 283 |
| Firefox | nearest + Oklab | 1103 | 16 | 2582 | 1481 | 47 |
| Firefox | nearest + Floyd–Steinberg | 1108 | 16 | 2888 | 1774 | 178 |
| Firefox | nearest resize only | 1265 | 12 | 1174 | 84 | 6 |
| WebKit | nearest + sRGB | 229 | 8 | 484 | 254 | 33 |
| WebKit | scale-aware Lanczos3 + sRGB | 235 | 11 | 718 | 497 | 219 |
| WebKit | nearest + Oklab | 233 | 10 | 494 | 269 | 55 |
| WebKit | nearest + Floyd–Steinberg | 231 | 10 | 525 | 306 | 135 |
| WebKit | nearest resize only | 234 | 7 | 250 | 23 | 4 |

## Remaining work

First use still copies and hashes the source. A changed source still needs a new hash.
For nearest plus sRGB, fresh candidate calls take 421/2427/478 ms in Chromium/Firefox/WebKit.
Historical JS takes 33.8/40/30 ms in the same run.
Changed settings also remain slower than JS throughout this matrix.
Even repeated nearest-only resize remains slower than recomputing that cheap filter in JS.

This change fixes repeated-source overhead. It does not establish browser kernel parity or eliminate all hashing and processing costs.
Keep release activation held. Profile cold and cache-miss calls separately before changing another policy or algorithm.

## Validation and evidence

- Full native library and integration suites pass.
- Standard scalar and pinned-toolchain threaded package builds pass.
- Independent frozen-spec guard passes against the trusted PR #138 worktree.
- All 44 interface tests, 35 adapter tests, and 16 worker-pipeline tests pass.
- Source reuse and stage ownership pass in Chromium 147.0.7727.15, Firefox 148.0.2, and Linux WebKit 26.4.
- Threaded ownership passes in Chromium and Firefox. WebKit's existing threaded limitation remains outside this test.
- Aligned/unaligned views, every byte position, trailing bytes, shadowed properties, detached buffers, dimensions, failures, pressure, and disposal have focused coverage.

[Raw results and runner scripts](wasm-source-reuse-evidence.tar.gz) retain both runs and the initial diagnosis.
The final manifest includes hashes for every baseline/candidate package JS and Wasm asset.
Full runtime assets and output PNGs remain under the repository's ignored `benchmark-results/wasm-source-reuse-2026-09-10/` directory.
No resize kernel, shared image contract, frozen specification, release profile, or deployment default changes.
