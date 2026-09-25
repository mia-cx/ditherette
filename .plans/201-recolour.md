# #201 Add image- and palette-aware recolouring to the Rust/Wasm crate

## Summary

Stacked on #106 (PR #223) and #202 (PR #222). A `recolour` effect fits an image to the colours an
enabled palette can represent, directly or through dithered mixtures, while keeping continuous colour.
Analysis (`image + palette + space → recipe`) is separate from application
(`image + recipe + strength → image`). The recipe is plain, editable data. Inside a chain, a step with no
recipe analyses the image reaching it; production caches that analysis by the chain prefix, so edits
after it never re-analyse.

## Acceptance criteria

- [ ] Public Rust/Wasm operations for standalone analysis and application, no website dependency.
- [ ] Fixtures show analysis responding to image content, palette (custom, full vs restricted), and working space.
- [ ] Recipes inspect, edit, and reapply deterministically; zero strength preserves the input.
- [ ] Gradients keep intermediate colours before quantization; works with every dither family in the same space.
- [ ] Treated vs untreated comparison across images and palettes (detail, separation, shifts, texture); analysis and application costs recorded separately.
- [ ] #202 criterion 4: recolouring composes with earlier effects, respects the enabled palette, and downstream edits reuse the analysis.

## TODOs

- [x] Spec: lightness–opponent coordinates for every working space.
- [x] Spec: recipe, application, strength, and context checks (`recolour.rs`).
- [x] Spec: deterministic analysis (`recolour_analysis.rs`) and standalone `analyze_recolour`.
- [x] Spec tests.
- [x] Record the recolour extension in the freeze checkpoint.
- [x] Prod: copy, conformance tests.
- [x] Prod: analysis cache keyed by chain prefix; processor + Wasm `privateAnalyzeRecolour`.
- [ ] Prod: benchmark and optimize with evidence.
- [ ] Package: `analyzeRecolour`, `recolour` effect type, validation, docs, tests, changeset.
- [ ] Evaluation: treated vs untreated metrics across fixtures and palettes; record costs.
- [ ] Final validation.

## Notes
