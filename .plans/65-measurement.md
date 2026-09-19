# S24 measurement and delivery evidence

## Decision

Retain the prepared, budget-accounted quantize integration. No new kernel optimization is claimed.
All 13 native cases pass with exact reference/accepted/candidate outputs.
Native full-call latency medians remain within 3% of the fresh literal baseline; the largest increase is 2.7433%.
The public runs are same-artifact package controls, not an accepted-versus-optimized public comparison.
Chromium's linear-RGB control is inconclusive. Every other public case passes.
No retry occurred. The predeclared 124-worker budget is complete, not silently extended.

## Audit and execution

The coordinator ran the prepared attempt 02 artifacts after draining agents, builds, tests, and owned browser processes.
The coordinator records completion at `2026-09-07T22:12:12.548Z` in `run-completion.json`, then ended the quiet phase.
I independently read all reports, events, requests, and result records after that phase.

| Runtime | Worker starts/reaps | Maximum live | Samples | Zero samples | Exit | Gate |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| Native | 52/52 | 1 | 1,040 | 0 | 0 | pass |
| Chromium | 24/24 | 1 | 480 | 0 | 2 | inconclusive |
| Firefox | 24/24 | 1 | 463 | 0 | 0 | pass |
| WebKit | 24/24 | 1 | 480 | 0 | 0 | pass |

All 124 records have exact raw reference/output values, matching fixture/settings identities, clean role revisions, and maximum-live count one.
Every recorded three-way comparison is exact, including packed coordinates, byte alpha, indices, palette, transparency, and warnings.
The event audit matches each started PID with one reap and leaves no live worker.
Firefox's Oklab/CIELAB trials retain 17 or 18 samples when their measured work reaches the declared 250 ms cap.
Those eight trials each retain 251.58–257.14 ms of measured work. Nothing was censored or padded to reach 20 samples.
Browser result records all report primed instances, no application cache, and cross-origin isolation.
The coordinator's completion record also reports no remaining owned processes.

## All cases

Values are pooled medians in milliseconds per call. Throughput rows normalize their recorded calibrated batches.
A ratio below one means the candidate role was faster in this run; it does not by itself prove an optimization.

| Runtime | Case | Accepted ms/call | Candidate ms/call | Ratio | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| native | srgb-forward | 0.053500 | 0.052586 | 0.982916 | pass |
| native | srgb-palette64-latency | 0.827267 | 0.826487 | 0.999057 | pass |
| native | srgb-palette16-latency | 0.243979 | 0.237624 | 0.973951 | pass |
| native | srgb-palette256-latency | 2.668661 | 2.694792 | 1.009791 | pass |
| native | srgb-palette64-throughput | 0.864131 | 0.825503 | 0.955298 | pass |
| native | linear-rgb-forward | 0.052741 | 0.052570 | 0.996767 | pass |
| native | linear-rgb-palette64-latency | 0.795778 | 0.811237 | 1.019428 | pass |
| native | oklab-forward | 0.162997 | 0.162137 | 0.994724 | pass |
| native | oklab-palette64-latency | 1.037290 | 1.035610 | 0.998380 | pass |
| native | cielab-forward | 0.180943 | 0.181848 | 1.005002 | pass |
| native | cielab-palette64-latency | 1.048811 | 1.044406 | 0.995800 | pass |
| native | ycbcr-forward | 0.055066 | 0.054781 | 0.994843 | pass |
| native | ycbcr-palette64-latency | 0.834738 | 0.857638 | 1.027433 | pass |
| chromium | srgb-palette64-latency | 0.815000 | 0.830000 | 1.018405 | pass |
| chromium | srgb-palette64-throughput | 0.827917 | 0.807500 | 0.975340 | pass |
| chromium | linear-rgb-palette64-latency | 0.802500 | 0.802500 | 1.000000 | inconclusive |
| chromium | oklab-palette64-latency | 1.050000 | 1.027500 | 0.978572 | pass |
| chromium | cielab-palette64-latency | 1.060000 | 1.095000 | 1.033019 | pass |
| chromium | ycbcr-palette64-latency | 0.875000 | 0.872500 | 0.997143 | pass |
| firefox | srgb-palette64-latency | 11.760000 | 11.860000 | 1.008503 | pass |
| firefox | srgb-palette64-throughput | 11.780000 | 11.740000 | 0.996604 | pass |
| firefox | linear-rgb-palette64-latency | 11.730000 | 11.750000 | 1.001705 | pass |
| firefox | oklab-palette64-latency | 14.130000 | 14.160000 | 1.002123 | pass |
| firefox | cielab-palette64-latency | 14.360000 | 14.160000 | 0.986072 | pass |
| firefox | ycbcr-palette64-latency | 11.930000 | 11.870000 | 0.994971 | pass |
| webkit | srgb-palette64-latency | 1.060000 | 1.060000 | 1.000000 | pass |
| webkit | srgb-palette64-throughput | 1.070000 | 1.080000 | 1.009346 | pass |
| webkit | linear-rgb-palette64-latency | 1.020000 | 1.040000 | 1.019608 | pass |
| webkit | oklab-palette64-latency | 1.300000 | 1.300000 | 1.000000 | pass |
| webkit | cielab-palette64-latency | 1.340000 | 1.320000 | 0.985075 | pass |
| webkit | ycbcr-palette64-latency | 1.140000 | 1.100000 | 0.964912 | pass |

Chromium linear RGB has pair ratios `0.9418961439762625` and `1.037974800060359`.
Both pooled role medians round to 0.802500 ms, but the pair spread fails the noise gate.
That required control remains incomplete performance evidence, not a confirmed regression or a pass.
S41 tracks it with Firefox's absolute complete-call cost.
Firefox candidate latency medians are 11.75–14.16 ms here, versus Chromium's 0.8025–1.095 ms.
This audit diagnoses no cause. It does not attribute the difference to copying, validation, Wasm execution, or any particular browser behavior.

## Source, runtime, and artifact identity

The literal implementation is `a23260edec0452fd17c13073636f548b07804230`.
Accepted native artifact source is `ccb9bceb28563c562dd5e6c05f68c056c18e3519`, with only the thin benchmark overlay.
Its production tree remains `d4847ee149dfa346a85a23f8be28e6c9eb1fb768`, identical to the literal checkpoint.
Measured candidate/coordinator/public source is `f4b90ecfcde63531fb992cf87ebda04d4e373032`.
Public accepted and candidate both use that source and the same complete package/transport/runtime identities.

Native builds record Rust 1.97.0, commit `2d8144b7880597b6e6d3dfd63a9a9efae3f533d3`, LLVM 22.1.6, and tool version 0.1.0.
The machine is Linux x86_64, kernel 6.12.95+deb13-amd64, AMD Ryzen 9 7950X, with 24 logical CPUs exposed.
Browser records identify Chromium 147.0.7727.15, Firefox 148.0.2, WebKit 26.4, Node v24.19.0, and Playwright 1.59.1.
Complete browser installations, launch semantics, served package/TypeScript/transport files, and build inputs remain bound in the prepared manifests.
Observed timer quanta are approximately 5 microseconds for Chromium and 20 microseconds for Firefox/WebKit.

All raw paths are under `.worktrees/v1-s24-bench/crates/ditherette-bench/target/s24-quantize-attempt-02/`.
[Prepared attempt 02](65-prepared-attempt-02.md) retains full prepared-manifest, provenance, fixture, and artifact hashes.
Key executable/package hashes are repeated here:

| Artifact | SHA-256 |
| --- | --- |
| Accepted native worker | `3a888084f2c9f9bea610eef4909100492816111034b99b826fd6bc4e6a316916` |
| Candidate/public worker | `2e6ce05e739cee84bc067b241800ade7197c833fff15b63bef5582dea2f54d3c` |
| Coordinator | `b93d13d1552e7340b58a200667daff6af670f73a1bb9bdf789664c032c258687` |
| Public tarball | `39f607810adce816973ad4adb0301607281698622341cd72e6edc02fcb0d3eee` |
| Build provenance | `b4777d04ea346c53cc015a37cdd64edc1e10ac07ce427ffefc01f62b6d4c77c0` |

| Evidence file | SHA-256 |
| --- | --- |
| `run-completion.json` | `5976aab1e1797cc9497576ed8dd21991d7c2f918ca76cc173d4a14035d92db02` |
| `results-native/report.json` | `6ecebee250792a23961261569a596eee8837146cad93d81fef1cd09c264b22cf` |
| `results-native/events.jsonl` | `7da6a332c53f6a0160d7dfaadbee1bbf6b72a5e2e3b22784cd81eb0717de77df` |
| `results-chromium/report.json` | `ae8f6aa5f55a3c791b359e92f578612fea7b470d6ac887bc32c0714e14dc8de7` |
| `results-chromium/events.jsonl` | `d748db123c5adec994a3f323c565fa63997cc7cf59dd8eb7186422f50f3a50c8` |
| `results-firefox/report.json` | `9fdb12f53d10a9464d1db66627d0c874eed33e0b93a04b46a573615dbf298002` |
| `results-firefox/events.jsonl` | `c5a66ae3c9d57cc28f80fee7c205642a66c0e7679c52b705e46a5df57b9cd10f` |
| `results-webkit/report.json` | `f3d3623089d056e1bf5400d21eb7fa0cc5996a02da02cc08b946d43034035134` |
| `results-webkit/events.jsonl` | `75d03096a3ec3c9c1bde9e1300d29628797edf4752c9125a2a761ce5baed4e20` |

## Memory and limits

Native complete-call timing includes validation, preparation, owned indices/metadata construction, and destruction; source bytes remain borrowed.
Forward controls exclude converter setup and output allocation, but observe every conversion through the same optimizer barrier.
Public timing includes the actual package call, input/result copies, preparation, and indexed result ownership.
No synthetic hash, application cache, or faithful TypeScript quantize adapter is claimed.
Fresh/primed initialization and application-cache scopes are not interchangeable.

The public observer checks every returned warmup/sample output outside its timer.
Calibrated throughput retains a whole batch under the 64 MiB declared output-storage/record cap.
Retention changes GC behavior and result lifetimes. It is not a measurement of pure package overhead or total JavaScript heap memory.
Native checks cover before/after outputs for fixed deterministic callables, not every timed intermediate result.
Neither scope proves absence of arbitrary stateful transient behavior outside its stated ownership assumptions.

Memory correctness comes from the independently validated allocation tests, not these latency reports.
Prepared capacity includes records, tables, palette/matcher Vec capacities, and warning strings; execution allocates no working float-alpha plane.
Public accounting also covers the owned source/index buffers and boundary records.
Exact/one-under budgets, seven injected preparation reservations, source/index failure, caught publication failure, and recovery are covered.
Private Wasm fixtures run 512 success/failure cycles without post-warmup externref or page growth.
No benchmark peak RSS, JavaScript heap total, or zero-overhead claim is made.

## Restack and validation

S22 parent `9eecc670d9ff587ff10f8d2f3a8b86bab600c988` joins at `48598b1824b133b3162a2343425a65d366ea5aae`.
The required `git rebase --rebase-merges origin/impl/v1-s22-convolution` succeeds.
The complete tracked tree compares unchanged against pre-restack `1d304f066b8b3598ca1ad7fe2839a59405a15dad`.
Original literal and measured-source commits remain ancestors. No processing, benchmark, spec, image, or freeze bytes change.
Only this delivery documentation follows the measured source.

The measured-source validation remains applicable: 56 benchmark Rust tests, 26 controlled JavaScript tests, Wasm compilation, formatting, and all three installed-package engines pass.
Public implementation evidence also records 307 native tests, nine private ABI tests, and 20 interface tests plus TypeScript fixtures.
The separately trusted freeze guard passes again after restacking.
Frozen revision remains `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`, digest `sha256:17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
No benchmark was rerun during delivery. S41 retains the unresolved control noise and absolute browser-cost investigation.

