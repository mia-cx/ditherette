# #211 Evaluate remaining scalar performance and shared-kernel opportunities

## Summary

Work through the 16 source-level findings from the two final code passes on main `90bf7c4b`.
Each item ends selected (committed with evidence), rejected with evidence, or deferred with a concrete reason.
Exact output, frozen `spec/`, capacity accounting, progress and alpha behavior stay unchanged.

## Acceptance criteria

- [x] Every #211 item is reported as selected, rejected with evidence, or deferred with a reason.
- [x] Selected changes keep byte-exact output against existing implementations and pass the frozen-spec guard.
- [x] Selected performance changes have browser qualification (Chromium and optimizing Firefox, threads disabled, cold and warm separate) or, for threaded-only paths, native threaded timing.
- [x] Maintenance changes delete code without changing behavior.

## TODOs

Performance:

- [x] Bicubic scale-aware scratch blocks opt-in. Selected.
- [x] Yliluoma: skip unused `matcher.nearest()` for `Everywhere`. Selected.
- [x] Yliluoma: hoist metric dispatch out of the mix search. Selected.
- [x] Yliluoma: reuse adaptive coordinate rows. Selected.
- [x] Yliluoma: bounded mixture memoization for `Everywhere`. Selected.
- [x] Lazy source opacity. Selected.
- [x] Pruned hue matcher on byte-feedback diffusion misses. Selected.
- [x] Trilinear: plan exact bilinear taps once per level. Selected (also plans the exact area stage).
- [x] Trilinear: read mip level 0 from the source view. Selected.
- [x] Bilinear/area: write the first vertical tap instead of zero-filling. Selected.
- [x] Bilinear planning preflight reuse. Rejected.
- [x] Resize band scratch and RGB cache double zeroing. Rejected (all three parts).
- [x] Banded perturbation adaptive rows and `BayerBytes` threads gate. Selected (both parts).

Maintenance:

- [x] Shared resize anchors, coordinate mapping and band assertions.
- [x] Merge boundary traits and benchmark loops.
- [x] Stale docs.

Validation:

- [x] Native tests scalar and `--features threads`, bench-subjects, wasm32 and bench crate checks, frozen-spec guard, private Wasm ABI tests.
- [x] Browser qualification for selected scalar changes.
- [x] Native threaded timing for threaded-only changes.

## Notes

- Worktree `.worktrees/perf-211-final-pass`, branch `perf/211-final-pass`, base `90bf7c4b`. Root checkout untouched.
- Browser harness: `ditherette-performance-20260920/celeste` via `run-q211-pair.mjs` (six-round `qualify`, optimizing Firefox, scalar Wasm, medians of processing ms). Raw results stay outside Git.
- Baseline scalar Wasm rebuilt from `90bf7c4b` reproduces the landed package byte for byte (432,281 B). Final scalar Wasm 442,971 B (+10,690, +2.5%): mostly per-metric Yliluoma search copies and trilinear plans.
- Builds after the VM stall use `nice` and `CARGO_BUILD_JOBS=2`.

### Evidence, all PNGs byte-exact

| Change | Chromium cold / warm | Firefox cold / warm |
|---|---|---|
| Bicubic scale-aware (10%) | 156.15→140.20 (−10.2%) / 130.55→115.40 (−11.6%) | 152.0→138.5 (−8.9%) / 134.5→117.5 (−12.6%) |
| Yliluoma Bayer 2 | 235.3→67.8 (−71.2%) / 229.9→62.2 (−73.0%) | 194.0→62.0 (−68.0%) / 188.5→59.5 (−68.4%) |
| Yliluoma Bayer 4 | 765.9→213.2 (−72.2%) / 758.2→208.0 (−72.6%) | 610→202 (−66.9%) / 590→201 (−65.9%) |
| Yliluoma Bayer 8 | 2876.6→790.9 (−72.5%) / 2864.9→785.5 (−72.6%) | 2305.5→754 (−67.3%) / 2249→754 (−66.5%) |
| Trilinear, level 0 + first tap (vs main) | 383.1→355.6 (−7.2%) / 243.2→226.0 (−7.1%) | 330.5→304.0 (−8.0%) / 221.0→187.5 (−15.2%) |
| Trilinear tap plans (vs previous) | 400.1→264.3 (−34.0%) / 228.4→170.5 (−25.4%) | 294.5→217.0 (−26.3%) / 190.5→134.5 (−29.4%) |
| Area 10% / 50% | −12.9% / −7.9% cold; −4.7% / +0.8% warm | −8.8% / −9.4% cold; 0.0% / −3.2% warm |
| Bilinear 25% resize / 50% process | −4.9% / −4.0% cold; −0.9% / −2.2% warm | −2.0% / −6.3% cold; +3.8% / −3.5% warm |
| Final HEAD, scale-50-process | 41.40→37.15 (−10.3%) / 32.25→29.70 (−7.9%) | not rerun |

- Batch-2 `scale-50-process` read +1–6% in both browsers; a bisect across `71c19aa4`, `d49ef5b0`, `4d7e12f1`, `376259e5` was non-monotonic (+8.7% at the memo commit, flat at the next), baseline itself varied 40.5–41.5 ms. Final HEAD shows −10.3%/−7.9%. Treated as code-layout noise.
- `extra-yliluoma-16` (adaptive, Bayer 16) exceeds the harness 300 s recipe limit on main in qualify mode. Native adaptive Bayer 4 at 128x96: rows vs direct −8.7% (sRGB), −13.0% (Oklch circular hue).
- Opacity: Celeste has transparent pixels, so the old scan exited early; the change removes a full read pass only for opaque sources and convolution-free recipes. Process-case controls stayed within noise.
- Diffusion pruning: no hue-arc diffusion recipe in the harness; `spec_diffusion` and the bounded-hue equivalence test cover exactness. Same pruning already measured for quantize.
- Threaded native (release, rayon): Bayer bytes in threaded non-banded calls 27.05→5.46 ms (−79.8%, Bayer 2), 26.38→5.61 ms (−78.7%, Bayer 8) at 1300x2084. Banded adaptive Oklab blue noise (32-row bands, 2 workers) 408.2→207.6 ms (−49.1%).

### Rejections

- Bilinear planning reuse: planning is 0.2–0.4% of a call natively (0.16–0.63 ms vs 57–303 ms kernel for 2600x4168 to 650–3900 wide); dropping one of three evaluation passes saves ≤0.13%.
- Banded RGB cache clear-once: slower natively in both variants (clear-once median 31.2–42.7 ms vs 28.8–31.8; no-clear with fresh tables 35.1–36.4 vs 30.6–30.9). Dropped.
- Scalar RGB cache double zero: one extra 1–2 MiB memset (~50–250 µs) per call, below browser timer resolution.
- Resize scratch discarded on band selection: only the Lanczos3 scale-aware quarter-scale band class wastes a large buffer (282 source rows x output width x 32 B, ~4.6–9 MB, ~1–2 ms zero-fill vs a multi-hundred-ms call, <1%); half-scale area/bilinear waste one source row.
