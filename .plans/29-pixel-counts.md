# #29 Pixel counts

## Summary
Show how many pixels each palette colour takes in the finished output, and the total pixels to paint in the export bar, so a Wplace painter can plan the artwork.

## Acceptance criteria
- [ ] The palette table has a Pixels column with each colour's count in the finished output, read from the processed palette indices.
- [ ] The export bar shows the total pixels to paint: every output pixel except Transparent.
- [ ] Counts show `—` while there is no output or the output is out of date (processing, or settings, palette, or source changed), never stale numbers.
- [ ] The output is scanned once per finished image, not per row or rerender.
- [ ] Table layout, keyboard access, and small-screen behaviour stay intact.

## TODOs
- [x] Add `countPalettePixels` and the `pixelCounts` store in `src/lib/processing/pixel-counts.ts`, with unit tests for counting semantics and the changed-source lifecycle.
- [x] Add the Pixels column to `PalettePanel.svelte` and the total badge to `ExportStrip.svelte`, with a component test.
- [ ] Document the counts in SPEC.md and DESIGN.md.
- [ ] Capture before/after screenshots of the palette and export bar with a processed image.

## Notes
- `exportable` in `export.ts` already means "finished and matches the current settings"; the counts reuse it as their readiness gate.
- Disabled colours are not in the output palette, so they count 0.
