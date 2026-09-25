# #106 Add composable filters and colour-grading effects

## Summary

Stacked on #202's pipeline (PR #222). Add the initial grading inventory as built-in effects, each with
a readable reference in `spec/effects/`, a frozen extension record, and a production copy optimized
against benchmarks. Chains, ordering, repetition, bypass, and serialization already come from #202.

## Acceptance criteria

- [ ] Initial inventory settled and documented: levels (from #202), curves, brightness-contrast, exposure, white-balance, hue-saturation.
- [ ] Each effect defines precision, working space, argument domains, neutral values, and alpha handling.
- [ ] Recipes serialize deterministically and reproduce results; presets are just stored recipes.
- [ ] Package/website boundary documented: the package owns effect semantics; the website owns editor state and presets.
- [ ] Production equals the reference byte-for-byte; costs measured.

## TODOs

- [x] Prod trait: replace `channel_map` with `per_channel` + `map_channel` so effects with different per-channel maps tabulate.
- [x] Spec: continuous colour-space helpers (`space.rs`) and the five effects with docs.
- [x] Spec tests: formulas, neutral identity, monotone curves, hue rotation, validation paths.
- [x] Record the grading extension in the freeze checkpoint.
- [ ] Prod: copy the effects, conformance tests.
- [ ] Prod: benchmark, then optimize with evidence.
- [ ] Package: types, validation, docs, tests, changeset.
- [ ] Final validation.

## Notes
