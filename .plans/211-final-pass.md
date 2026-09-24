# #211 Evaluate remaining scalar performance and shared-kernel opportunities

## Summary

Work through the 16 source-level findings from the two final code passes on main `90bf7c4b`.
Each item ends selected (committed with evidence), rejected with evidence, or deferred with a concrete reason.
Exact output, frozen `spec/`, capacity accounting, progress and alpha behavior stay unchanged.

## Acceptance criteria

- [ ] Every #211 item is reported as selected, rejected with evidence, or deferred with a reason.
- [ ] Selected changes keep byte-exact output against existing implementations and pass the frozen-spec guard.
- [ ] Selected performance changes have native paired or browser qualification evidence (Chromium and optimizing Firefox, threads disabled, cold and warm separate).
- [ ] Maintenance changes delete code without changing behavior or Wasm hot-path codegen.

## TODOs

Performance, in issue priority order:

- [ ] Bicubic scale-aware scratch blocks opt-in.
- [ ] Yliluoma: skip unused `matcher.nearest()` for `Everywhere`.
- [ ] Yliluoma: hoist metric dispatch out of the mix search.
- [ ] Yliluoma: reuse adaptive coordinate rows.
- [ ] Yliluoma: bounded mixture memoization for `Everywhere`.
- [ ] Lazy source opacity.
- [ ] Pruned hue matcher on byte-feedback diffusion misses.
- [ ] Trilinear: plan exact bilinear taps once per level.
- [ ] Trilinear: read mip level 0 from the source view.
- [ ] Bilinear/area: write the first vertical tap instead of zero-filling.
- [ ] Bilinear planning preflight reuse.
- [ ] Resize band scratch and RGB cache double zeroing.
- [ ] Banded perturbation adaptive rows and `BayerBytes` threads gate.

Maintenance:

- [ ] Shared resize anchors, coordinate mapping and band assertions.
- [ ] Merge boundary traits and benchmark loops.
- [ ] Stale docs.

Validation:

- [ ] Focused native tests and frozen-spec guard per item.
- [ ] Native paired prod-prod runs for selected kernels.
- [ ] Browser qualification for selected changes.

## Notes

- Worktree `.worktrees/perf-211-final-pass`, branch `perf/211-final-pass`, base `90bf7c4b`. Root checkout stays untouched.
- Browser harness: `/home/mia/mia-cx/ditherette-performance-20260920/celeste` (`CELESTE_PROTOCOL=qualify`, six rounds, optimizing Firefox). Evidence stays outside Git.
- Band-only items (resize band scratch, banded perturb) run only with `threads`; the scalar browser qualification cannot reach them.
