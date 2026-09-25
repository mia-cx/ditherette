# #218 Make Yliluoma viable for video-size frames

## Summary

Replace repeated exhaustive Euclidean mix scans with a bounded, exact search over prepared discrete mixes. Keep the frozen `spec/` implementation unchanged and retain the literal scan for unsupported metrics.

## Acceptance criteria

- [x] Record scalar native and browser Wasm baselines for Wplace/Oklab at 480×270, including the 3840×2160 browser input.
- [x] Oklab 4×4 full browser processing is at least 10× faster than its fresh baseline.
- [x] The accelerated path matches frozen `spec/` byte for byte, including ties, alpha, both placement modes, and all Bayer sizes.
- [x] Prepared memory is bounded and accounted for; call-local indices cannot become stale across requests.
- [x] Existing metrics retain correct behavior, and representative non-Yliluoma processing does not regress materially.

## TODOs

- [x] Establish a reproducible 4K/Oklab Yliluoma browser benchmark and record baseline timings.
- [x] Implement exact indexed nearest-mix search for Oklab with focused differential tests.
- [x] Integrate the optional, bounded index into standalone and pipeline calls.
- [x] Run scalar/native and browser qualification, retain measured wins, and document cost, fidelity, and limits.

## Notes

- Worktree: `.worktrees/issue-218-yliluoma`. Branch: `perf/218-yliluoma-index`. Base: `5cf5da54d`.
- Previous scalar Wasm 4×4 Everywhere result: 205.6 ms at 52×83 in the external `q211-b1a-candidate-chromium-extra` report. This is not a 4K/Oklab baseline.
- Full Wplace uses 63 visible colors, yielding 2,016 unordered pairs and 34,272 literal score evaluations per uncached RGB for 4×4.
- Keep `spec/` untouched. Benchmark jobs must run one at a time. Clean Rust build targets after the PR.
- Chromium scalar Wasm at 3840×2160 nearest to 480×270, 63 Wplace colors, Oklab, Everywhere:
  - 2×2 warm median: 795.8 ms baseline, 25.7 ms indexed. Output PNG SHA-256 matches.
  - 4×4 warm median: 2596 ms baseline, 32.5 ms final build, a 79.9× gain. Output PNG SHA-256 matches.
  - 8×8 warm median: 9928 ms baseline, 66.4 ms indexed. Output PNG SHA-256 matches.
- Evidence lives outside the repository under `ditherette-performance-20260920/celeste/video218*-chromium`.
- The index is call-local, so palette and source changes cannot reuse stale mixtures. Its actual reserved bytes count against the processing limit. Small outputs and insufficient budgets use the literal scan.
- A recorded `ditherette-bench` native pair compares clean main `5cf5da54` with `d226d8cc` on a synthetic 480×270 frame using the 63-color Wplace palette and Oklab 4×4. Median complete-call time was 7441.1 ms versus 96.5 ms, a 77.1× gain. The pair gate passed with zero differing pixels and metadata.
- The scalar Wasm binary grew from 439,519 to 459,915 bytes in the final package build, about 4.6%. Browser setup and JS worker roundtrip were excluded from the processing timings above.
- Representative untouched browser paths did not regress: YCbCr direct quantization measured 3.4 to 3.2 ms; Oklab Atkinson diffusion measured 16.5 to 16.5 ms. Both rendered PNGs stayed byte-identical.
