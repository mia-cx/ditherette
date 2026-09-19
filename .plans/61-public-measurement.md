# S20 public browser measurements

Historical evidence for the artifact identified below. [The restoration](108-restore-landed.md) changes its nearest implementation.
The diagnostic proposal below is held; fresh release measurements must use restored production.

Trial 02 completes the fixed initial matrix in Chromium, Firefox, and WebKit.
Every output comparison is byte-exact against the frozen reference, including metadata.
All three performance reports return `regression`. This is benchmark-tooling evidence, not release-performance acceptance.

## Scope and artifacts

Read [the fixed trial budget](61-initial-trial-budget.md) for fixture dimensions, preparation, and sample limits.
Each latency sample times one synchronous public call. Throughput measures calibrated batches separately.
Package timing includes request validation, allocations, source copying, the canonical nearest kernel, and durable output copying.
Preparation, transport JSON, correctness verification, and disposal remain outside processing timers.
S19 has no application cache or content hashing. Fresh and primed instances do not mean cold and warm application caches.

Both roles are independently built, packed, and installed from clean revision `e84a55eddb0014f97b64446408bfb5f656deb5d4`.
Their tarballs both hash to `bed93cd2085df64a2ca8ba578fd6d72babccc539042e83847e66a691bde59c1d`.
The accepted role runs the actual website TypeScript adapter for processing cases. The candidate runs the installed package.
Initialization rows compare package against package because TypeScript has no Wasm initialization equivalent.
They are controls, not claimed TypeScript speedups.

Raw evidence stays in `crates/ditherette-bench/target/s20-public-trial-02/` within `.worktrees/v1-s20-browser-bench`.
Each engine has a complete prepared asset snapshot, launch/reap journal, requests, results, stderr, and `report.json`.
Prepared manifests bind source/build provenance, Node, Playwright, browser files and aliases, package assets, TypeScript closure, and scripts.
Host system libraries remain an explicit trust boundary.

| Runtime | Version |
| --- | --- |
| Node | 24.19.0 |
| Playwright | 1.59.1 |
| Chromium | 147.0.7727.15 |
| Firefox | 148.0.2 |
| WebKit | 26.4, revision 2272 |

All engines run headless with cross-origin isolation and no additional launch arguments.
The exclusive coordinator runs engines sequentially. All implementation agents, builds, and tests finish before measurement.
Session `63230` completes with each engine returning exit 2 for its performance report, not a transport failure.
A subsequent scoped `/proc` check finds no owned benchmark worker, Node transport, browser, or compiler.

| Engine | Workers started/reaped | Retained samples | Zero samples |
| --- | ---: | ---: | ---: |
| Chromium | 72/72 | 7,200 | 0 |
| Firefox | 72/72 | 7,081 | 0 |
| WebKit | 72/72 | 7,200 | 52 |

Every engine journal has maximum live-worker count one and zero remaining workers.
Firefox enlargement candidate trials retain 72, 65, 72, and 72 samples after the declared one-second cap.
WebKit's zero samples all belong to identity latency. No samples are removed or replaced.
The independent read-only audit recomputes every report median and gate, and verifies all 2,465 snapshot files.
All 216 raw outputs match their frozen references and paired outputs exactly.
Worker SHA-256 is `15544ff0c420a387e0fd016bd2d077b29e10f95a9725347c66d2df1e9fedb8d4`.
The served asset identity is `0516d67ece568e7f96bcf57683cadcff12017263366be231168ff8977404c3a4`.

These SHA-256 values identify files within each engine's `*-results/` directory.
Each retained `prepared.json` matches the corresponding preparation copy.

| Engine | File | SHA-256 |
| --- | --- | --- |
| Chromium | `report.json` | `2f5338d5ddac421fd262d4455f0cd7cb10264a9feb63ebde734cc6617afee80d` |
| Chromium | `events.jsonl` | `cd1bb0f103d0d568e0b11949d1e56aa91ed09bbac2fdf08a15b2e1f5eb6e22e5` |
| Chromium | `prepared.json` | `f471ab5a133c76afdd19985de0e74639f46fe0f1e52d96a40c3b9a90f0c32e82` |
| Firefox | `report.json` | `84cbaf27a98d41f29edb340103577a1f86ca01d3e50448a6af3f9379e72ff15b` |
| Firefox | `events.jsonl` | `156f2fbdd8d49991486415554c7008448a120415d8983bf671683f2f4d6bdd63` |
| Firefox | `prepared.json` | `fa7b4a926caa490ee9455d226edd0946cbadaf66d94fd587c7e3dd9da0a13c62` |
| WebKit | `report.json` | `b9eb70f9430772053f3a1bd9d974ef4ee972760808565d260b152539a9b42b04` |
| WebKit | `events.jsonl` | `b049a65db77c952457ec6cd82f314d0a92fc491db0811fe39625b7ff6ff2db8f` |
| WebKit | `prepared.json` | `7f353522b41154a53038077cd6caa2a87f9db9f02a661e584a6f91ab006d7619` |

## Results

Times are median milliseconds. Ratios are package divided by TypeScript, except package/package initialization controls.
The recorded gate also considers pair consistency and timer resolution. A rounded ratio alone does not determine acceptance.

| Engine | Case | Accepted ms | Candidate ms | Ratio | Gate |
| --- | --- | ---: | ---: | ---: | --- |
| Chromium | Identity latency | 0.560 | 0.325 | 0.580 | Pass |
| Chromium | Identity throughput | 0.568 | 0.338 | 0.596 | Pass |
| Chromium | Reduction latency | 0.740 | 0.665 | 0.899 | Pass |
| Chromium | Reduction throughput | 0.744 | 0.671 | 0.903 | Pass |
| Chromium | First reduction call | 0.750 | 2.085 | 2.780 | Regression |
| Chromium | Enlargement latency | 2.965 | 2.383 | 0.804 | Pass |
| Chromium | Unequal axes latency | 0.995 | 0.820 | 0.824 | Pass |
| Chromium | Initialization bytes | 0.335 | 0.345 | 1.030 | Pass |
| Chromium | Initialization compiled | 0.050 | 0.055 | 1.100 | Inconclusive |
| Firefox | Identity latency | 0.280 | 0.340 | 1.214 | Inconclusive |
| Firefox | Identity throughput | 0.297 | 0.285 | 0.959 | Pass |
| Firefox | Reduction latency | 0.640 | 3.620 | 5.656 | Regression |
| Firefox | Reduction throughput | 0.632 | 3.640 | 5.755 | Regression |
| Firefox | First reduction call | 0.640 | 4.980 | 7.781 | Regression |
| Firefox | Enlargement latency | 2.560 | 13.920 | 5.438 | Regression |
| Firefox | Unequal axes latency | 0.820 | 4.680 | 5.707 | Regression |
| Firefox | Initialization bytes | 2.840 | 2.680 | 0.944 | Inconclusive |
| Firefox | Initialization compiled | 0.120 | 0.120 | 1.000 | Inconclusive |
| WebKit | Identity latency | 0.020 | 0.060 | 3.000 | Inconclusive |
| WebKit | Identity throughput | 0.049 | 0.085 | 1.722 | Regression |
| WebKit | Reduction latency | 0.300 | 0.400 | 1.333 | Regression |
| WebKit | Reduction throughput | 0.365 | 0.404 | 1.107 | Inconclusive |
| WebKit | First reduction call | 0.300 | 1.700 | 5.667 | Regression |
| WebKit | Enlargement latency | 1.280 | 2.420 | 1.891 | Regression |
| WebKit | Unequal axes latency | 0.420 | 0.820 | 1.952 | Regression |
| WebKit | Initialization bytes | 0.760 | 0.780 | 1.026 | Inconclusive |
| WebKit | Initialization compiled | 0.140 | 0.140 | 1.000 | Inconclusive |

Firefox identity latency and compiled initialization are resolution-limited.
WebKit identity latency and byte initialization are resolution-limited.
Other inconclusive rows fail consistency requirements, not necessarily timer resolution.

## Required follow-up

S41 must resolve confirmed per-case regressions and obtain fresh complete release evidence.
Chromium warm-call improvements do not excuse its first-call regression or the other engines' regressions.
Do not rerun unchanged candidates just to seek a passing result.

Mia reports historical compiled resize results over 100 times faster than TypeScript.
The old benchmark batches its legacy kernel inside Wasm. The new public API uses the canonical S19 kernel.
This is a kernel and timing-scope difference; it does not prove the cause of these regressions.
A bounded developer-only comparison will measure the same canonical kernel internally and through the unchanged public call.
It must keep full output proof and exclusive execution. Kernel-only results cannot replace complete-call release gates.

The known TypeScript 2-to-49 center tie mismatch remains outside this exact matrix, with conformance evidence retained.
No frozen reference or TypeScript arithmetic changes to hide that mismatch.
The [incomplete first trial](61-trial-01-failure.md) remains separate. Its partial samples never enter these results.
