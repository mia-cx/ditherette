# Native scalar measurements

The baseline completes 101 spec/production cases. It retains 56 pass, five incorrect, 34 regression, and six inconclusive gates.
The combined converter candidate completes 38 exact cases, with 33 pass and five inconclusive gates.
A separate five-case repeat resolves both nearest controls. Tiny inverse, random threshold, and Yliluoma remain inconclusive.

The six ordinary perturb cases improve 12.18–54.91× against previous production. Seven placement components improve 7.31–8.88×.
All 15 measured field-related cases pass, including zero-strength and tiny perturb controls.
These results support selecting the field-only change. Final field-only source/artifact measurements remain pending below.
Yliluoma stays held because its modified tiny path remains inconclusive after the declared repeat.

This is native scalar evidence on one machine. It does not rank browser/Wasm, threaded execution, or TypeScript implementations.
Baseline regression means production is slower than frozen spec under this scope. It does not identify a newly introduced regression.

## Evidence and execution

Raw directories share this root:

```text
/home/mia/mia-cx/ditherette/.worktrees/v1-scalar-spec-bench/target/scalar-comparison/
```

| Run directory | Cases | Started/reaped | Actual samples | Gate |
| --- | ---: | ---: | ---: | --- |
| spec-prod-baseline-results | 101 | 404/404 | 7,619 | incorrect |
| prod-selection-results | 38 | 152/152 | 2,675 | inconclusive |
| prod-selection-repeat-results | 5 | 20/20 | 400 | inconclusive |
| Total, including the separate repeat | 144 | 576/576 | 10,694 | No aggregate acceptance claim |

The 144 entries are 101 baseline cases, 38 selection cases, and five repeat cases. They are not 144 distinct recipes.
Each run uses two alternating AB/BA pairs, single-call sampling, 50 ms warmup, and a 250 ms measurement cap.
Each worker retains five to twenty samples. The declared target-sample setting is 2 ms; every actual sample contains one call.
All application-cache settings are `not-applicable`. Lower sample counts reflect the existing cap and minimum, not missing workers.

The audit reads every raw result and verifies pooled medians and both pair ratios against the report.
It checks clean source identities, single-call iteration counts, finite samples, exact trial/pid reaps, and AB/BA order.
All 1,728 lifecycle events contain matching starting/started/reaped records. The maximum live owned worker count is one.
Every worker also reports a maximum of one live benchmark process. No owned worker overlaps another across these three runs.
These journals establish owned worker serialization. The coordinator's quiet-host attestation separately covers other builds/tests/jobs.

| Run | First launch admission, UTC | Final reap, UTC |
| --- | --- | --- |
| Baseline | 2026-09-09 21:24:01.346 | 2026-09-09 21:25:30.396 |
| Selection | 2026-09-09 21:30:07.814 | 2026-09-09 21:30:45.293 |
| Repeat | 2026-09-09 21:31:28.701 | 2026-09-09 21:31:30.062 |

Machine is `athena-hephaestus`, AMD Ryzen 9 7950X 16-Core Processor, with 24 exposed logical CPUs.
OS is Linux x86_64, kernel `6.12.95+deb13-amd64`.
All workers record Rust 1.97.0 (`2d8144b78`, 2026-07-07), LLVM 22.1.6, and benchmark tool version 0.1.0.

| Artifact | Source revision | Worker SHA-256 |
| --- | --- | --- |
| Baseline, both algorithm roles; selection/repeat accepted | `2b9bbd68bc26d8ec07ab9bd273f2e7aeef241fb5` | `0e0b1c39e097d83541d61269b9edbc7c88f699c51470a1059af6d694026eba30` |
| Selection/repeat combined candidate | `086bbd47fb40ff0d439aed83e285b67b79caa46c` | `55fecb032fbec424f20cf223e42e448e4d8f4fb3c0ab31e71abd97b5b018f6e6` |

The baseline uses one executable containing real frozen and production callables. Its two roles select different algorithms.
The selection and repeat use different revisions with the same production subject IDs.
The candidate includes field/placement converter reuse and Yliluoma target-converter reuse. Its Yliluoma change is held.
These worker hashes are recorded artifact identities checked against every result. This audit hashes result files, not executable bytes.

Coordinator-provided archival location is `benchmark-results/scalar-spec-prod-2026-09-09/baseline-and-selection.tar.gz` in the benchmark worktree.
Its recorded SHA-256 is `41dcc49051989ae51b7d57443ccf595e20ec52e1210e595cd74f36fe0b2ea1ca`.
The coordinator reports 481,111,068 archive bytes and a successful byte-for-byte `tar -d` comparison against originals.
Root owns archive validation and retention. This reporting pass does not extract, delete, or modify sibling evidence.

## Timing scopes and coverage

| Cases | Scope and timed work | Ordinary fixture |
| --- | --- | --- |
| resize-* | Registered scalar resize call, caller-owned output; algorithm-specific plan/scratch work remains inside its adapter | 512×384 → 173×129 |
| forward-* | Frozen image forward export versus prepared production packed writer; converter creation and output allocation precede timing | 512×384 |
| inverse-* | Frozen/production f32 image inverses over the same precomputed frozen coordinates; output storage prepared before timing | 512×384 |
| source-construction-* | Per-pixel source conversion including production converter construction | 128×96 |
| reconstruct-* | f64 reconstruction into prepared RGBA8; frozen-generated coordinates include domain offsets before timing | 512×384 |
| placement-* | Full mask grid, including each selected placement call's converter work and source-neighbor reads | 128×96, adaptive radius 1 |
| scores-* | Metric formula batches over precomputed frozen coordinate pairs; output allocation and function selection precede timing | 512×384 |
| threshold-* | Full threshold grid over global coordinates or seeded pixel indices | 512×384 |
| perturb-* | Complete perturb pixel loop into preallocated RGBA8, including converter setup and adaptive reads; validation/storage precede timing | 128×96 |
| quantize-* and diffusion-* | Borrowed-source complete native request, including validation, preparation, owned output allocation, and destruction | 128×96 |
| yliluoma-* | Same borrowed complete-call scope, including exhaustive pair/ratio search | 32×24, palette8 |

Tiny cases use 1×1 sources. Tiny nearest produces 2×2 output; other tiny outputs remain 1×1.
The ordinary quantize cases cover all fifteen matching policies with palette32, including one transparent entry.
All ordinary and tiny diffusion cases use Oklab Euclidean matching and palette32.
The `srgb-bytes` diffusion label identifies feedback, not matching space. All four kernels cover both feedback modes.
Ordinary diffusion uses strength 0.7, serpentine scan, and everywhere placement.
Yliluoma uses sRGB Euclidean matching, everywhere placement, and Bayer sizes 2, 4, 8, and 16.

All seven spaces have forward, inverse, source-construction, reconstruction, and placement rows.
Seven score rows cover shared formula families; fifteen full quantize rows cover policy dispatch and palette preparation.
This matrix does not isolate palette construction or prepared nearest scans, nor time composed Process/separable exports.
Perturb rows exclude Processor boundaries, hashing, source snapshots, caches, and public JS copies.
Borrowed complete calls also exclude those Processor/public costs. No public wrapper timing is presented as raw kernel timing.

## Baseline medians

Values below are exact pooled medians in nanoseconds from actual arrays.
C/A is candidate divided by accepted, rounded to four decimals. Below one means the candidate role is faster.
Sample counts show the actual pooled accepted/candidate counts from two workers per role.
A pass means the existing gate passes; it does not require an improvement.

| Case | Accepted ns | Candidate ns | C/A | Samples A/C | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| resize-nearest-scalar | 29760 | 5595 | 0.1880 | 40/40 | pass |
| resize-area-scalar | 1146795.5 | 603015.5 | 0.5258 | 40/40 | incorrect |
| resize-bilinear-scalar | 2088496.5 | 885761 | 0.4241 | 40/40 | incorrect |
| resize-bicubic-catmull-rom | 1826901 | 816719 | 0.4471 | 40/40 | pass |
| resize-bicubic-catmull-rom-scale-aware | 10962655.5 | 1336718.5 | 0.1219 | 40/40 | incorrect |
| resize-lanczos2-fixed | 6974993 | 836700.5 | 0.1200 | 40/40 | pass |
| resize-lanczos2-scale-aware | 50570221 | 1402058.5 | 0.0277 | 10/40 | incorrect |
| resize-lanczos3-fixed | 14202867 | 1488096 | 0.1048 | 36/40 | pass |
| resize-lanczos3-scale-aware | 111213224 | 1968525 | 0.0177 | 10/40 | incorrect |
| resize-trilinear-mip-area | 4427748 | 3834498 | 0.8660 | 40/40 | pass |
| forward-srgb | 187018 | 920295.5 | 4.9209 | 40/40 | regression |
| inverse-srgb | 944771.5 | 948007 | 1.0034 | 40/40 | pass |
| source-construction-srgb | 32271 | 14185278 | 439.5674 | 40/35 | regression |
| reconstruct-srgb | 2734163 | 2771733 | 1.0137 | 40/40 | pass |
| placement-srgb | 1122680 | 129474907.5 | 115.3266 | 40/10 | regression |
| forward-linear-rgb | 2348045 | 912446 | 0.3886 | 40/40 | pass |
| inverse-linear-rgb | 3472611 | 3448420.5 | 0.9930 | 40/40 | pass |
| source-construction-linear-rgb | 283060 | 14134342 | 49.9341 | 40/36 | regression |
| reconstruct-linear-rgb | 5061028.5 | 5095955 | 1.0069 | 40/40 | pass |
| placement-linear-rgb | 4538589.5 | 129572257 | 28.5490 | 40/10 | regression |
| forward-oklab | 10022791.5 | 2713727 | 0.2708 | 40/40 | pass |
| inverse-oklab | 4140187 | 4139912.5 | 0.9999 | 40/40 | pass |
| source-construction-oklab | 629175 | 14224374.5 | 22.6080 | 40/36 | regression |
| reconstruct-oklab | 8350971.5 | 8362222.5 | 1.0013 | 40/40 | pass |
| placement-oklab | 7708665 | 132418744 | 17.1779 | 40/10 | regression |
| forward-cielab | 9870197 | 2933862 | 0.2972 | 40/40 | pass |
| inverse-cielab | 4423597 | 4353496 | 0.9842 | 40/40 | pass |
| source-construction-cielab | 625875.5 | 14810868 | 23.6642 | 40/35 | regression |
| reconstruct-cielab | 5448845 | 5458960 | 1.0019 | 40/40 | pass |
| placement-cielab | 7908292 | 133472370.5 | 16.8775 | 40/10 | regression |
| forward-ycbcr | 245094 | 963716.5 | 3.9320 | 40/40 | regression |
| inverse-ycbcr | 1232667 | 1233331.5 | 1.0005 | 40/40 | pass |
| source-construction-ycbcr | 61111 | 14319087 | 234.3128 | 40/36 | regression |
| reconstruct-ycbcr | 2850920.5 | 2860919.5 | 1.0035 | 40/40 | pass |
| placement-ycbcr | 1967649.5 | 131138652 | 66.6474 | 40/10 | regression |
| forward-oklch | 14493579.5 | 5486690 | 0.3786 | 36/40 | pass |
| inverse-oklch | 6803639 | 6818279 | 1.0022 | 40/40 | pass |
| source-construction-oklch | 912206 | 14533353 | 15.9321 | 40/36 | regression |
| reconstruct-oklch | 7699880.5 | 7673123.5 | 0.9965 | 40/40 | pass |
| placement-oklch | 10206673 | 134972331.5 | 13.2239 | 40/10 | regression |
| forward-cielch | 13974943.5 | 6293083.5 | 0.4503 | 36/40 | pass |
| inverse-cielch | 6656209.5 | 6731637.5 | 1.0113 | 40/40 | pass |
| source-construction-cielch | 878175.5 | 14641434 | 16.6726 | 40/35 | regression |
| reconstruct-cielch | 8286089.5 | 8335471.5 | 1.0060 | 40/40 | pass |
| placement-cielch | 9953253 | 134440395.5 | 13.5072 | 40/10 | regression |
| scores-euclidean | 305700.5 | 370667 | 1.2125 | 40/40 | inconclusive |
| scores-chord | 1123199.5 | 1018318 | 0.9066 | 40/40 | inconclusive |
| scores-arc | 640481.5 | 751318.5 | 1.1731 | 40/40 | regression |
| scores-compuphase | 1377374 | 1380014 | 1.0019 | 40/40 | pass |
| scores-rec601 | 297185 | 372961 | 1.2550 | 40/40 | regression |
| scores-rec709 | 295760 | 302280.5 | 1.0220 | 40/40 | pass |
| scores-ciede2000 | 14573405 | 14533812 | 0.9973 | 35/35 | pass |
| quantize-srgb-euclidean-palette32 | 880434.5 | 920906 | 1.0460 | 40/40 | pass |
| quantize-linear-rgb-euclidean-palette32 | 1285471.5 | 918915.5 | 0.7148 | 40/40 | pass |
| quantize-oklab-euclidean-palette32 | 1636724 | 1165495.5 | 0.7121 | 40/40 | pass |
| quantize-cielab-euclidean-palette32 | 1644289 | 1190676 | 0.7241 | 40/40 | pass |
| quantize-ycbcr-euclidean-palette32 | 969347 | 956766 | 0.9870 | 40/40 | pass |
| quantize-srgb-compuphase-palette32 | 3087278.5 | 3052998 | 0.9889 | 40/40 | pass |
| quantize-srgb-rec601-palette32 | 918115.5 | 1005643 | 1.0953 | 40/40 | inconclusive |
| quantize-srgb-rec709-palette32 | 906800 | 974522 | 1.0747 | 40/40 | pass |
| quantize-oklch-euclidean-palette32 | 1914793 | 1366164 | 0.7135 | 40/40 | pass |
| quantize-oklch-circular-hue-palette32 | 3432333.5 | 3535541 | 1.0301 | 40/40 | pass |
| quantize-oklch-hue-arc-palette32 | 2573070.5 | 3317803.5 | 1.2894 | 40/40 | regression |
| quantize-cielab-ciede2000-palette32 | 28952173.5 | 28931045.5 | 0.9993 | 18/18 | pass |
| quantize-cielch-euclidean-palette32 | 1893553 | 1408894 | 0.7440 | 40/40 | pass |
| quantize-cielch-circular-hue-palette32 | 3437010.5 | 3567266.5 | 1.0379 | 40/40 | pass |
| quantize-cielch-hue-arc-palette32 | 2536298.5 | 3385690 | 1.3349 | 40/40 | regression |
| threshold-bayer2 | 275664.5 | 272659.5 | 0.9891 | 40/40 | pass |
| perturb-bayer2-srgb | 196468.5 | 14285786 | 72.7129 | 40/36 | regression |
| threshold-bayer4 | 379281.5 | 379456.5 | 1.0005 | 40/40 | pass |
| perturb-bayer4-linear-rgb | 830130 | 14628021.5 | 17.6214 | 40/36 | regression |
| threshold-bayer8 | 483192.5 | 479268 | 0.9919 | 40/40 | pass |
| perturb-bayer8-oklab | 8334729.5 | 139042358 | 16.6823 | 40/10 | regression |
| threshold-bayer16 | 578460.5 | 574160 | 0.9926 | 40/40 | pass |
| perturb-bayer16-cielab | 1342483 | 14985487 | 11.1625 | 40/34 | regression |
| threshold-random | 210488.5 | 210413.5 | 0.9996 | 40/40 | pass |
| perturb-random-oklch | 10260033 | 134438551 | 13.1031 | 40/10 | regression |
| threshold-blue | 94847 | 94527.5 | 0.9966 | 40/40 | pass |
| perturb-blue-cielch | 1728380 | 15462350.5 | 8.9462 | 40/34 | regression |
| diffusion-floyd-steinberg-srgb-bytes | 2044371 | 1755920.5 | 0.8589 | 40/40 | pass |
| diffusion-floyd-steinberg-matching | 1892913 | 1793146 | 0.9473 | 40/40 | pass |
| diffusion-sierra-srgb-bytes | 2177874 | 2202193 | 1.0112 | 40/40 | pass |
| diffusion-sierra-matching | 2060560.5 | 2253919 | 1.0938 | 40/40 | inconclusive |
| diffusion-sierra-lite-srgb-bytes | 2049405.5 | 1686524 | 0.8229 | 40/40 | pass |
| diffusion-sierra-lite-matching | 1917098.5 | 1732165.5 | 0.9035 | 40/40 | pass |
| diffusion-atkinson-srgb-bytes | 2113692 | 1902623 | 0.9001 | 40/40 | pass |
| diffusion-atkinson-matching | 1933943 | 1931998.5 | 0.9990 | 40/40 | pass |
| yliluoma-2 | 336756 | 1245701.5 | 3.6991 | 40/40 | regression |
| yliluoma-4 | 1069573 | 1932763 | 1.8070 | 40/40 | regression |
| yliluoma-8 | 3836247.5 | 4754892.5 | 1.2395 | 40/40 | regression |
| yliluoma-16 | 14888938.5 | 15710565.5 | 1.0552 | 34/32 | pass |
| perturb-zero-strength | 40460 | 50170 | 1.2400 | 40/40 | inconclusive |
| tiny-resize-nearest-scalar | 40 | 50 | 1.2500 | 40/40 | regression |
| tiny-forward-srgb | 20 | 30 | 1.5000 | 40/40 | regression |
| tiny-inverse-srgb | 30.5 | 40 | 1.3115 | 40/40 | inconclusive |
| tiny-scores-euclidean | 20 | 20 | 1.0000 | 40/40 | pass |
| tiny-quantize-srgb-euclidean-palette32 | 530 | 1560 | 2.9434 | 40/40 | regression |
| tiny-threshold-random | 20 | 20.5 | 1.0250 | 40/40 | pass |
| tiny-diffusion-floyd-steinberg-srgb-bytes | 2579.5 | 2105 | 0.8160 | 40/40 | pass |
| tiny-yliluoma-2 | 621 | 2950 | 4.7504 | 40/40 | regression |
| tiny-perturb-bayer4-linear-rgb | 80 | 1180.5 | 14.7562 | 40/40 | regression |

## Five existing resize mismatches

Each output contains 22,317 pixels and 89,268 RGBA bytes. All four verification entries per case retain the same mismatch metrics.
Reference comparisons remain exact; the production result differs from frozen. The reports classify all five cases as incorrect.

| Case | Differing bytes | Differing pixels | Maximum byte difference | First byte; (x,y) | Frozen RGBA → production RGBA | Metadata mismatch |
| --- | ---: | ---: | ---: | --- | --- | --- |
| resize-area-scalar | 181 | 181 | 1 | 2839; (17, 4) | [205,65,107,145] → [205,65,107,146] | alpha |
| resize-bilinear-scalar | 243 | 243 | 1 | 346; (86, 0) | [90,56,47,143] → [90,56,46,143] | none |
| resize-bicubic-catmull-rom-scale-aware | 247 | 247 | 1 | 1730; (86, 2) | [39,156,46,108] → [39,156,47,108] | none |
| resize-lanczos2-scale-aware | 236 | 236 | 1 | 1038; (86, 1) | [42,154,47,90] → [42,154,46,90] | none |
| resize-lanczos3-scale-aware | 239 | 239 | 1 | 1038; (86, 1) | [36,161,47,87] → [36,161,46,87] | none |

All maximum per-pixel RGBA Euclidean distances are one. All changed pixels differ in exactly one channel by one byte.
Area's reported alpha mismatch accompanies alpha-byte drift. The other four cases report no metadata mismatch.
These metrics document the drift. They grant no visual approval, tolerance change, or release acceptance.
The other 96 baseline cases have exact report comparisons, regardless of their timing gates.

## Combined candidate selection

Accepted is previous production; candidate contains both field/placement and Yliluoma converter reuse.
All 152 verification entries pass exactness. All 38 cases are exact across native frozen and both production roles.
The six ordinary perturb rows improve 12.18–54.91×. Adaptive placement components improve 7.31–8.88×.
Every affected field row passes, including zero-strength and tiny perturb controls.

Four ordinary Yliluoma rows improve 1.07–3.74×, but the modified tiny Yliluoma row is inconclusive.
These are observed candidate gains. Holding Yliluoma means its gains cannot be attributed to the final field-only implementation.

| Case | Accepted ns | Candidate ns | C/A | Samples A/C | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| resize-nearest-scalar | 5545 | 9465 | 1.7069 | 40/40 | inconclusive |
| forward-srgb | 919199 | 930314 | 1.0121 | 40/40 | pass |
| placement-srgb | 130756473.5 | 14718918 | 0.1126 | 10/35 | pass |
| placement-linear-rgb | 131000382.5 | 14917556 | 0.1139 | 10/34 | pass |
| placement-oklab | 134074022 | 16442836 | 0.1226 | 10/32 | pass |
| placement-cielab | 133730396 | 16512057 | 0.1235 | 10/31 | pass |
| placement-ycbcr | 130695415 | 14944801 | 0.1143 | 10/34 | pass |
| placement-oklch | 134760561.5 | 18094337 | 0.1343 | 10/28 | pass |
| placement-cielch | 135933295 | 18588800.5 | 0.1367 | 10/28 | pass |
| quantize-srgb-euclidean-palette32 | 921500.5 | 931844.5 | 1.0112 | 40/40 | pass |
| perturb-bayer2-srgb | 14402172.5 | 265359 | 0.0184 | 36/40 | pass |
| perturb-bayer4-linear-rgb | 14739769 | 610665 | 0.0414 | 34/40 | pass |
| perturb-bayer8-oklab | 139593134 | 2542050.5 | 0.0182 | 10/40 | pass |
| perturb-bayer16-cielab | 15223870.5 | 851568.5 | 0.0559 | 34/40 | pass |
| perturb-random-oklch | 135487787 | 3922062.5 | 0.0289 | 10/40 | pass |
| perturb-blue-cielch | 15710154 | 1289801 | 0.0821 | 32/40 | pass |
| diffusion-floyd-steinberg-srgb-bytes | 1756439 | 1753648.5 | 0.9984 | 40/40 | pass |
| diffusion-floyd-steinberg-matching | 1820904 | 1862036 | 1.0226 | 40/40 | pass |
| diffusion-sierra-srgb-bytes | 2209976 | 2222196 | 1.0055 | 40/40 | pass |
| diffusion-sierra-matching | 2259262 | 2281372 | 1.0098 | 40/40 | pass |
| diffusion-sierra-lite-srgb-bytes | 1703247.5 | 1682966.5 | 0.9881 | 40/40 | pass |
| diffusion-sierra-lite-matching | 1748342.5 | 1782994.5 | 1.0198 | 40/40 | pass |
| diffusion-atkinson-srgb-bytes | 1913246.5 | 1910186 | 0.9984 | 40/40 | pass |
| diffusion-atkinson-matching | 1939646 | 1979287 | 1.0204 | 40/40 | pass |
| yliluoma-2 | 1249450 | 333800 | 0.2672 | 40/40 | pass |
| yliluoma-4 | 1975697 | 1055452.5 | 0.5342 | 40/40 | pass |
| yliluoma-8 | 4746126 | 3861516.5 | 0.8136 | 40/40 | pass |
| yliluoma-16 | 15778189.5 | 14760088 | 0.9355 | 32/35 | pass |
| perturb-zero-strength | 50341 | 51630.5 | 1.0256 | 40/40 | pass |
| tiny-resize-nearest-scalar | 50 | 49 | 0.9800 | 40/40 | inconclusive |
| tiny-forward-srgb | 30 | 30 | 1.0000 | 40/40 | pass |
| tiny-inverse-srgb | 30.5 | 40 | 1.3115 | 40/40 | inconclusive |
| tiny-scores-euclidean | 20 | 20 | 1.0000 | 40/40 | pass |
| tiny-quantize-srgb-euclidean-palette32 | 1560 | 1570 | 1.0064 | 40/40 | pass |
| tiny-threshold-random | 20.5 | 21 | 1.0244 | 40/40 | inconclusive |
| tiny-diffusion-floyd-steinberg-srgb-bytes | 2110 | 2105 | 0.9976 | 40/40 | pass |
| tiny-yliluoma-2 | 2960 | 2645 | 0.8936 | 40/40 | inconclusive |
| tiny-perturb-bayer4-linear-rgb | 1170 | 1190 | 1.0171 | 40/40 | pass |

## Declared repeat and remaining diagnoses

The repeat uses the same revisions, recipes, two pairs, and collector settings. Its evidence remains separate from the first run.
All twenty verification entries are exact. Both nearest controls pass; three tiny controls remain inconclusive.
Every repeat worker retains twenty samples. No pooled value below replaces an original case or clears a failed gate.

| Case | Accepted ns | Candidate ns | C/A | Samples A/C | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| resize-nearest-scalar | 5541 | 5569.5 | 1.0051 | 40/40 | pass |
| tiny-resize-nearest-scalar | 50 | 50 | 1.0000 | 40/40 | pass |
| tiny-inverse-srgb | 30.5 | 35.5 | 1.1639 | 40/40 | inconclusive |
| tiny-threshold-random | 21 | 21 | 1.0000 | 40/40 | inconclusive |
| tiny-yliluoma-2 | 2960 | 1810 | 0.6115 | 40/40 | inconclusive |

| Still inconclusive | Repeat pair C/A ratios | Interpretation |
| --- | --- | --- |
| tiny-inverse-srgb | 1.142857, 1.016393 | Unchanged inverse code; approximately 30–40 ns calls cannot establish a consistent comparison here |
| tiny-threshold-random | 1.450000, 0.724138 | Unchanged threshold code; opposing pair ratios remain noisy despite an equal pooled median |
| tiny-yliluoma-2 | 0.604027, 0.963664 | Modified code; large pair disagreement prevents selecting this candidate |

All three reports record `resolution_limited: false` for these rows. This audit preserves that classification.
The short durations and pair disagreement support a noise diagnosis, not a relabeled timer-resolution failure or confirmed regression.
The field-only decision relies on passing affected field gates. Unchanged inverse/threshold diagnoses remain visible.

## Actual sample-count exceptions

The main tables show role totals. The following rows retain fewer than twenty observations in one or both workers.
Values show pair-zero plus pair-one counts. No count is inferred from the declared maximum.

### Spec / previous production baseline

- resize-lanczos2-scale-aware: A 5+5; C 20+20.
- resize-lanczos3-fixed: A 18+18; C 20+20.
- resize-lanczos3-scale-aware: A 5+5; C 20+20.
- source-construction-srgb: A 20+20; C 17+18.
- placement-srgb: A 20+20; C 5+5.
- source-construction-linear-rgb: A 20+20; C 18+18.
- placement-linear-rgb: A 20+20; C 5+5.
- source-construction-oklab: A 20+20; C 18+18.
- placement-oklab: A 20+20; C 5+5.
- source-construction-cielab: A 20+20; C 18+17.
- placement-cielab: A 20+20; C 5+5.
- source-construction-ycbcr: A 20+20; C 18+18.
- placement-ycbcr: A 20+20; C 5+5.
- forward-oklch: A 18+18; C 20+20.
- source-construction-oklch: A 20+20; C 18+18.
- placement-oklch: A 20+20; C 5+5.
- forward-cielch: A 18+18; C 20+20.
- source-construction-cielch: A 20+20; C 17+18.
- placement-cielch: A 20+20; C 5+5.
- scores-ciede2000: A 17+18; C 17+18.
- quantize-cielab-ciede2000-palette32: A 9+9; C 9+9.
- perturb-bayer2-srgb: A 20+20; C 18+18.
- perturb-bayer4-linear-rgb: A 20+20; C 18+18.
- perturb-bayer8-oklab: A 20+20; C 5+5.
- perturb-bayer16-cielab: A 20+20; C 17+17.
- perturb-random-oklch: A 20+20; C 5+5.
- perturb-blue-cielch: A 20+20; C 17+17.
- yliluoma-16: A 17+17; C 16+16.

### Previous production / combined candidate

- placement-srgb: A 5+5; C 17+18.
- placement-linear-rgb: A 5+5; C 17+17.
- placement-oklab: A 5+5; C 16+16.
- placement-cielab: A 5+5; C 16+15.
- placement-ycbcr: A 5+5; C 17+17.
- placement-oklch: A 5+5; C 14+14.
- placement-cielch: A 5+5; C 14+14.
- perturb-bayer2-srgb: A 18+18; C 20+20.
- perturb-bayer4-linear-rgb: A 17+17; C 20+20.
- perturb-bayer8-oklab: A 5+5; C 20+20.
- perturb-bayer16-cielab: A 17+17; C 20+20.
- perturb-random-oklch: A 5+5; C 20+20.
- perturb-blue-cielch: A 16+16; C 20+20.
- yliluoma-16: A 16+16; C 17+18.

### Repeat of five inconclusive cases

Every worker retains 20 samples.

## Reproducible audit and file identities

Run the read-only helper against the retained result directories:

```sh
node .plans/83-scalar-report-audit.mjs \\
  ../v1-scalar-spec-bench/target/scalar-comparison/spec-prod-baseline-results \\
  ../v1-scalar-spec-bench/target/scalar-comparison/prod-selection-results \\
  ../v1-scalar-spec-bench/target/scalar-comparison/prod-selection-repeat-results
```

The helper prints JSON only. It checks actual samples, medians, pair ratios, source identities, and complete serial AB/BA journals.
Its worker inventory digest hashes sorted records of `filename + NUL + worker-file-SHA256 + newline`.
Report, prepared descriptor, event journal, and every result file were read directly for this audit.
Prepared descriptors bind full fixtures, settings, executable identities, and machine metadata without copying fixture bodies into this document.

### Spec / previous production baseline

```text
report.json      7f79f71f07fa35acef288228d60009a9ee5aa9b1eafba26a071378e55d406d51
prepared.json    498a38c14eadb68d5e665e95668d38043df765e42c150a1c1488ddbd11a23baa
events.jsonl     5b391f8245b9a1008639c91a8e9035836dd6e1cbe81a3e7006993c6791f5e0bf
worker inventory 34343ebb04c56060791c1006274a2aa674b37332c59cca575c49d678408f7919
```

### Previous production / combined candidate

```text
report.json      8e3819111b6fba1af06b1132a42d3a96ab775bbe1763be86632aea3000e9d820
prepared.json    6afd7e60fbf94fc47698b78356955e332291b8c4cf3cd59d17442aefc9ae108b
events.jsonl     c50fc5720c66a23465813f90824a0a24a3874361cd8eb6a7736cd1488447ba99
worker inventory f427f14cc69fe6011403b3f468d30b067b62b33b0f8ed91b775c72fc2bdb3b88
```

### Repeat of five inconclusive cases

```text
report.json      bff52ddba12193ebe790308275b4fba3c03a6b0323dae576c5b8f37e3461e9e5
prepared.json    1dc91a40c5ce65d9d8c8d994a88564d598c2ef11fb940db50e5ceb87235ca660
events.jsonl     dd7227550fadc749f228a1ec52317a62e6e5e79fee8959852af51b5494cca173
worker inventory 050c7c12dff542e2f3727eac5148c912977cf201d5d601c377869804046f5ee7
```

## Final selected implementation measurements

Pending coordinator completion. The following evidence is deliberately absent from this report version:

1. Exact field-only selected source revision and fresh executable identity after holding the Yliluoma change.
2. Final-selected spec/production and required previous-production comparisons, with their actual counts, medians, and gates.
3. Final artifact/archive verification, release-ledger reconciliation, and remaining scalar follow-up decisions.

The coordinator identifies fresh final candidate `8e09c05d9455af977746060c39d531848eee8b90`; its build and freeze guard are underway.
That candidate has no final measurement outcome in this checkpoint.
The first three runs above remain immutable historical evidence. Final measurements must identify their own source and timing scope.
This reporting pass changes only this document and its read-only audit helper. It runs no builds, tests, or benchmarks.
