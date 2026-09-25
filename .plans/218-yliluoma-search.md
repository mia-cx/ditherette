# #218 Make Yliluoma viable for video-size frames

## Summary

Replace repeated exhaustive Euclidean mix scans with a bounded, exact search over prepared discrete mixes. Keep the frozen `spec/` implementation unchanged and retain the literal scan for unsupported metrics.

## Acceptance criteria

- [ ] Record scalar native and browser Wasm baselines for a 3840×2160 to 480×270 Wplace/Oklab recipe, with new-frame and same-frame timing distinguished.
- [ ] Oklab 4×4 full processing is at least 10× faster than its fresh baseline, or document the measured shortfall and keep #218 open.
- [ ] The accelerated path matches frozen `spec/` byte for byte, including ties, alpha, both placement modes, and all Bayer sizes.
- [ ] Prepared memory is bounded and accounted for; cache reuse does not break invalidation, eviction, or multi-frame safety.
- [ ] Existing metrics retain correct behavior, and representative non-Yliluoma processing does not regress materially.

## TODOs

- [ ] Establish a reproducible 4K/Oklab Yliluoma benchmark and record baseline timings.
- [ ] Implement exact indexed nearest-mix search for Euclidean metrics with focused differential tests.
- [ ] Integrate the index into production Yliluoma preparation and bounded pipeline reuse.
- [ ] Run scalar/native and browser qualification, retain measured wins, and document cost, fidelity, and limits.

## Notes

- Worktree: `.worktrees/issue-218-yliluoma`. Branch: `perf/218-yliluoma-index`. Base: `5cf5da54d`.
- Previous scalar Wasm 4×4 Everywhere result: 205.6 ms at 52×83 in the external `q211-b1a-candidate-chromium-extra` report. This is not a 4K/Oklab baseline.
- Full Wplace uses 63 visible colors, yielding 2,016 unordered pairs and 34,272 literal score evaluations per uncached RGB for 4×4.
- Keep `spec/` untouched. Benchmark jobs must run one at a time. Clean Rust build targets after the PR.
