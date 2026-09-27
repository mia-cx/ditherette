# #201 Add image- and palette-aware recolouring to the Rust/Wasm crate

## Summary

Stacked on #106 (PR #223) and #202 (PR #222). A `recolour` effect fits an image to the colours an
enabled palette can represent, directly or through dithered mixtures, while keeping continuous colour.
Analysis (`image + palette + space → recipe`) is separate from application
(`image + recipe + strength → image`). The recipe is plain, editable data. Inside a chain, a step with no
recipe analyses the image reaching it; production caches that analysis by the chain prefix, so edits
after it never re-analyse.

## Acceptance criteria

- [x] Public Rust/Wasm operations for standalone analysis and application, no website dependency.
- [x] Fixtures show analysis responding to image content, palette (custom, full vs restricted), and working space.
- [x] Recipes inspect, edit, and reapply deterministically; zero strength preserves the input.
- [x] Gradients keep intermediate colours before quantization; works with every dither family in the same space.
- [x] Treated vs untreated comparison across images and palettes (detail, separation, shifts, texture); analysis and application costs recorded separately.
- [x] #202 criterion 4: recolouring composes with earlier effects, respects the enabled palette, and downstream edits reuse the analysis.

## TODOs

- [x] Spec: lightness–opponent coordinates for every working space.
- [x] Spec: recipe, application, strength, and context checks (`recolour.rs`).
- [x] Spec: deterministic analysis (`recolour_analysis.rs`) and standalone `analyze_recolour`.
- [x] Spec tests.
- [x] Record the recolour extension in the freeze checkpoint.
- [x] Prod: copy, conformance tests.
- [x] Prod: analysis cache keyed by chain prefix; processor + Wasm `privateAnalyzeRecolour`.
- [x] Prod: benchmark and optimize with evidence.
- [x] Package: `analyzeRecolour`, `recolour` effect type, validation, docs, tests, changeset.
- [x] Evaluation: treated vs untreated metrics across fixtures and palettes; record costs.
- [x] Final validation.

## Notes
- Palette reach is the support function of the palette's convex hull in the opponent plane: what dithered mixtures can average to.
- First evaluation showed full tone stretches moved every image; treatment now compresses only out-of-range tones and scales redistribution by palette sparsity.
- Analysis cache keys hash exactly what analysis reads, so hits work across applyEffects, process, and analyzeRecolour.
- Built-in chains resolve recipe-less recolour steps first, then memoize the whole pointwise chain per input colour.
