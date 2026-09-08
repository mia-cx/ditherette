# S26 converter reuse measurement

## Decision

Retain accepted baseline `3915f60519995cb9087a18b3bfd6bd7220ae804a`.
Candidate `b237b7468fa5fc349760bc0086bd1748113b6d82` remains separate and unselected.
The candidate exceeds the 20% complete-call target, but required verification and noise gates do not all pass.
No retry or further candidate revision runs in this slice.

## Run and artifacts

The single comparison runs from `2026-09-08T06:26:02.855Z` to `2026-09-08T06:35:02.623Z`.
All 208 workers start and are reaped, with maximum live worker count one and 4,048 timing samples.
All implementation agents, builds, and tests drain before measurement.
The final scoped process audit at `2026-09-08T06:35:45.903Z` finds no project benchmark, browser, build, or test children.

Artifacts remain in `.worktrees/v1-s26-integration/target/s26-trial-01/`.
`artifact-audit.json`, `quiet-clearance.json`, and `completion-audit.json` record identities and process accounting.
Each runtime retains immutable prepared files, requests, raw results, stderr, events, and its original report.
Fresh native/public preparation cleans only local crate outputs and verifies revision-bound artifacts before timing.
Native preparation verifies both complete executable digests and embedded clean revisions.
Both public roles rebuild scalar/threaded Wasm, pack, install, and snapshot actual package/browser inputs.

Rust 1.97.0, LLVM 22.1.6, Node 24.19.0, Playwright 1.59.1, pnpm 11.13.0.
Browsers are Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Machine is AMD Ryzen 9 7950X, 24 logical CPUs exposed, Linux 6.12.95+deb13-amd64.
Quiet-clearance load averages are 0.78, 1.42, and 0.88. Unrelated host work is not stopped.

## Gate limitations

Five native threshold controls receive `incorrect` because the verifier demands an explicit working-space identity.
These are palette-independent scalar thresholds. Their reports say `incomplete`; they do not report a numeric mismatch.
Preserve that defect and its original reports. A separate verifier repair must establish their output verification.
Every other native case passes. All browser output checks are exact.
Native verification checks deterministic callable endpoints; browser verification checks every measured and warmup output outside timing.

Chromium marks the Bayer4/Oklab/CompuPhase composition inconclusive.
Firefox marks sRGB/Bayer2 perturbation inconclusive.
WebKit marks sRGB/Bayer2 and YCbCr/Bayer16 perturbation inconclusive.
These are pair-noise failures, not confirmed regressions. They still prevent selecting the candidate under the declared gate.
The original gate remains unchanged. Strong median gains do not silently override incomplete evidence.

## Median complete-call and component results

Times below are milliseconds for one 64×48 image or declared component batch.
Lower ratios are faster. All controls retain their declared construction/allocation timing scope.

### native

100 workers; 2000 samples; original aggregate gate `incorrect`.

| Case | Accepted ms | Candidate ms | Ratio | Gate |
| --- | ---: | ---: | ---: | --- |
| srgb-inverse-f32-image | 0.014594 | 0.014680 | 1.005858 | pass |
| linear-rgb-inverse-f32-image | 0.053840 | 0.054196 | 1.006603 | pass |
| oklab-inverse-f32-image | 0.063970 | 0.064256 | 1.004471 | pass |
| oklch-inverse-f32-image | 0.103736 | 0.103842 | 1.001017 | pass |
| cielab-inverse-f32-image | 0.066711 | 0.067186 | 1.007120 | pass |
| cielch-inverse-f32-image | 0.122422 | 0.121947 | 0.996120 | pass |
| ycbcr-inverse-f32-image | 0.019040 | 0.019085 | 1.002363 | pass |
| bayer2-threshold-grid | 0.003200 | 0.003200 | 1.000000 | incorrect |
| bayer4-threshold-grid | 0.004630 | 0.004630 | 1.000000 | incorrect |
| bayer8-threshold-grid | 0.006090 | 0.006040 | 0.991872 | incorrect |
| bayer16-threshold-grid | 0.007910 | 0.007860 | 0.993742 | incorrect |
| random-threshold-grid | 0.003200 | 0.003204 | 1.001406 | incorrect |
| oklab-adaptive1-mask | 33.395480 | 4.163352 | 0.124668 | pass |
| oklch-adaptive2-mask | 33.678988 | 4.594669 | 0.136425 | pass |
| srgb-source-construction-inclusive | 3.615272 | 3.629888 | 1.004043 | pass |
| oklab-source-construction-inclusive | 3.669599 | 3.685839 | 1.004425 | pass |
| perturb-srgb-bayer2-everywhere | 3.689529 | 0.049630 | 0.013452 | pass |
| perturb-linear-rgb-random-everywhere | 3.773010 | 0.112072 | 0.029703 | pass |
| perturb-oklab-bayer4-adaptive1 | 34.969369 | 0.629885 | 0.018012 | pass |
| perturb-oklch-random-adaptive2 | 34.162262 | 0.996735 | 0.029176 | pass |
| perturb-cielab-bayer8-everywhere | 3.868902 | 0.210763 | 0.054476 | pass |
| perturb-cielch-random-adaptive1 | 38.162646 | 1.431832 | 0.037519 | pass |
| perturb-ycbcr-bayer16-everywhere | 3.741685 | 0.058846 | 0.015727 | pass |
| separable-bayer4-oklab-everywhere-srgb-compuphase-palette64 | 5.423192 | 1.761998 | 0.324901 | pass |
| separable-random-ycbcr-adaptive2-oklch-hue-arc-palette64 | 37.969558 | 1.835044 | 0.048329 | pass |

### chromium

36 workers; 720 samples; original aggregate gate `inconclusive`.

| Case | Accepted ms | Candidate ms | Ratio | Gate |
| --- | ---: | ---: | ---: | --- |
| perturb-srgb-bayer2-everywhere | 21.117500 | 0.105000 | 0.004972 | pass |
| perturb-linear-rgb-random-everywhere | 21.480000 | 0.380000 | 0.017691 | pass |
| perturb-oklab-bayer4-adaptive1 | 202.032500 | 1.255000 | 0.006212 | pass |
| perturb-oklch-random-adaptive2 | 193.980000 | 1.707500 | 0.008802 | pass |
| perturb-cielab-bayer8-everywhere | 21.907500 | 0.527500 | 0.024079 | pass |
| perturb-cielch-random-adaptive1 | 216.027500 | 2.405000 | 0.011133 | pass |
| perturb-ycbcr-bayer16-everywhere | 21.595000 | 0.125000 | 0.005788 | pass |
| separable-bayer4-oklab-everywhere-srgb-compuphase-palette64 | 23.022500 | 1.577500 | 0.068520 | inconclusive |
| separable-random-ycbcr-adaptive2-oklch-hue-arc-palette64 | 214.550000 | 2.165000 | 0.010091 | pass |

### firefox

36 workers; 608 samples; original aggregate gate `inconclusive`.

| Case | Accepted ms | Candidate ms | Ratio | Gate |
| --- | ---: | ---: | ---: | --- |
| perturb-srgb-bayer2-everywhere | 190.480000 | 0.620000 | 0.003255 | inconclusive |
| perturb-linear-rgb-random-everywhere | 191.790000 | 2.720000 | 0.014182 | pass |
| perturb-oklab-bayer4-adaptive1 | 1800.840000 | 10.390000 | 0.005770 | pass |
| perturb-oklch-random-adaptive2 | 1734.200000 | 13.200000 | 0.007612 | pass |
| perturb-cielab-bayer8-everywhere | 192.610000 | 3.770000 | 0.019573 | pass |
| perturb-cielch-random-adaptive1 | 1917.000000 | 16.380000 | 0.008545 | pass |
| perturb-ycbcr-bayer16-everywhere | 190.180000 | 0.760000 | 0.003996 | pass |
| separable-bayer4-oklab-everywhere-srgb-compuphase-palette64 | 201.220000 | 12.350000 | 0.061376 | pass |
| separable-random-ycbcr-adaptive2-oklch-hue-arc-palette64 | 1907.850000 | 16.740000 | 0.008774 | pass |

### webkit

36 workers; 720 samples; original aggregate gate `inconclusive`.

| Case | Accepted ms | Candidate ms | Ratio | Gate |
| --- | ---: | ---: | ---: | --- |
| perturb-srgb-bayer2-everywhere | 20.330000 | 0.100000 | 0.004919 | inconclusive |
| perturb-linear-rgb-random-everywhere | 20.740000 | 0.400000 | 0.019286 | pass |
| perturb-oklab-bayer4-adaptive1 | 192.720000 | 1.280000 | 0.006642 | pass |
| perturb-oklch-random-adaptive2 | 187.810000 | 1.850000 | 0.009850 | pass |
| perturb-cielab-bayer8-everywhere | 21.580000 | 0.500000 | 0.023170 | pass |
| perturb-cielch-random-adaptive1 | 204.940000 | 2.480000 | 0.012101 | pass |
| perturb-ycbcr-bayer16-everywhere | 20.370000 | 0.140000 | 0.006873 | inconclusive |
| separable-bayer4-oklab-everywhere-srgb-compuphase-palette64 | 22.030000 | 1.740000 | 0.078983 | pass |
| separable-random-ycbcr-adaptive2-oklch-hue-arc-palette64 | 205.710000 | 2.260000 | 0.010986 | pass |

## Distribution sizes

| Role | Artifact | Raw bytes | Gzip-9 bytes | SHA-256 |
| --- | --- | ---: | ---: | --- |
| accepted | scalar Wasm | 239542 | 109025 | `dcb020ebfb151375f1bd945d3fe4388f05497dd8200644bdf4cd9f82814b22c4` |
| accepted | threads Wasm | 327180 | 138775 | `abd075dca9c93aca8a3b87407f66a8afa371c6544030d932e36e790710ebb7cf` |
| accepted | tarball | 276580 | already compressed | `aa7befbef890f47f83831a7f9f735a9d1a9c5067895620cd36d60f369eab29f2` |
| candidate | scalar Wasm | 239754 | 109049 | `0a42e6803633d78ed182234e91efebc18c7c3fcfffab290ebf8e56db9f2bba86` |
| candidate | threads Wasm | 327051 | 138718 | `9be7d30161c3ae13504bfa3dd0710f489a07ade6f933ca7e8af9b2f78c593468` |
| candidate | tarball | 276609 | already compressed | `4f0526068a79915839658fae8a3f6dc149f6ccb01c8d5a07b5969e0121d92134` |

Accepted scalar grows 7.76% raw and 7.26% gzip from the retained S25 baseline while adding seven inverse spaces and field processing.
Accepted threaded Wasm grows 5.07% raw and 5.47% gzip from S25.
Candidate scalar differs by 212 raw bytes and 24 gzip bytes from this baseline; it remains unselected.

## Conformance and delivery

The literal implementation checkpoint is `e156cfbfe0d3dfb598e2f5746bca1ddc46a5f69a`.
The bounded public baseline is `089251287e387cb575e22e8993d8989a371a089d`.
Native/private/interface checks, both builds, and three-engine installed-package checks pass for the baseline and candidate.
Each package engine checks 91 frozen field vectors and 1,365 exact compositions.
The actual benchmark adapter also passes 47 quantize fixtures, 12 field fixtures, and 10 actual compositions per engine.
Frozen content remains 106 files with digest `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.

The delivery rebase onto S25 `3a9db011207a44f230ae519edc93c900a747c021` produces `482b4c0816d123ef9fb118cb32c4f18cfb3766fa`.
It preserves every runtime, benchmark, package, frozen, and shared image byte from measured accepted `3915f605`.
The sole conflict preserves already-completed S25 evidence. Two merge-only S26 plan files are restored unchanged.
No PR is merged, no package published, and no release or rollout activated.
