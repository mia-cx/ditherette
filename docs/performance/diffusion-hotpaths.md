# Scalar diffusion hot paths

The scalar browser screen found repeated preparation inside adaptive diffusion and repeated metric dispatch inside palette scans.
This change reuses the prepared converter, specializes finite scans, classifies sinks from alpha, and memoizes rounded byte-feedback RGB values.
It preserves the frozen reference, existing filter equations, scan order, tie order and non-finite failures.
Matching feedback retains its floating-point coordinates and does not use the RGB cache.

## Browser screening

Celeste decodes to 2600 × 4168 RGBA8 pixels. These are median warm public processing times in Chromium 147.
Each cell has three samples. This is candidate screening, not qualification of the wider JS performance target.
Imports, processor creation and priming are outside the public-call timer. Worker transfer and canvas submission have separate measurements.

| Setting | Before diffusion work | Converter and metric changes | Current candidate | Fresh JS |
|---|---:|---:|---:|---:|
| Floyd-Steinberg, 10% | 42.8 ms | 21.7 ms | 19.1 ms | 30.1 ms |
| Floyd-Steinberg, 50% | 940.6 ms | 398.2 ms | 258.0 ms | 321.7 ms |
| Sierra, 50% | 1081.5 ms | 535.5 ms | 300.2 ms | 334.2 ms |
| Sierra-lite, 50% | 905.1 ms | 363.3 ms | 247.0 ms | 353.0 ms |
| Adaptive sRGB, 10% | 416.6 ms | 22.9 ms | 21.2 ms | 17.6 ms |
| Adaptive linear RGB, 10% | 861.4 ms | 63.1 ms | 37.0 ms | 42.4 ms |
| Adaptive Oklch, 10% | 916.7 ms | 136.7 ms | 103.3 ms | 101.3 ms |

The current candidate keeps all 16 compared Wasm PNGs byte-identical to its predecessor.
The predecessor keeps those same images identical to the accepted package.
JS uses its historical algorithms. Its rendered differences remain recorded, not treated as Wasm regressions or silently normalized away.
Sierra and several adaptive settings still miss the target of 20% lower time than JS.
Firefox, WebKit and six-sample cold-worker qualification remain separate work.

## Native selection

The first converter/metric selection has seven passes and one inconclusive timing result, with exact frozen outputs in all eight cases.
A fresh comparison of sink classification and RGB-cache support against that predecessor passes all eight cases.
Candidate/accepted median ratios range from 0.738 to 0.901. Each case uses two pairs with up to 80 samples per worker.
These original 128-pixel-wide fixtures do not allocate the optional native RGB table.
A separate 24-case plan adds 512 × 96 mixed-alpha and opaque repeated-color fixtures to exercise that allocation.
All 24 cases pass with exact frozen outputs. Wider byte-feedback ratios range from 0.445 to 0.704.
The original eight controls also pass in that run. All matching-feedback controls retain their native allocation cost and pass.
The earlier inconclusive trial remains in the evidence; it is not relabeled as a pass.

## Ownership and limits

Public byte feedback reserves optional cache storage only after mandatory memory fits.
Tight budgets keep the direct fallible scan. Failed scans never enter the cache.
Public matching feedback allocates no RGB table.
The native prepared API receives feedback later, so preparation may reserve an unused width-sized table for matching feedback.
That native cost stays in the wider controls instead of being excluded from timing.

Source commits are `5c001199`, `f89b4444`, `32622ebb` and `b9fba62f`, stacked on identity bypass `43fe220b`.
Native snapshots identify equivalent production code at `8164c2b2` and `95aad503` before the identity-only JavaScript change.
The current scalar Wasm snapshot and every measurement retain complete SHA-256 and source provenance.

The combined integration at `e927eacc` passes 452 native tests, 46 interface tests and 13 private Wasm tests.
Scalar and threaded package builds pass. The trusted freeze guard retains checkpoint `cef2b60a` and content digest `17ba3be3`.
Installed-package progress and source-snapshot checks pass in Chromium, optimizing Firefox and WebKit.
The combined tarball SHA-256 is `c63371359911ae9acbeeba18465a9f6fddbffa4eaee3dc33f3256382423ab47a`.
No PR merge, package publication or website activation is part of this work.

Raw native workers, browser samples, manifests, provenance and validation logs are in `diffusion-hotpaths-evidence.tar.xz` beside this report.
Archive SHA-256 is `19f10b6b931f24312e1f258093011c26f23e851cd46601a192c307a919c5ef30`.
The three native trials retain 160 serial workers and 10,880 samples. Every recorded output matches frozen bytes.
The archive retains the original screening scripts with hashes matching their manifests, plus the later six-sample protocol.
PNG files and executable snapshots remain in the corresponding worktree benchmark directories, outside compiler targets.
