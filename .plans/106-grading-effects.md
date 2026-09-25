# #106 Add composable filters and colour-grading effects

## Summary

Stacked on #202's pipeline (PR #222). Add the initial grading inventory as built-in effects, each with
a readable reference in `spec/effects/`, a frozen extension record, and a production copy optimized
against benchmarks. Chains, ordering, repetition, bypass, and serialization already come from #202.

## Acceptance criteria

- [x] Initial inventory settled and documented: levels (from #202), curves, brightness-contrast, exposure, white-balance, hue-saturation.
- [x] Each effect defines precision, working space, argument domains, neutral values, and alpha handling.
- [x] Recipes serialize deterministically and reproduce results; presets are just stored recipes.
- [x] Package/website boundary documented: the package owns effect semantics; the website owns editor state and presets.
- [x] Production equals the reference byte-for-byte; costs measured.

## TODOs

- [x] Prod trait: replace `channel_map` with `per_channel` + `map_channel` so effects with different per-channel maps tabulate.
- [x] Spec: continuous colour-space helpers (`space.rs`) and the five effects with docs.
- [x] Spec tests: formulas, neutral identity, monotone curves, hue rotation, validation paths.
- [x] Record the grading extension in the freeze checkpoint.
- [x] Prod: copy the effects, conformance tests.
- [x] Prod: benchmark, then optimize with evidence.
- [x] Package: types, validation, docs, tests, changeset.
- [x] Final validation.

## Notes
- Hue-saturation works in Oklab via an a/b rotation; lightness blends the whole Oklab colour toward white or black.
- Curves use Fritsch–Butland monotone cubic tangents; knots evaluate exactly.
- The freeze syntax guard rejects the token `path` inside macro calls, so registry dispatch uses explicit match arms.
