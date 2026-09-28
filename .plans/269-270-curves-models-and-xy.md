# #269 Switch a curves layer between colour models · #270 Edit arbitrary XY curves

## Summary
A curves layer picks its colour model: RGB (the existing `curves` steps), linear RGB, HSL, HSV,
Oklab, OKLCH, CIELAB, CIELCh, YCbCr (the `model-curves` effect from #277), or Arbitrary XY (the
`channel-curve` effect from #278). The channel checkboxes follow the model. Arbitrary XY picks an
input channel for x and an output channel for y from any model, draws a neutral flat line, and
wraps when x is a hue.

## Acceptance criteria
- [ ] A curves layer switches between every model and Arbitrary XY; switching resets the curves.
- [ ] Channel checkboxes use the model's channel names and colours, and all curves draw together.
- [x] RGB still emits `curves` steps; other models emit one `model-curves` step; Arbitrary XY emits one `channel-curve` step.
- [ ] Arbitrary XY: X and Y channel pickers, a flat neutral line at 0.5, and a hue spectrum under a hue x axis.
- [ ] A hue x axis wraps: the end points stay at 0 and 1 with the same y, and interior points dragged past one edge continue at the other.
- [x] Saved layers the package would reject are dropped on load.

## TODOs
- [x] Layer model: `model` on the curves layer step, channel metadata per model, `packageSteps` and load validation for every model, with tests.
- [x] Periodic spline: `evaluatePeriodicCurve` matching the reference's cyclic Fritsch–Butland spline, with tests.
- [ ] Model switcher and per-model channel checkboxes in the curves editor.
- [ ] Arbitrary XY editor: channel pickers, flat neutral line, hue spectrum, wrapping hue x axis.
- [ ] Docs and changeset.
- [ ] Validation: `pnpm check`, `vitest run`, ESLint, and a rendered check in Chromium.

## Notes
- The web stack (#235 → … → #281) and the curve effects chain (#247 → #277 → #278) both sit on
  main, so this branch merges the chain into the web stack top. The PR diff includes the chain
  until those merge.
- The package's `dist` is gitignored; it was rebuilt locally with the repo's wasm-pack.
- TODO 1: curves layers carry `model`; validation runs the package's `isEffect` on every step a layer turns into. `vitest run src/lib/stores/effects.spec.ts` (7) and `pnpm check` pass. The XY editor branch is a placeholder until TODO 4.
- TODO 2: `evaluatePeriodicCurve` mirrors `PeriodicSpline` in the reference; `evaluateCurve` now shares its tangent helper. `vitest run src/lib/effects/spline.spec.ts` (5) passes.
