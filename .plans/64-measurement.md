# S23 shared mip measurements

## Selected result

Keep exact shared-mip implementation `07d528a6` after one bounded paired trial.
All 80 workers exit successfully. All native and browser cases pass exact frozen and cross-artifact checks.
The trial retains 1,344 samples, with at most one live benchmark worker throughout.
Fractional downscale latency falls 27% to 56%, depending on shape and runtime.
Integer-LOD and enlargement controls remain within the 10% regression limit.
No approximation, second tuning run, release, or rollout is selected.

## Revisions and retained artifacts

Prepared baseline `0be73eb64d9baced6a32ff19204dd50cddd7aba3` retains the literal missing-implementation baseline `fba85a94`.
Accepted artifact source `2c9bec9069e335d0696e506a1e0b603aced17ea2` adds benchmark-only registration to that prepared baseline.
Candidate source `180ca47aeee5ab418df05ca578e580d4e0295436` includes the exact shared-mip implementation and identical benchmark registration.
Both production/package inputs remain unchanged during artifact preparation and measurements.

Accepted files are under `.worktrees/v1-s23-trilinear/target/`:

- `s23-accepted-native-01/ditherette-bench`, SHA-256 `2e2e3ac0c63f5be9d54bdf550996c0a0e253f32489fa1f75e2644dcd7df9ac48`.
- `s23-accepted-native-01/ditherette-bench-pair`, SHA-256 `4bda2832f54570f9df3e47457d2cd3281b132041720dc04aed1737d7aac3a935`.
- `s23-accepted-public-01/ditherette.tgz`, SHA-256 `2b6bc4aafce650d021d86249e7d8ffbef6a599b28d3e25d30d92c96b23de1021`.

Candidate files are under `.worktrees/v1-s23-reuse/target/`:

- `s23-candidate-native-01/ditherette-bench`, SHA-256 `5ad4a0818ab69751dd64699ff954d03df50a08fc22f80c34586a500ae43b1e87`.
- `s23-candidate-native-01/ditherette-bench-pair`, SHA-256 `c39849786857661bb16f6022a86df405b4062ce6539af2b0e9200cb6648d5e52`.
- `s23-candidate-public-01/ditherette.tgz`, SHA-256 `c256bc8ebabbc568313707c70e24013f3d228b3b43c2763143d6451c7bbc2b58`.

`s23-trial-01` retains four complete reports, every request/result, process events, and immutable executable/browser snapshots.
Package build provenance and source inventories accompany both public bundles.
Quiet clearance at 2026-09-07 21:20:56 UTC records all agents idle, no owned process, and host load 0.82/1.84/1.69.
Unrelated host activity was observed, not stopped. The post-run audit found only its own short-lived inspection process.
Every launched worker has a matching reaped event before implementation resumes.

## Scope and budget

Two alternating AB/BA pairs use at most 20 samples, 50 ms warmup, 250 ms measurement, and a 2 ms throughput target.
Five cases run in each environment. Native timing includes plan/scratch allocation, initialization, execution, and temporary destruction.
Its caller owns output allocation. Browser timing measures the complete package call through the installed tarball.
Both browser roles are real package artifacts. There is no website trilinear implementation or TypeScript speedup claim.
Application caches are absent; browser instances are primed. No non-exact timing override is enabled.

Node 24.19.0 and Playwright 1.59.1 use Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Each browser is headless and isolated, with no extra launch arguments. WebKit aliases retain hardlink identity.

| Environment | Workers launched/reaped | Samples | Gate |
| --- | ---: | ---: | --- |
| Native | 20 | 400 | Pass |
| Chromium | 20 | 400 | Pass |
| Firefox | 20 | 144 | Pass |
| WebKit | 20 | 400 | Pass |

Firefox reaches the fixed measurement cap before collecting 20 samples per worker. Raw samples remain unchanged.

## Medians

Cells show accepted milliseconds / candidate milliseconds. Throughput entries show per-call cost.

| Case | Native | Chromium | Firefox | WebKit |
| --- | ---: | ---: | ---: | ---: |
| 512x384 to 173x129 | 5.300743 / 3.877162 | 9.055 / 6.000 | 70.52 / 51.19 | 8.25 / 5.53 |
| 512x384 to 53x41 latency | 3.989399 / 2.087991 | 6.9075 / 3.055 | 52.73 / 27.21 | 6.16 / 2.72 |
| 512x384 to 53x41 throughput | 3.940687 / 2.087271 | 6.910 / 3.045 | 52.03 / 27.23 | 6.06 / 2.71 |
| 512x384 to 64x48 integer LOD | 1.951114 / 1.972205 | 3.410 / 2.795 | 25.46 / 25.30 | 3.04 / 2.44 |
| 128x96 to 389x291 enlargement | 3.950604 / 3.951823 | 7.145 / 7.175 | 48.49 / 47.93 | 6.36 / 6.42 |

The shared chain retains each rounded mip once. It changes neither filter arithmetic nor intermediate storage conversions.
Existing allocation fixtures confirm the lower retained capacity and account for all live buffers before source import.
All restored area/bilinear/shared helpers remain byte-identical. Independent read-only review found no candidate defect.
S41 still owns complete-call Firefox costs and final release measurements; this slice does not establish release readiness.
