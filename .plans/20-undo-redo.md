# #20 Add undo/redo keybinds for fast A/B setting toggles

## Summary

Keyboard undo and redo for processing settings, so users can flip between recent adjustments. History holds settings only, never images; the worker's bounded stage cache serves repeat toggles on its own.

## Acceptance criteria

- [ ] ⌘Z / ⇧⌘Z on macOS and Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y elsewhere undo and redo settings.
- [ ] Undo and redo set the stores, so processing reschedules through the existing subscriptions.
- [ ] One drag or burst of edits is one history entry; pointermove ticks never add entries.
- [ ] History covers output, dither, colour space, active palette, palette enabled state, custom palettes, and effect layers.
- [ ] Preview pan, zoom, and mode stay out of history.
- [ ] A new source or a crop change starts a fresh history, so undo never pairs a size with the wrong frame.
- [ ] Focused text fields keep native undo.
- [ ] History is bounded and holds no image buffers.

## TODOs

- [x] Add `src/lib/stores/history.ts`: snapshots of the tracked stores, a settle rule that commits one entry once no pointer is held and settings stay still, bounded undo/redo stacks, and source/crop boundaries. Browser spec covers coalescing, undo/redo, redo clearing, boundaries, and the bound.
- [x] Start history with the page, add Undo and Redo to the Edit menu, and bind the shortcuts in the app bar's key handler. AppBar spec covers the shortcuts and the text-field exemption.
- [ ] Document the feature in `DESIGN.md`, add a changeset, run checks, and verify in Chromium.

## Notes

- TODO 1: `pnpm vitest --run --project client src/lib/stores/history.browser.spec.ts` passes (5 tests); `pnpm check` 0 errors. Needed `pnpm package:build` first so `ditherette` resolves.
- TODO 2: AppBar + history specs pass together (10 tests); `pnpm check` 0 errors. Prettier also reflowed one Adjustments menu item that was unformatted on main.
