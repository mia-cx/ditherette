# Tiling sweep lessons

## What went wrong with nearest

- The row-range nearest adapter was not equivalent to the scalar nearest dispatcher. Scalar nearest used same-width row copy, exact downscale, exact upscale row repeat, near-identity span-copy, and generic word-copy; the row-range path mostly measured generic mapped-row word-copy plus tiling overhead.
- The sweep treated every dimension as if it should choose a tiled configuration. Old nearest production tiling was gated and intentionally fell back to scalar for tiny outputs, identity/same-width, strong exact integer downscales, near-identity minify, and one-band plans.
- Curve fitting used the least-bad tiled row even when scalar won. Curves must be fitted only from profitable tiled rows; otherwise the generated policy models slowdown.
- The old nearest policy was filter-specific. It used nearest-specific thresholds and worker caps, not generic row-band curves.

## Rules for adding other filters

1. Row-range adapters must preserve scalar dispatch semantics before their timings are trusted.
2. Scalar fallback is a valid policy decision. Best-case reports and curve fitting must distinguish `scalar` from `tiled`.
3. Fit production curves only from rows where tiled beats scalar beyond noise.
4. Keep filter-specific gates near filter adapters; keep `prod::tiling` generic and limited to worker budgets, geometry, assignment, and executor plumbing.
5. Run one filter at a time and inspect sweep output before using curves for production defaults.
