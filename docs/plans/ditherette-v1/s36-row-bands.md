# S36 measured quantize and field row bands

Issue #78. This report covers S36 only; S37 mixing implementation stays in its child branch.

## Artifact and protocol

Root measured complete public calls in isolated HostWorkers using the same threaded developer package for both roles.
Accepted forces scalar. Candidate requests row bands before method timers.
This is not ordinary scalar-package versus threaded-package evidence.
Every call includes input and output copies, hashing, preparation, dispatch, and result ownership.

The fixed matrix uses two alternating pairs, 20 requested samples, 50 ms warmup,
and a 10-second nominal per-worker measurement budget with the existing minimum-five collection rule.
S36 contributes 22 cells, 88 workers, and 44 actual scalar/row pairs per engine.
Every retained S36 pair matches bytes and public metadata, including warnings.
Each engine also records 44 separate reference probes. These are not additional actual pairs.
No S36 frozen-reference mismatch, retained-output instability, or cross-engine mismatch appeared.

Both complete engine journals record 200 started and reaped workers, maximum live one, and correct alternating order.
The S36 subset observed required threading, cross-origin isolation, pool eight, and the requested stage policies.
Progress was unrequested; these timings supply no callback telemetry.
Native tests independently cover caller-thread callbacks and transactional failures.

- Source revision: `5d16c5682f354fecd75ca7f761802d9e2ea75ab5`, clean.
- Package asset digest: `68cbc40b30810d6491435ea9e8989850f7977ae1030c0d296f19bf9243eba62d`.
- Compiler: rustc 1.97.0 (2d8144b78 2026-07-07); Node v24.19.0; Playwright 1.59.1.
- Chromium 147.0.7727.15; runtime digest `2de3e25c063d9da5a8b997a848f1a8d9ebf37969f8b504d0cd1327117667670a`.
- Firefox 148.0.2; runtime digest `3b3773b8fdf636054c3994b34670c6caff4cebf10f2800ca02ce2e2fd6367cd4`.

Retained evidence is under `.worktrees/v1-s35-37-bench/target`:

- `rows-trial-02/chromium-results/report.json`, SHA-256 `160255130439e20dbbcd46a224f4911bed8fc5c10cc3840f46627a09e2148642`.
- `rows-trial-03/firefox-results/report.json`, SHA-256 `77af173eb9b0c2203d8ad43a75181a68e90d57cf2a8388c1e7b1784603594e73`.
- `rows-trial-03/combined-analysis.json` contains exact per-pair medians, sample counts, identities, and gates.

## Selection

At least 769 by 513 pixels selects sRGB random/everywhere perturbation and the corresponding separable class.
Direct quantization and separable matching require sRGB Euclidean, preserve alpha, 15 visible entries, and trailing Transparent.
Pools of at least four select four workers and 128-row bands.
Pools of two or three select the measured two-worker, 32-row alternative.

At least 65 by 49 pixels selects Oklab blue-noise/adaptive-radius-two perturbation with two workers and 32-row bands.
Its separable class additionally requires Oklab Euclidean, preserve alpha, 63 visible entries, and trailing Transparent.

These are conservative cost-class heuristics using representative measurements, not predictions for every image or parameter.
Nonzero strength, seed, palette RGB values, alpha threshold, and adaptive mask values do not define separate structural loop classes.
Zero strength, small dimensions, other recipe classes, and one-worker pools stay scalar.
Direct Oklab quantization remains scalar because Chromium is inconclusive.
The four-worker Oklab control has one band and shows no useful gain.
Unmeasured Process field scheduling stays scalar; developer overrides still test that composition.
Diffusion retains scalar scan order.

## Complete-call medians

Values are scalar / rows in milliseconds. Gates are Chromium / Firefox.
Small means 33 by 25. Large means sRGB 769 by 513 or Oklab 65 by 49.
Two means two workers / 32 rows; four means four workers / 128 rows.
Warm cases use final-cache-hit controls and are overhead checks, never kernel-speedup claims.

| Case | Chromium ms | Firefox ms | Gates |
| --- | ---: | ---: | --- |
| quantize-sRGB-small-two | 0.275 / 0.2825 | 0.99 / 1.04 | pass / inconclusive |
| quantize-sRGB-large-two | 42.2625 / 29.1 | 268.37 / 175.18 | pass / pass |
| perturb-sRGB-small-two | 6.2275 / 6.2825 | 51.95 / 52.07 | pass / pass |
| perturb-sRGB-large-two | 2870.73 / 1625.1625 | 24515.88 / 13863.73 | pass / pass |
| separable-sRGB-small-two | 6.385 / 6.47 | 53.41 / 52.73 | pass / pass |
| separable-sRGB-large-two | 2889.55 / 1639.635 | 24728.74 / 13993.51 | pass / pass |
| quantize-Oklab-small-two | 0.5725 / 0.565 | 2.6 / 2.68 | inconclusive / pass |
| quantize-Oklab-large-two | 1.4 / 1.1275 | 8.41 / 5.9 | inconclusive / pass |
| perturb-Oklab-small-two | 60.33 / 60.3575 | 514.34 / 515.65 | pass / pass |
| perturb-Oklab-large-two | 232.005 / 150.9475 | 1977.72 / 1299.11 | pass / pass |
| separable-Oklab-small-two | 60.7825 / 60.665 | 515.79 / 516.01 | pass / pass |
| separable-Oklab-large-two | 234.03 / 152.255 | 1999.04 / 1298.65 | pass / pass |
| quantize-sRGB-large-four | 42.875 / 26.8425 | 265.96 / 162.66 | pass / pass |
| perturb-sRGB-large-four | 2863.31 / 1449.9925 | 24576.84 / 12364.43 | pass / pass |
| separable-sRGB-large-four | 2896.195 / 1462.7 | 24705.4 / 12477.25 | pass / pass |
| separable-Oklab-large-four | 231.6025 / 231.7875 | 1985.86 / 1983.33 | pass / pass |
| quantize-sRGB-large-warm | 8.6275 / 8.7675 | 48.48 / 49.64 | pass / pass |
| perturb-sRGB-large-warm | 9.105 / 9.0675 | 50.03 / 49.34 | pass / pass |
| separable-sRGB-large-warm | 8.525 / 8.55 | 49.29 / 48.45 | pass / pass |
| quantize-Oklab-large-warm | 0.165 / 0.17 | 0.72 / 0.74 | inconclusive / pass |
| perturb-Oklab-large-warm | 0.125 / 0.125 | 0.62 / 0.6 | inconclusive / pass |
| separable-Oklab-large-warm | 0.21 / 0.2125 | 0.88 / 0.88 | pass / pass |

Chromium has 18 passing cells and four inconclusive cells.
Firefox has 21 passing cells and one inconclusive cell.
A passing gate alone does not establish a gain; the small and one-band controls remain scalar.
All noisy controls stay in the evidence. No retry seeks a desired result.

## Actual sample counts

Chromium collected 1536/1,760 S36 samples.
Firefox collected 1359/1,760.
All unlisted cells collect 20/20 samples in both pairs.
Below, each entry is scalar/rows; entries separate pair zero and pair one.

| Time-capped cell | Chromium pairs | Firefox pairs |
| --- | --- | --- |
| perturb-sRGB-large-two | 5/7, 5/7 | 5/5, 5/5 |
| separable-sRGB-large-two | 5/7, 5/7 | 5/5, 5/5 |
| perturb-Oklab-large-two | 20/20, 20/20 | 6/8, 6/8 |
| separable-Oklab-large-two | 20/20, 20/20 | 5/8, 6/8 |
| perturb-sRGB-large-four | 5/7, 5/7 | 5/5, 5/5 |
| separable-sRGB-large-four | 5/7, 5/7 | 5/5, 5/5 |
| separable-Oklab-large-four | 20/20, 20/20 | 6/6, 6/6 |

Time-capped passing gains remain valid under the predeclared minimum-five rule.
Requested 20 is not an added acceptance requirement.
Sample limits remain explicit; genuinely noisy gates do not select automatic execution.
Timer resolution is about five microseconds in Chromium and 20 microseconds in Firefox.
Warm and small inconclusive cells remain controls, not acceleration evidence.

## Publication boundary

The S36-only publication branch starts at `16136471`, including selector `1513153d`
and final S35 parent `a8418904`. It adds developer-getter cleanup `ddcd7477`.
The combined branch remains unchanged at `5933ae55`; none of its S37 files or callsite enters this branch.

Ordinary-package browser validation completes against combined source `dc81818a`, as recorded below.
No new Wasm build, browser run, measurement, push, or PR operation occurs in this separation task.

Fresh native validation on this exact S36-only branch passes 44 scalar library tests and 45 threaded library tests.
Each configuration also passes nine focused integration tests.
Both commands use `CARGO_TARGET_DIR=target/compiler` within the publication worktree:

```sh
cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --release --lib --test prod_quantize_row_bands --test prod_field_row_bands --test prod_field_band_allocation --test prod_processor_fields
cargo test --manifest-path crates/ditherette-wasm/Cargo.toml --release --features threads --lib --test prod_quantize_row_bands --test prod_field_row_bands --test prod_field_band_allocation --test prod_processor_fields
```

All owned native jobs exited. The returned old delivery compiler cache was removed after checking jobs, symlinks, and contents.
That reclaimed 171 MiB of rebuildable files. Source, joined ancestry, and every trial artifact remain intact.

## Final installed-package disposition

The publication branch joins S35 PR 126 head `6bbe113b99a08f2a11ade6296ddf7f25e32e041b` and rebases onto its published branch.
Its diff remains S36-only. Field/quantize production and shared build inputs match the combined tested source `dc81818a`.
The pure S36 native checks above remain applicable because delivery changes only ancestry and evidence documents.

The ordinary tarball SHA-256 is `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d`.
The trusted frozen guard passes. The untimed automatic host suite passes 23 checks, including nine cases per engine.
It verifies ordinary scalar/required-threaded exact bytes and metadata, Oklab quantize(perturb) composition, progress, and worker disposal.
Both ordinary binaries exclude the developer policy export. Exact invocation and cleanup are in `.plans/77-79-auto-host.md`.

S41 retains noisy small/warm controls, unmeasured classes, automatic-selector overhead follow-up, and WebKit threaded cleanup.
No new performance measurement or change to frozen expectations accompanies this delivery.
