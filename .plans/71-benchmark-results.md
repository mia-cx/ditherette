# S30 complete Process measurement report

Issue #71. Measured source `e5aae7bf0e1761af2f970b6da75d34cf3a813323`, 2026-09-08.
The required complete API is retained. No new source candidate exists or is selected.

All 64 actual staged/Process pairs match exactly, including case identity, indexed output, palette, transparency, and warnings.
All four aggregate gates remain `Incorrect`, solely because area retains its inherited frozen-reference difference.
There is no confirmed greater-than-10% regression. Inconclusive cases remain inconclusive, not performance wins.

## Compared calls and fixed budget

Both roles use the same clean source, native executable, public package, and frozen-only oracle.
Accepted executes actual resize followed by actual ditherAndQuantize. Candidate executes actual Process.
The labels identify call paths, not different source revisions. Output implementation identities intentionally differ.

Native subjects are `prod:process:request:staged-v1` and `prod:process:request:processor-v1`.
Public backends are `package-staged` and `package`.
Both paths include ordinary per-call preparation, allocations, and boundary copies.
Native output construction and drop stay inside timing. Browser observation and disposal stay outside the timed public methods.
Public instance initialization, module compilation, fixture serialization, frozen verification, and preflight are untimed.
The public trials use primed scalar instances, no application cache, and single-call latency without batching.

The [predeclared matrix](71-benchmark.md#declared-experiment) remains unchanged.
It uses all seven filters, representative dither families, 16-color palettes, and a small four-color Yliluoma control.
Eight cases × two alternating-order pairs × two roles × four runtimes gives 128 serial workers.
Each worker declares 50 ms warmup, at most 20 samples, and a 10-second measured-call cap.
No pilot, retry, input substitution, sample censoring, or new optimization followed the declaration.

| Runtime | Workers started / reaped | Samples | Aggregate gate |
| --- | ---: | ---: | --- |
| Native | 32 / 32 | 640 | Incorrect |
| Chromium | 32 / 32 | 640 | Incorrect |
| Firefox | 32 / 32 | 584 | Incorrect |
| WebKit | 32 / 32 | 640 | Incorrect |
| Total | 128 / 128 | 2,504 | No overall pass |

Event logs show maximum live workers of one and zero unreaped workers.
Firefox's area and bilinear workers each stop after 13 samples at the declared cap.
The collector checks the cap after a completed call, so those eight trials accumulate 10.21–10.35 seconds each.
This is 56 fewer samples than the 2,560 maximum, not a retry or missing-worker condition.

## Timing results

Values are pooled sample medians in milliseconds; ratios below one favor Process.
A case-level `pass` means the declared no-greater-than-10%-regression gate passes, not a proven speedup.
Area timings are diagnostic only. Their presence does not change the `Incorrect` gate.
Exact pair ratios, sample counts, flags, and original report hashes are in [the machine-readable summary](71-benchmark-results.json).

### Native

| Case | Staged ms | Process ms | Process / staged | Gate |
| --- | ---: | ---: | ---: | --- |
| nearest-none | 2.6743 | 2.6660 | 0.9969 | pass |
| area-bayer | 15.9624 | 15.6285 | 0.9791 | incorrect |
| bilinear-random | 15.7967 | 15.6323 | 0.9896 | pass |
| bicubic-blue-noise | 4.0456 | 4.0273 | 0.9955 | pass |
| lanczos2-floyd-steinberg | 0.5510 | 0.5377 | 0.9759 | pass |
| lanczos3-atkinson | 0.6788 | 0.6729 | 0.9914 | pass |
| trilinear-sierra-lite | 1.1819 | 1.1744 | 0.9936 | pass |
| nearest-yliluoma | 0.0365 | 0.0216 | 0.5919 | inconclusive |

### Chromium 147.0.7727.15

| Case | Staged ms | Process ms | Process / staged | Gate |
| --- | ---: | ---: | ---: | --- |
| nearest-none | 5.0675 | 4.9050 | 0.9679 | pass |
| area-bayer | 88.7000 | 89.0925 | 1.0044 | incorrect |
| bilinear-random | 88.9025 | 88.5925 | 0.9965 | pass |
| bicubic-blue-noise | 22.6775 | 22.8150 | 1.0061 | pass |
| lanczos2-floyd-steinberg | 1.1150 | 1.1150 | 1.0000 | pass |
| lanczos3-atkinson | 1.3700 | 1.3375 | 0.9763 | pass |
| trilinear-sierra-lite | 2.0475 | 2.0200 | 0.9866 | pass |
| nearest-yliluoma | 0.1100 | 0.1150 | 1.0455 | inconclusive |

### Firefox 148.0.2

| Case | Staged ms | Process ms | Process / staged | Gate |
| --- | ---: | ---: | ---: | --- |
| nearest-none | 32.5700 | 32.6200 | 1.0015 | pass |
| area-bayer | 785.3200 | 788.4400 | 1.0040 | incorrect |
| bilinear-random | 790.7200 | 788.2800 | 0.9969 | pass |
| bicubic-blue-noise | 200.7700 | 201.0900 | 1.0016 | pass |
| lanczos2-floyd-steinberg | 8.3400 | 8.1800 | 0.9808 | pass |
| lanczos3-atkinson | 10.3900 | 10.3800 | 0.9990 | pass |
| trilinear-sierra-lite | 16.0700 | 15.9100 | 0.9900 | pass |
| nearest-yliluoma | 0.9800 | 0.9200 | 0.9388 | pass |

### WebKit 26.4

| Case | Staged ms | Process ms | Process / staged | Gate |
| --- | ---: | ---: | ---: | --- |
| nearest-none | 5.7800 | 5.8600 | 1.0138 | inconclusive |
| area-bayer | 84.9400 | 84.8900 | 0.9994 | incorrect |
| bilinear-random | 85.1600 | 85.5800 | 1.0049 | pass |
| bicubic-blue-noise | 21.7400 | 21.7300 | 0.9995 | pass |
| lanczos2-floyd-steinberg | 1.1500 | 1.1200 | 0.9739 | inconclusive |
| lanczos3-atkinson | 1.3600 | 1.3600 | 1.0000 | pass |
| trilinear-sierra-lite | 1.9100 | 1.9600 | 1.0262 | pass |
| nearest-yliluoma | 0.1000 | 0.1000 | 1.0000 | inconclusive |

Native Yliluoma pair ratios are 0.4887 and 1.0037. The pooled gain is not stable evidence.
Chromium Yliluoma remains timer-limited; its roughly 5 µs quantum prevents a conclusive threshold decision.
WebKit nearest, Floyd-Steinberg, and Yliluoma remain inconclusive from pair disagreement.
Their pair ratios are respectively 1.0547/0.9493, 0.9016/1.0000, and 1.0000/1.2000.
Firefox and WebKit expose roughly 20 µs timer quanta. No timing zeros or coarse observations were replaced.

## Exact composition and inherited area attribution

Raw `*.result.json` files were paired by case, pair number, and role.
Every actual pair has equal `output.case` and `output.output`.
Comparing the complete `RecordedOutput` objects would also compare intentionally different implementation identities.

The verification array contains two proof contexts per pair.
In `crates/ditherette-bench/src/paired.rs::compare`, the first uses accepted staged output and candidate Process output.
The second substitutes the candidate worker's frozen reference into the proof's accepted slot.
That second proof checks reference agreement and frozen-versus-Process conformance.
For area, the alternating `accepted_candidate.exact: false` entries belong to the second context.
They do not mean actual staged and Process outputs differ.
`paired/coordinator.rs` preserves these as separate `review-001-*-production` and `review-001-*-reference` evidence directories.

Area-only `measure_nonexact: true` was approved before measurements.
The other seven cases retain `false`. The frozen gate is unchanged.
On native and all three browsers, landed area resize differs from frozen resize in 125 RGBA bytes, each by at most one.
The seven differing indexed positions are 2644, 3160, 6772, 7804, 8836, 11932, and 12448.
Unchanged frozen post-resize processing on the landed resized bytes equals both actual production calls exactly.

Each browser executes separately identified frozen area-resize and post-resize Process probes.
Actual browser resized bytes equal the independently identified post-resize input.
Both primed and fresh compositions pass this attribution without altered reference bytes or tolerances.
All eight Process frozen outputs also match their native frozen counterparts in this fixture set.
The existing `blue-noise-oklab-adaptive2` native-versus-Wasm diagnostic remains separate and retained.
Its actual package output matches the target-local frozen Wasm reference; no universal cross-target parity is claimed.

## Validation and provenance

Fresh installed-package adapter conformance passes in Chromium 147.0.7727.15, Firefox 148.0.2, and WebKit 26.4.
Each engine retains 431 identified main references, two separately identified area probes, and 16 Process/staged compositions.
The main fixtures cover 47 quantize, 12 field, four blue-noise, 360 diffusion, and eight Process cases.
Oracle contexts close before package initialization. Evidence records the oracle manifest and tarball hash before assertions.
This supplements the runtime owner's 423 exact Process/staged compositions per engine and focused native/private/public checks.
The coordinator completed the trusted frozen guard before measurements. This report pass ran no builds or test suites.

The host is AMD Ryzen 9 7950X, Linux x86_64, kernel 6.12.95+deb13-amd64, with 24 logical CPUs exposed.
Native/scalar uses Rust 1.97.0; threads uses Rust 1.82.0-nightly.
Browser transport uses Node 24.19.0 and Playwright 1.59.1.
The build records pnpm 11.13.0, TypeScript 6.0.3, wasm-pack 0.15.0, wasm-bindgen 0.2.121, and wasm-opt 117.

All retained paths below are relative to
`.worktrees/v1-s30-process/target/s30-trial-01/`.

| Artifact | SHA-256 |
| --- | --- |
| Native worker | `9575273f58645c8d107f6df3848d49fbfde1ec0354ae7e54a51ecc28db6310b0` |
| Native coordinator | `1a2256c9b981e6bb414de138bf76e739fd80249de75e01653a7a1a6306c35c5c` |
| Native build provenance | `d9f37ef080b8a2dd23c51d0c7bf36097fff7e313d52bf9a134f0760b1f0dfc46` |
| Public build provenance | `81a05b374a1605fc1e29472ca0801b9d3ca852eee113fc57c7c5d5a37f0c24dc` |
| Published-format tarball | `379c733b02bc67a24500d3ae825901d17d5fa342f93d114c20761da1aa9193b2` |
| Scalar Wasm | `170b1ad5a9a13290b0d2e07db141f35895abf3d51cb237b999b86f93122fa739` |
| Threads Wasm | `9731aad8eb95ad8aeb9dad59f13711202e3818a69b81cb14624f147f39089e71` |
| Process-capable frozen oracle Wasm | `300f61644c4b7757d1ad80b97c515121a5ad241fa0051e9827e448f0067ffb64` |
| Frozen oracle manifest | `09f703c2cb1447ab4b028a8d16691b1534a62a72279d1387cba05b1fecbdabaf` |
| Original Process fixture with area probes | `63ac50059dfe6fd4681d9afdbe9a1dca8394c19ae561b0d0bf2e3a3d986b6488` |
| Three-engine conformance summary | `fbcfce096e5987f44327b2228d75476556ca6cc60d8aca5b84d74f93b3f0d72b` |

[The JSON summary](71-benchmark-results.json) binds every runtime's report, prepared snapshot, event log, runtime digest, and conformance evidence.
It also records a digest of each complete results directory, including raw requests/results, stderr, and mismatch review artifacts.
Each directory digest hashes the bytewise-path-sorted list of relative paths, byte lengths, and individual SHA-256 values.
The exact UTF-8 JSON encoding is recorded there for reproduction.
No evidence file or runtime source was changed for this report.

## Artifact sizes and held work

Both compared roles have identical artifact sizes because they use the same build.

| Artifact | Raw bytes | Gzip level 9 bytes |
| --- | ---: | ---: |
| Scalar Wasm | 267,054 | 122,611 |
| Threads Wasm | 358,201 | 153,689 |

The installed package contains 38 files totaling 806,673 bytes. Its tarball is 308,337 bytes.
The native worker is 5,845,912 bytes; the coordinator is 3,284,976 bytes.
Gzip sizes are computed from unmodified Wasm bytes, not separate shipped artifacts.
There is no retained tarball for the complete pre-S30 join `22b6dd78`.
Incomplete S28/S29 sibling artifacts are not substituted, and no predecessor rebuild was run.
This report therefore makes no pre-S30 package-growth claim.

S41 retains the inconclusive timing cases, representative palette/matching expansion, and area release-evidence gap.
S41/S42 retain the missing valid predecessor size comparison if needed.
These results do not authorize a new nonexact optimization or override the inherited frozen gate.
The complete required API remains implemented and validated. No PR merge, publication, deployment, or cleanup belongs to this report.

