# S19 nearest measurement

Historical evidence for the recorded revisions only. [The restoration](108-restore-landed.md) removes this replacement from current production.
These measurements do not establish restored-kernel performance.

The first bounded native experiment passes every required case with exact frozen-reference output.
The candidate meets the 20% improvement target. No second candidate or repeated measurement is needed.
This evidence covers native kernels, not complete browser calls or initialization. Those measurements remain required in S20.

## Artifacts and execution

| Role | Clean source revision | Executable SHA-256 |
|---|---|---|
| Accepted literal copy | `89b570e0dbb4280352b157bfde5b20c3a7e80a9e` | `28ff10056bfc58fd35755e937f7178c69088742cbb3e43bbd960aa6ecb47b0f4` |
| Incremental candidate | `f9b51e45cad691a207e8a0e346bdd419af7dd909` | `d1a372be61863164312cb638afce5b9c9992bd42be939b4b16947fe515b0a842` |

The accepted revision has unchanged Rust bytes from literal-copy checkpoint `0ede7f6c`.
Both final executables use `cargo +1.97.0 build --locked --release --bins -j 4` in their own clean worktrees.
The candidate's separate example prepares the typed experiment and does not run image measurements.
Immutable executable snapshots, their complete digests, and the prepared fixture/settings records precede measurement.

Host `athena-hephaestus` runs Linux `6.12.95+deb13-amd64` on an AMD Ryzen 9 7950X with 24 visible logical CPUs.
All three implementation agents and their owned build/test processes stop before the quiet attestation.
No unrelated project process is stopped. The journal records one-minute host load between 0.52 at entry and 0.76 at exit.

The external paired coordinator owns the host lease for all four alternating AB/BA pairs.
It launches 80 sequential benchmark children and reaps all 80. Every child observes one live benchmark process.
All 8,000 requested samples survive, with 100 samples in every child result.
Session `66379` exits zero. A subsequent `/proc` check finds no owned child still alive.
The launch/reap journal spans 44.355 seconds. No implementation, build, or test runs during that interval.

## Measured results

All rows use the center anchor. Latency times one native call; throughput reports calibrated per-call time.
Warmup is 250 ms per child, measurement is capped at 1,000 ms, and the throughput calibration target is 5 ms.
Fixture decoding, allocation, JS copies, and hashing are outside native timing. There is no application cache in this adapter.

| Case | Input → output | Accepted median µs | Candidate median µs | Reduction | Gate |
|---|---|---:|---:|---:|---|
| Identity latency | 512×384 → 512×384 | 379.655 | 12.861 | 96.61% | Pass |
| Identity throughput | 512×384 → 512×384 | 382.206 | 13.334 | 96.51% | Pass |
| Reduction latency | 512×384 → 256×192 | 93.872 | 45.276 | 51.77% | Pass |
| Reduction throughput | 512×384 → 256×192 | 95.586 | 45.957 | 51.92% | Pass |
| Enlargement latency | 128×96 → 512×384 | 381.170 | 181.713 | 52.33% | Pass |
| Enlargement throughput | 128×96 → 512×384 | 382.384 | 182.217 | 52.35% | Pass |
| Unequal axes latency | 512×96 → 128×384 | 93.301 | 45.071 | 51.69% | Pass |
| Unequal axes throughput | 512×96 → 128×384 | 94.849 | 45.858 | 51.65% | Pass |
| Tiny latency | 3×2 → 7×5 | 0.100 | 0.070 | 30.00% | Pass |
| Tiny throughput | 3×2 → 7×5 | 0.086 | 0.054 | 36.64% | Pass |

Each case passes the fresh-pair regression gate, including every alternating pair and its spread bound.
Every reference/accepted/candidate comparison is exact, including metadata. No visual-loss approval is needed.
The unchanged-width row copy explains the separate identity result; it is not a general 97% resize claim.

## Retained evidence

The worktree `.worktrees/v1-s19-nearest-opt` retains these generated artifacts:

- `crates/ditherette-bench/target/s19-nearest-experiment.json` contains the fixed typed experiment.
- `crates/ditherette-bench/target/s19-nearest-prepared/` contains both immutable executable snapshots and `prepared.json`.
- `crates/ditherette-bench/target/s19-nearest-trial-01/` contains the prepared input, 80 requests, 80 raw results, stderr, launch/reap journal, and `report.json`.

The committed generator `crates/ditherette-bench/examples/s19_nearest_plan.rs` reproduces fixture bytes and identities.
The literal-copy manifest and checkpoint remain historical proof; the measured kernel may now replace the canonical production implementation.
Subsequent promotion must preserve its arithmetic. Public-call conformance and browser measurements remain separate completion gates.
