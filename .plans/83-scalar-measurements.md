# Native scalar measurements

Field/placement converter reuse and packed forward loops are selected at `8e09c05d` after passing the affected previous-production gates.
The final 101-case spec/selected-production comparison records 66 pass, five incorrect, 25 regression, and five inconclusive gates.
All six retained runs contain 1,232 reaped workers and 23,149 actual samples. Full final results appear below.
CIELAB forward is 8.54% slower than previous production in its passing repeat. Yliluoma target-converter reuse remains held.

## Historical baseline and first candidate

The baseline completes 101 spec/production cases. It retains 56 pass, five incorrect, 34 regression, and six inconclusive gates.
The combined converter candidate completes 38 exact cases, with 33 pass and five inconclusive gates.
A separate five-case repeat resolves both nearest controls. Tiny inverse, random threshold, and Yliluoma remain inconclusive.

The six ordinary perturb cases improve 12.18–54.91× against previous production. Seven placement components improve 7.31–8.88×.
All 15 measured field-related cases pass, including zero-strength and tiny perturb controls.
These results support the field change; the final field-plus-packed selection appears below.
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

Select field/placement converter reuse and the packed forward-loop change at `8e09c05d9455af977746060c39d531848eee8b90`.
The fresh native executable has SHA-256 `1390d22c75a477fed99b5ee99daf0e364db7bbf9a8509bb5d2b0f75568564070`.
Its previous-production comparison passes every affected gate after one declared repeat of five inconclusive cases.
The original 58-case report remains 53 pass and five inconclusive. The separate repeat records five pass.
Those repeated results do not overwrite initial noise or combine samples with the first run.

The selected source changes only `prod/color/packed.rs`, `prod/dither/perturb.rs`, and `prod/dither/placement.rs` under production.
Its Yliluoma request file matches previous production byte-for-byte.
Yliluoma target-converter candidate `086bbd47` is absent. Its earlier measured gains remain unselected historical evidence.
The packed change moves sRGB/YCbCr dispatch outside pixel loops and uses fixed-size chunks with existing color helpers.
Field/placement conversion reuse preserves the selected arithmetic and source-neighbor semantics.

### Final execution and identity audit

| Run directory | Cases | Started/reaped | Actual samples | Report gates |
| --- | ---: | ---: | ---: | --- |
| packed-selection-results | 58 | 232/232 | 4,225 | 53 pass, 5 inconclusive |
| packed-selection-repeat-results | 5 | 20/20 | 386 | 5 pass |
| spec-prod-selected-results | 101 | 404/404 | 7,844 | 66 pass, 5 incorrect, 25 regression, 5 inconclusive |
| Final three runs | 164 entries | 656/656 | 12,455 | Separate comparisons |
| All six runs in this document | 308 entries | 1,232/1,232 | 23,149 | Includes distinct comparisons and repeats |

Counts come from every raw worker sample array. No worker or sample total assumes the configured maximum.
The final three journals have 1,968 matching starting/started/reaped events and exact sequential AB/BA order.
All workers report one maximum live benchmark process; all owned workers drain before the next starts.
The run windows also exclude overlap with the three earlier runs. This does not replace the coordinator's host-quiescence attestation.
Machine, Rust/LLVM versions, sampling policy, and native scalar scopes match the earlier recorded environment.

| Run | First launch admission, UTC | Final reap, UTC |
| --- | --- | --- |
| Packed selection | 2026-09-09 21:40:46.219 | 2026-09-09 21:41:38.184 |
| Packed repeat | 2026-09-09 21:42:08.939 | 2026-09-09 21:42:12.355 |
| Selected spec/production | 2026-09-09 21:42:50.758 | 2026-09-09 21:44:08.830 |

Selection and repeat use previous-production artifact `2b9bbd68` as accepted and selected `8e09c05d` as candidate.
Final spec/production uses `8e09c05d` and its same executable hash for both roles, dispatching frozen versus production algorithms.
All worker source/artifact identities match their prepared descriptors. Raw samples reproduce every median and alternating pair ratio.

The selected spec/production experiment object equals the original full experiment object exactly, including all 101 recipes and notes.
It reuses `spec-prod-baseline.json`, SHA-256 `578dc59b9fb9404eae5471bb1d013c3dc7b181f6e99bcadf8d161d4c131c55e8`.
The unchanged wording "baseline" describes this new selected-production baseline. It does not indicate reuse of earlier measurements.
Canonical `JSON.stringify(experiment)` SHA-256 is `5476bc71774593d264cf8445ffbfd38ff338bbf27eedc07a9da2d037eb5f67a7` in both prepared descriptors.
The packed selection plan SHA-256 is `832608c1dae2d449938be5c588456191d3745bc3048af07f5bab90e01ed7c25b`.
The packed repeat plan SHA-256 is `413d0175a29b7b69b673d872922d6b23c4c9effa81d500b288ccfb02070f7698`.
Every repeat recipe equals its corresponding first-run recipe, including identities, settings, and timing policy.

### Selected production versus previous production

Ordinary sRGB and YCbCr packed forward conversion improve 5.59× and 6.52× respectively.
The six ordinary perturb cases improve 11.91–55.52×; seven placement grids improve 7.11–8.72×.
These are direct previous/selected-production comparisons, separate from the spec-relative results below.
All 232 selection verification entries and twenty repeat entries pass exactness.

| Case | Accepted ns | Candidate ns | C/A | Samples A/C | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| resize-nearest-scalar | 5590 | 5695 | 1.0188 | 40/40 | pass |
| forward-srgb | 925504 | 165527.5 | 0.1789 | 40/40 | pass |
| placement-srgb | 130794232 | 14997342.5 | 0.1147 | 10/34 | pass |
| forward-linear-rgb | 913363.5 | 879172.5 | 0.9626 | 40/40 | pass |
| placement-linear-rgb | 129722645.5 | 14883401.5 | 0.1147 | 10/34 | pass |
| forward-oklab | 2673135.5 | 2611894.5 | 0.9771 | 40/40 | pass |
| placement-oklab | 132598947.5 | 16621741 | 0.1254 | 10/31 | pass |
| forward-cielab | 2981030.5 | 3173553 | 1.0646 | 40/40 | inconclusive |
| placement-cielab | 131781000 | 16586501 | 0.1259 | 10/31 | pass |
| forward-ycbcr | 959604.5 | 147092.5 | 0.1533 | 40/40 | pass |
| placement-ycbcr | 129957929.5 | 15137109.5 | 0.1165 | 10/34 | pass |
| forward-oklch | 5465593 | 5422887.5 | 0.9922 | 40/40 | pass |
| placement-oklch | 135838439.5 | 18372298 | 0.1353 | 10/28 | pass |
| forward-cielch | 6257350.5 | 6270605 | 1.0021 | 40/40 | pass |
| placement-cielch | 134098161.5 | 18871667 | 0.1407 | 10/27 | pass |
| quantize-srgb-euclidean-palette32 | 921404 | 929509 | 1.0088 | 40/40 | pass |
| quantize-linear-rgb-euclidean-palette32 | 938074.5 | 923419.5 | 0.9844 | 40/40 | pass |
| quantize-oklab-euclidean-palette32 | 1160038 | 1155022.5 | 0.9957 | 40/40 | pass |
| quantize-cielab-euclidean-palette32 | 1176873.5 | 1211498 | 1.0294 | 40/40 | pass |
| quantize-ycbcr-euclidean-palette32 | 959125 | 957114 | 0.9979 | 40/40 | pass |
| quantize-srgb-compuphase-palette32 | 3054131.5 | 3084282 | 1.0099 | 40/40 | pass |
| quantize-srgb-rec601-palette32 | 1006480 | 990159.5 | 0.9838 | 40/40 | pass |
| quantize-srgb-rec709-palette32 | 975894 | 969369.5 | 0.9933 | 40/40 | pass |
| quantize-oklch-euclidean-palette32 | 1378490.5 | 1372576 | 0.9957 | 40/40 | pass |
| quantize-oklch-circular-hue-palette32 | 3516263 | 3492804 | 0.9933 | 40/40 | pass |
| quantize-oklch-hue-arc-palette32 | 3410507 | 3310280 | 0.9706 | 40/40 | pass |
| quantize-cielab-ciede2000-palette32 | 28497858.5 | 28446771 | 0.9982 | 18/18 | pass |
| quantize-cielch-euclidean-palette32 | 1422641 | 1427956.5 | 1.0037 | 40/40 | pass |
| quantize-cielch-circular-hue-palette32 | 3626064 | 3544724 | 0.9776 | 40/40 | pass |
| quantize-cielch-hue-arc-palette32 | 3352741 | 3325370.5 | 0.9918 | 40/40 | pass |
| perturb-bayer2-srgb | 14372824 | 265884 | 0.0185 | 36/40 | pass |
| perturb-bayer4-linear-rgb | 14561130.5 | 607944 | 0.0418 | 36/40 | pass |
| perturb-bayer8-oklab | 139557754.5 | 2513473.5 | 0.0180 | 10/40 | pass |
| perturb-bayer16-cielab | 15195975.5 | 862838.5 | 0.0568 | 34/40 | pass |
| perturb-random-oklch | 135805960 | 3902279 | 0.0287 | 10/40 | pass |
| perturb-blue-cielch | 15601171.5 | 1309640 | 0.0839 | 32/40 | pass |
| diffusion-floyd-steinberg-srgb-bytes | 1766086.5 | 1763706.5 | 0.9987 | 40/40 | pass |
| diffusion-floyd-steinberg-matching | 1815893 | 1826822.5 | 1.0060 | 40/40 | pass |
| diffusion-sierra-srgb-bytes | 2189648 | 2207423.5 | 1.0081 | 40/40 | pass |
| diffusion-sierra-matching | 2256914.5 | 2239663.5 | 0.9924 | 40/40 | pass |
| diffusion-sierra-lite-srgb-bytes | 1679030.5 | 1686871 | 1.0047 | 40/40 | pass |
| diffusion-sierra-lite-matching | 1731740.5 | 1774631 | 1.0248 | 40/40 | pass |
| diffusion-atkinson-srgb-bytes | 1911914.5 | 1903753.5 | 0.9957 | 40/40 | pass |
| diffusion-atkinson-matching | 1933969.5 | 1928308.5 | 0.9971 | 40/40 | pass |
| yliluoma-2 | 1238308.5 | 1278629.5 | 1.0326 | 40/40 | pass |
| yliluoma-4 | 1933774 | 1965030 | 1.0162 | 40/40 | pass |
| yliluoma-8 | 4721982 | 4738828 | 1.0036 | 40/40 | pass |
| yliluoma-16 | 15677987 | 15969052 | 1.0186 | 33/29 | inconclusive |
| perturb-zero-strength | 49901 | 59245.5 | 1.1873 | 40/40 | inconclusive |
| tiny-resize-nearest-scalar | 50 | 50 | 1.0000 | 40/40 | inconclusive |
| tiny-forward-srgb | 30 | 20 | 0.6667 | 40/40 | pass |
| tiny-inverse-srgb | 40 | 30 | 0.7500 | 40/40 | pass |
| tiny-scores-euclidean | 20 | 20 | 1.0000 | 40/40 | pass |
| tiny-quantize-srgb-euclidean-palette32 | 1590 | 1560 | 0.9811 | 40/40 | pass |
| tiny-threshold-random | 21 | 20 | 0.9524 | 40/40 | inconclusive |
| tiny-diffusion-floyd-steinberg-srgb-bytes | 2110 | 2110.5 | 1.0002 | 40/40 | pass |
| tiny-yliluoma-2 | 2931 | 2991 | 1.0205 | 40/40 | pass |
| tiny-perturb-bayer4-linear-rgb | 1170 | 1210 | 1.0342 | 40/40 | pass |

### Single repeat and the CIELAB tradeoff

All five repeated cases pass. Yliluoma size16 retains 16+17 samples per role; every other repeated role retains 20+20.
This accounts for 386 actual samples rather than the 400-sample maximum.

CIELAB forward rises from 2,947,605 ns to 3,199,324 ns, an 8.54% slower pooled median.
Its pair ratios are 1.075418 and 1.089435. Both remain within the declared 10% regression limit.
The original CIELAB run is inconclusive, with pair ratios 1.109526 and 1.058116.
Selecting the packed change preserves this measured CIELAB tradeoff; it is not an across-the-board forward improvement.

| Case | Accepted ns | Candidate ns | C/A | Samples A/C | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| forward-cielab | 2947605 | 3199324 | 1.0854 | 40/40 | pass |
| yliluoma-16 | 15631475 | 15668228 | 1.0024 | 33/33 | pass |
| perturb-zero-strength | 50320.5 | 51560 | 1.0246 | 40/40 | pass |
| tiny-resize-nearest-scalar | 50 | 50 | 1.0000 | 40/40 | pass |
| tiny-threshold-random | 20.5 | 20 | 0.9756 | 40/40 | pass |

### Family comparison against frozen spec

The range is frozen-spec median divided by selected-production median. Above one means selected production is faster.
Ranges cover ordinary cases within each family, not pooled aggregate speedups.
They exclude nine tiny controls and zero-strength perturb; every excluded row remains in the full table.

| Family | Spec / selected production range | Gate and scope caveat |
| --- | ---: | --- |
| Resize, exact cases | 1.1486–9.4401× | Five pass. The other five filters retain incorrect drift diagnostics. |
| Forward | 1.1586–3.8294× | Seven pass. CIELAB is still slower than previous production in its separate selection repeat. |
| Inverse | 0.9928–1.0102× | Seven pass; near parity. |
| Construction-inclusive source conversion | 0.0023–0.0616× | Seven regressions. This helper still constructs a converter for each read. |
| Wide reconstruction | 0.9917–1.0015× | Seven pass; near parity. |
| Placement grid | 0.0800–0.5615× | Seven regressions against spec despite improvements against previous production. Each standalone mask call still constructs a converter. |
| Metric score formulas | 0.9213–1.1593× | Three pass, four inconclusive. |
| Quantize, fifteen policies | 0.7636–1.4277× | Thirteen pass. Both cylindrical hue-arc policies regress. |
| Threshold fields | 0.9714–1.0209× | Six pass; near parity. |
| Perturb, nonzero ordinary cases | 0.7312–3.3308× | Five pass. Bayer2/sRGB still regresses against spec. |
| Diffusion | 0.9191–1.2190× | Eight pass. All use Oklab matching; byte/matching labels distinguish feedback. |
| Yliluoma | 0.2668–0.9429× | Sizes 2/4/8 regress; size16 passes. Target-converter candidate remains held. |

All five incorrect resize cases reproduce the exact earlier mismatch counts, first mismatches, and metadata differences.
They remain one-byte drift diagnostics with no tolerance change or acceptance inference.
The final spec-relative gate remains incorrect overall. Twenty-five timing regressions and five inconclusive cases remain visible.
Passing previous-production selection gates does not imply universal superiority to spec or release readiness.

### All selected spec/production medians

Accepted is frozen spec; candidate is selected production. Values retain the same nanosecond and sample-count conventions as earlier tables.

| Case | Accepted ns | Candidate ns | C/A | Samples A/C | Gate |
| --- | ---: | ---: | ---: | ---: | --- |
| resize-nearest-scalar | 37590.5 | 5680 | 0.1511 | 40/40 | pass |
| resize-area-scalar | 1145697 | 606664 | 0.5295 | 40/40 | incorrect |
| resize-bilinear-scalar | 2076712.5 | 891909.5 | 0.4295 | 40/40 | incorrect |
| resize-bicubic-catmull-rom | 1696870.5 | 830397 | 0.4894 | 40/40 | pass |
| resize-bicubic-catmull-rom-scale-aware | 9938085 | 1350501 | 0.1359 | 40/40 | incorrect |
| resize-lanczos2-fixed | 7057626.5 | 836367.5 | 0.1185 | 40/40 | pass |
| resize-lanczos2-scale-aware | 50545227.5 | 1402645.5 | 0.0278 | 10/40 | incorrect |
| resize-lanczos3-fixed | 14209440 | 1505223 | 0.1059 | 36/40 | pass |
| resize-lanczos3-scale-aware | 110506794 | 1983629.5 | 0.0180 | 10/40 | incorrect |
| resize-trilinear-mip-area | 4426167 | 3853514 | 0.8706 | 40/40 | pass |
| forward-srgb | 192483 | 166127.5 | 0.8631 | 40/40 | pass |
| inverse-srgb | 953880 | 946749.5 | 0.9925 | 40/40 | pass |
| source-construction-srgb | 32440 | 14305855.5 | 440.9943 | 40/36 | regression |
| reconstruct-srgb | 2773637 | 2774262 | 1.0002 | 40/40 | pass |
| placement-srgb | 1199808.5 | 15002187.5 | 12.5038 | 40/34 | regression |
| forward-linear-rgb | 2348965.5 | 881783.5 | 0.3754 | 40/40 | pass |
| inverse-linear-rgb | 3489658.5 | 3495678.5 | 1.0017 | 40/40 | pass |
| source-construction-linear-rgb | 284814 | 14643929 | 51.4158 | 40/35 | regression |
| reconstruct-linear-rgb | 5089071.5 | 5114186.5 | 1.0049 | 40/40 | pass |
| placement-linear-rgb | 4537259 | 14882384.5 | 3.2800 | 40/34 | regression |
| forward-oklab | 10045261.5 | 2623184.5 | 0.2611 | 40/40 | pass |
| inverse-oklab | 4227120 | 4221553.5 | 0.9987 | 40/40 | pass |
| source-construction-oklab | 657270.5 | 14625172 | 22.2514 | 40/35 | regression |
| reconstruct-oklab | 8417732 | 8458562 | 1.0049 | 40/40 | pass |
| placement-oklab | 7819223 | 16477699.5 | 2.1073 | 40/32 | regression |
| forward-cielab | 9988646.5 | 3179723 | 0.3183 | 40/40 | pass |
| inverse-cielab | 4320211 | 4333175.5 | 1.0030 | 40/40 | pass |
| source-construction-cielab | 616779 | 14707872 | 23.8463 | 40/35 | regression |
| reconstruct-cielab | 5388566.5 | 5390135.5 | 1.0003 | 40/40 | pass |
| placement-cielab | 7813198.5 | 16936360.5 | 2.1677 | 40/30 | regression |
| forward-ycbcr | 244064 | 147587.5 | 0.6047 | 40/40 | pass |
| inverse-ycbcr | 1233278.5 | 1242232.5 | 1.0073 | 40/40 | pass |
| source-construction-ycbcr | 59680.5 | 14685393 | 246.0669 | 40/35 | regression |
| reconstruct-ycbcr | 2867243 | 2862892.5 | 0.9985 | 40/40 | pass |
| placement-ycbcr | 1990720 | 15282241 | 7.6767 | 40/34 | regression |
| forward-oklch | 14897649.5 | 5625730 | 0.3776 | 34/40 | pass |
| inverse-oklch | 7006809.5 | 6936020 | 0.9899 | 40/40 | pass |
| source-construction-oklch | 916114 | 14873550.5 | 16.2355 | 40/34 | regression |
| reconstruct-oklch | 7686886.5 | 7751016 | 1.0083 | 40/40 | pass |
| placement-oklch | 10288195 | 18323107 | 1.7810 | 40/28 | regression |
| forward-cielch | 13976685.5 | 6278485 | 0.4492 | 36/40 | pass |
| inverse-cielch | 6689196.5 | 6659060 | 0.9955 | 40/40 | pass |
| source-construction-cielch | 880309 | 14895649.5 | 16.9209 | 40/34 | regression |
| reconstruct-cielch | 8245064.5 | 8242268.5 | 0.9997 | 40/40 | pass |
| placement-cielch | 9965740 | 18959506.5 | 1.9025 | 40/28 | regression |
| scores-euclidean | 336730 | 290455 | 0.8626 | 40/40 | inconclusive |
| scores-chord | 1010030.5 | 1096321.5 | 1.0854 | 40/40 | inconclusive |
| scores-arc | 651045 | 651999.5 | 1.0015 | 40/40 | pass |
| scores-compuphase | 1379111.5 | 1381331 | 1.0016 | 40/40 | pass |
| scores-rec601 | 307569 | 326695 | 1.0622 | 40/40 | inconclusive |
| scores-rec709 | 299814 | 302685 | 1.0096 | 40/40 | inconclusive |
| scores-ciede2000 | 14527384 | 14548165 | 1.0014 | 36/36 | pass |
| quantize-srgb-euclidean-palette32 | 888508.5 | 921669 | 1.0373 | 40/40 | pass |
| quantize-linear-rgb-euclidean-palette32 | 1286813.5 | 915014 | 0.7111 | 40/40 | pass |
| quantize-oklab-euclidean-palette32 | 1654474.5 | 1158852 | 0.7004 | 40/40 | pass |
| quantize-cielab-euclidean-palette32 | 1670465.5 | 1182293 | 0.7078 | 40/40 | pass |
| quantize-ycbcr-euclidean-palette32 | 982454.5 | 956533.5 | 0.9736 | 40/40 | pass |
| quantize-srgb-compuphase-palette32 | 3085641 | 3045180.5 | 0.9869 | 40/40 | pass |
| quantize-srgb-rec601-palette32 | 925268 | 985929.5 | 1.0656 | 40/40 | pass |
| quantize-srgb-rec709-palette32 | 915149 | 975874.5 | 1.0664 | 40/40 | pass |
| quantize-oklch-euclidean-palette32 | 1916404 | 1374571 | 0.7173 | 40/40 | pass |
| quantize-oklch-circular-hue-palette32 | 3556758.5 | 3562175 | 1.0015 | 40/40 | pass |
| quantize-oklch-hue-arc-palette32 | 2707780.5 | 3296480 | 1.2174 | 40/40 | regression |
| quantize-cielab-ciede2000-palette32 | 28825924.5 | 28294717.5 | 0.9816 | 18/18 | pass |
| quantize-cielch-euclidean-palette32 | 1900109 | 1422441.5 | 0.7486 | 40/40 | pass |
| quantize-cielch-circular-hue-palette32 | 3475201.5 | 3557968.5 | 1.0238 | 40/40 | pass |
| quantize-cielch-hue-arc-palette32 | 2581484 | 3380785.5 | 1.3096 | 40/40 | regression |
| threshold-bayer2 | 275679 | 283794 | 1.0294 | 40/40 | pass |
| perturb-bayer2-srgb | 198923 | 272040 | 1.3676 | 40/40 | regression |
| threshold-bayer4 | 377899.5 | 387646 | 1.0258 | 40/40 | pass |
| perturb-bayer4-linear-rgb | 839458 | 616329.5 | 0.7342 | 40/40 | pass |
| threshold-bayer8 | 487548.5 | 489906.5 | 1.0048 | 40/40 | pass |
| perturb-bayer8-oklab | 8381671 | 2516393 | 0.3002 | 40/40 | pass |
| threshold-bayer16 | 593209 | 598754.5 | 1.0093 | 40/40 | pass |
| perturb-bayer16-cielab | 1329465 | 860508.5 | 0.6473 | 40/40 | pass |
| threshold-random | 210388.5 | 215088 | 1.0223 | 40/40 | pass |
| perturb-random-oklch | 10358835.5 | 3908744.5 | 0.3773 | 40/40 | pass |
| threshold-blue | 96741.5 | 94761 | 0.9795 | 40/40 | pass |
| perturb-blue-cielch | 1765291 | 1302448.5 | 0.7378 | 40/40 | pass |
| diffusion-floyd-steinberg-srgb-bytes | 2039005.5 | 1755301.5 | 0.8609 | 40/40 | pass |
| diffusion-floyd-steinberg-matching | 1910398.5 | 1824122.5 | 0.9548 | 40/40 | pass |
| diffusion-sierra-srgb-bytes | 2191007.5 | 2182117.5 | 0.9959 | 40/40 | pass |
| diffusion-sierra-matching | 2057206.5 | 2238344.5 | 1.0881 | 40/40 | pass |
| diffusion-sierra-lite-srgb-bytes | 2046027 | 1678450.5 | 0.8203 | 40/40 | pass |
| diffusion-sierra-lite-matching | 1916904 | 1746141.5 | 0.9109 | 40/40 | pass |
| diffusion-atkinson-srgb-bytes | 2089356 | 1912888.5 | 0.9155 | 40/40 | pass |
| diffusion-atkinson-matching | 1945779 | 1961405 | 1.0080 | 40/40 | pass |
| yliluoma-2 | 334685 | 1254624.5 | 3.7487 | 40/40 | regression |
| yliluoma-4 | 1061461.5 | 1952614 | 1.8396 | 40/40 | regression |
| yliluoma-8 | 3839018 | 4760711 | 1.2401 | 40/40 | regression |
| yliluoma-16 | 14937475.5 | 15842779 | 1.0606 | 34/32 | pass |
| perturb-zero-strength | 31255 | 51621 | 1.6516 | 40/40 | regression |
| tiny-resize-nearest-scalar | 40 | 50 | 1.2500 | 40/40 | regression |
| tiny-forward-srgb | 29 | 20.5 | 0.7069 | 40/40 | inconclusive |
| tiny-inverse-srgb | 31 | 31 | 1.0000 | 40/40 | pass |
| tiny-scores-euclidean | 20 | 20 | 1.0000 | 40/40 | pass |
| tiny-quantize-srgb-euclidean-palette32 | 520 | 1570 | 3.0192 | 40/40 | regression |
| tiny-threshold-random | 20 | 21 | 1.0500 | 40/40 | pass |
| tiny-diffusion-floyd-steinberg-srgb-bytes | 2520 | 2125 | 0.8433 | 40/40 | pass |
| tiny-yliluoma-2 | 625 | 2981 | 4.7696 | 40/40 | regression |
| tiny-perturb-bayer4-linear-rgb | 80 | 1209.5 | 15.1188 | 40/40 | regression |

### Final evidence hashes and audit command

```sh
node .plans/83-scalar-report-audit.mjs \\
  ../v1-scalar-spec-bench/target/scalar-comparison/packed-selection-results \\
  ../v1-scalar-spec-bench/target/scalar-comparison/packed-selection-repeat-results \\
  ../v1-scalar-spec-bench/target/scalar-comparison/spec-prod-selected-results
```

### packed-selection-results

```text
report.json      6bdf7721cb2b9f621cc2edd8bdefcf7342ef015c4eb569fb21f9dd8247fa701b
prepared.json    c6b9d15f2c2f41672df0b577f2534cd5d09557acea67d94972db4fabcb0a9155
events.jsonl     f5839c47b6e402acb7bbd4bf4fb0157d785cd377fefb534bfff96459bfef0463
worker inventory 38313880136f31a05e1d3d56f8ce523220a9760f6a6ebd2f5173eb2ca14bbab4
```

### packed-selection-repeat-results

```text
report.json      15f701515106632296c433a4845470b885bf5a47d4b7ff5dcc0548a8b6b52261
prepared.json    7083de2c10579cb311ce74fde9afc72f3d0195132a4f46ef525ffb0c26daeee6
events.jsonl     5089f1a17137517f658aea80bd74fe43346b27018488650d0bbe8074114b9989
worker inventory 523b7f970122d2fb4d3746b7ddeaaab41d1b527c99931857974601a7ba7c8667
```

### spec-prod-selected-results

```text
report.json      53ce1d217dadf32b301d5c367bd4521e007ce3c667b78402eeaff7c4fe9336d2
prepared.json    7d63394b669e14851100c806d5123edb381d1cd94987918bc0210e7b8c2ea3af
events.jsonl     354614a5073f8170f6e2e68209bcb76ad189ffbfd3ddf653d692315004d4a908
worker inventory e729825f5d1b75b904f2932edb181eec2cde02d71a7a9c08cb5061ed7ed19569
```

The reporting audit reads and hashes all final reports, prepared descriptors, journals, and raw worker results.
The coordinator records `benchmark-results/scalar-spec-prod-2026-09-09/selected-production.tar.gz` in the benchmark worktree.
Its SHA-256 is `0b1069920357bcf8daa0d53afc8d0784a314f4dcaf19e81cce88d7f2e91c0090`.
It retains all three final result directories, selection plans/prepared copies, and candidate native provenance/binaries.
The coordinator reports a 554 MiB archive and two successful byte-for-byte comparisons, both exit zero.
Plans/prepared/results match their original benchmark target; the native directory matches the selected candidate target.
Remote evidence branches retain measured commits at `evidence/v1-scalar-selected-8e09c05d` and `evidence/v1-scalar-held-086bbd47`.
Delivery rebases may change commit IDs; production bytes must match the recorded measured source.
Root owns the final archives, artifact-provenance retention, PRs, release-map updates, and cleanup.
No additional optimization or experiment is proposed by this report.
This reporting pass changes only this document. It runs no builds, tests, or benchmarks and leaves no owned jobs.
