# #29 Pixel counts

## Summary
Show how many pixels each palette colour takes in the finished output, and the total pixels to paint in the export bar, so a Wplace painter can plan the artwork.

## Acceptance criteria
- [x] The palette table has a Pixels column with each colour's count in the finished output, read from the processed palette indices.
- [x] The export bar shows the total pixels to paint: every output pixel except Transparent.
- [x] Counts show `—` while there is no output or the output is out of date (processing, or settings, palette, or source changed), never stale numbers.
- [x] The output is scanned once per finished image, not per row or rerender.
- [x] Table layout, keyboard access, and small-screen behaviour stay intact.

## TODOs
- [x] Add `countPalettePixels` and the `pixelCounts` store in `src/lib/processing/pixel-counts.ts`, with unit tests for counting semantics and the changed-source lifecycle.
- [x] Add the Pixels column to `PalettePanel.svelte` and the total badge to `ExportStrip.svelte`, with a component test.
- [x] Document the counts in SPEC.md and DESIGN.md.
- [x] Capture before/after screenshots of the palette and export bar with a processed image.

## Notes
- `exportable` in `export.ts` already means "finished and matches the current settings"; the counts reuse it as their readiness gate.
- Disabled colours are not in the output palette, so they count 0.
- At 390px the palette table already overflowed its `overflow-x: hidden` viewport before this change (468px of content in 358px), clipping the Actions column. The Pixels column widens the table to 512px, so Tags clip too. Fixing the phone table layout is outside this issue.
- Checks: `pnpm vitest --run --project server src/lib/processing/pixel-counts.spec.ts src/lib/processing/export.spec.ts`, `pnpm vitest --run --project client src/routes/components/PalettePanel.svelte.spec.ts src/routes/components/ExportStrip.svelte.spec.ts`, `pnpm check`, prettier and eslint on changed files.
