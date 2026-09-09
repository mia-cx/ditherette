# S41 release status

Release is blocked. The production implementation and public package stay unchanged.
The exact SHA batching candidate provides no demonstrated win and remains unselected.

| Evidence | Completed coverage | Remaining gate |
| --- | --- | --- |
| [Candidate selection](83-selection.md) | 32 workers, 640 samples | No useful win; WebKit resize inconclusive |
| [Fresh historical anchors](83-anchors.md) | 32 cells, 128 workers, 2,560 samples | Seven cold regressions, four inconclusive cells |
| [Scalar release matrix](83-matrix.md#release-baseline) | 24 of 45 cells | 21 incomplete cells after disk exhaustion; inherited bilinear drift |
| [Actual website TypeScript](83-matrix.md#actual-website-typescript) | Nine cells, 36 workers, 720 samples | Nine confirmed regressions; equivalence limited to admitted fixtures |
| [Initialization](83-matrix.md#initialization) | Ten cells, 40 workers, 800 samples | Three scalar compiled controls inconclusive; threaded WebKit excluded |
| [Automatic execution continuation](83-automatic-continuation.md) | Combined 20 cells, 80 workers, 1,318 samples | Two regressions, two inherited bilinear mismatches, one inconclusive cell |
| [Maximum output](83-capped-probe.md) | Untimed resource attempt only | Transport string limit; no successful capped measurement |

All completed actual production-role comparisons agree exactly. That does not waive differences from the frozen reference.
The automatic Firefox warm nearest and warm mixing controls regress. Their thread/scalar ratios are 1.227 and 1.288.
Chromium warm mixing remains inconclusive. No new automatic threshold is selected from these results.

The [fixed matrix summary](83-matrix.json), [historical summary](83-anchors.json), and
[continuation summary](83-automatic-continuation.json) retain per-call samples, pair gates, identities, and serial event evidence.
Partial attempts remain separate. Raw image payloads and rebuildable snapshots are removed during finished-worktree cleanup.

## Reproduction and holds

Use the committed `release_integration_plan` example with lane names `release`, `automatic`,
`typescript`, `initialization`, `initialization-threads`, `anchors`, or `anchors-native`.
Its `inventory` mode lists the fixed case indices without large image arrays.
Build ordinary artifacts in clean worktrees using `prepare-native-benchmark.mjs` and
`prepare-public-benchmark.mjs`. Follow [PAIRED.md](../crates/ditherette-bench/PAIRED.md)
and [EXECUTION.md](../crates/ditherette-bench/EXECUTION.md) for preparation, fresh role identities, and exclusive execution.

The missing scalar release cells are Firefox 9–14 and WebKit 0–14.
Repeat both alternating pairs for interrupted Firefox 9; its partial records cannot complete a new comparison.
Capped output requires a transport repair and successful untimed feasibility check before any measurement.
Do not substitute a smaller output or increase limits globally.

Confirmed performance regressions require new bounded exact optimization work and fresh comparisons.
Noise requires fresh paired evidence within a declared budget, not relabeling a pooled median as passing.
Frozen drift requires Mia's visual decision or an exact production change. Frozen references stay unchanged.
Threaded WebKit requires demonstrated worker cleanup. The current successful scalar coverage does not certify it.

S43 must carry these release blockers alongside S42's size review and publisher setup holds.
All implementation PRs remain unmerged. Publishing, deployment, default activation, and retirement activation remain held.
