# S41 bounded candidate selection

Keep the current production implementation. Canonical SHA-write batching does not
show a useful complete-call win, and one required WebKit comparison is inconclusive.
The candidate remains separate and unselected. No repeat is planned.

## Fresh artifacts

| Role | Source revision | Ordinary tarball SHA-256 |
| --- | --- | --- |
| Current | `a895267baea624a6e89bfcef6c5147f170e8a8f7` | `1e9fa5c926f41190a10ef6230a1acbc349718ac0ab94c8542fa2b1a20bc4775d` |
| Candidate | `80838c2c1ee11788e2e2f195b51f5b34a2c06f59` | `2fd1adeb95d036d62239d1eb24d3d19b3df7072cfc370de42f35fdb83335aef9` |

Both clean snapshots use the same benchmark infrastructure. Their sole production
diff is the previously tested canonical-key writer in `prod/pipeline/identity.rs`.
Fresh candidate release tests pass 74 checks. No landed resize kernel changes.
Current preparation reproduces the S40 ordinary tarball byte-for-byte.

Native executable hashes are `2c26ec86d5ba668667438713ecbeb9c44fc2c384ab0e05a32e3ff09aa4320650`
and `fdbf1932a699ca9319976a90cb570b28944356a4e9ddc4087e1c45a9116b15e7`.
Full provenance remains in each immutable source worktree's `target/s41-*-native`
and `target/s41-*-public` directories.

## Results

The fixed trial finishes in 78.1 seconds. All 32 workers exit before the next starts.
Each worker records 20 samples, for 640 samples total. Every observed process count
is one. All 16 actual production pairs are byte-identical; frozen comparisons also pass.
No agents, builds, or tests run during measurement. Background services remain untouched.

Each entry is candidate/current median latency. Values below 1 favor the candidate.

| Runtime | Cold Lanczos3 resize | Cold resized diffusion Process | Gate |
| --- | ---: | ---: | --- |
| Native | 1.002 | 0.999 | Pass |
| Chromium 147.0.7727.15 | 0.997 | 0.997 | Pass |
| Firefox 148.0.2 | 1.020 | 0.997 | Pass |
| WebKit 26.4 | 1.171 | 1.045 | Resize inconclusive; Process passes |

WebKit resize's alternating ratios are 0.973 and 1.276. This is inconclusive,
not a confirmed regression. Native, Chromium, and Firefox show no useful improvement.
The passing gate is a regression check, not proof of a speedup.

Raw requests, results, samples, process events, prepared snapshots, and completion
records remain in `.worktrees/v1-resize-integration/target/s41-selection-01`.

| Report | SHA-256 |
| --- | --- |
| Native | `1c57ad7243e99ef931568f2d05c329923562a66f5905421e96be7873747dc31c` |
| Chromium | `b4ec8ac2ccdd10524226c6808f68bafb9ff8ef4c8b358fdbd0e30024b67073a2` |
| Firefox | `56f312a1904498cfafe5de01c857f1d11421bf38a343ddee14886ea3c97ff911` |
| WebKit | `8675586f384d6bb22d1c0cba161d8e0a4b17aebe0a22b040e7f0c55c5aee5d23` |

## Next comparison

Historical cold/warm anchors compare freshly rebuilt pre-S32 `d51a70a2` against
the retained current `a895267b`. The historical production and benchmark source
stay unchanged. Browser anchors retain their original page execution; all eight
case JSON values match the historical generator. This lane has 128 serial workers
and a 20-minute launch deadline. Incomplete or noisy results remain open release gates.

Historical tarball SHA-256 is `83a07b5a9bc7f0f04ba1bcb4c6db13f8daa6101bf4b656651aa06be9f1cf59eb`.
Historical native executable SHA-256 is `c0498434d41f774cc076d9d587a6944669b73140a4cb123e7bfbb35ac47bc715`.
