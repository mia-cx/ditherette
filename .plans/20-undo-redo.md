# #20 Add undo/redo keybinds for fast A/B setting toggles

## Summary

Keyboard undo and redo for processing settings, so users can flip between recent adjustments. History holds settings only, never images; the worker's bounded stage cache serves repeat toggles on its own.

## Acceptance criteria

- [x] ⌘Z / ⇧⌘Z on macOS and Ctrl+Z / Ctrl+Shift+Z / Ctrl+Y elsewhere undo and redo settings.
- [x] Undo and redo set the stores, so processing reschedules through the existing subscriptions.
- [x] One drag or burst of edits is one history entry; pointermove ticks never add entries.
- [x] History covers output, dither, colour space, active palette, palette enabled state, custom palettes, and effect layers.
- [x] Preview pan, zoom, and mode stay out of history.
- [x] A new source or a crop change starts a fresh history, so undo never pairs a size with the wrong frame.
- [x] Focused text fields keep native undo.
- [x] History is bounded and holds no image buffers.

## TODOs

- [x] Add `src/lib/stores/history.ts`: snapshots of the tracked stores, a settle rule that commits one entry once no pointer is held and settings stay still, bounded undo/redo stacks, and source/crop boundaries. Browser spec covers coalescing, undo/redo, redo clearing, boundaries, and the bound.
- [x] Start history with the page, add Undo and Redo to the Edit menu, and bind the shortcuts in the app bar's key handler. AppBar spec covers the shortcuts and the text-field exemption.
- [x] Document the feature in `DESIGN.md`, add a changeset, run checks, and verify in Chromium.

## Notes

- TODO 1: `pnpm vitest --run --project client src/lib/stores/history.browser.spec.ts` passes (5 tests); `pnpm check` 0 errors. Needed `pnpm package:build` first so `ditherette` resolves.
- TODO 2: AppBar + history specs pass together (10 tests); `pnpm check` 0 errors. Prettier also reflowed one Adjustments menu item that was unformatted on main.
- TODO 3: `pnpm test:unit --run` 28 files, 197 tests pass (a first run failed only on Vite's dependency-optimisation reload); `pnpm check` 0 errors; eslint and prettier clean on touched files.
- The user's Chromium CDP on 127.0.0.1:9228 was not listening, so live QA ran in headless Playwright Chromium against `vite dev --port 5188`: a 30-tick dither drag with a 600 ms mid-drag pause undid in one step; menu-chosen algorithm undid next; Ctrl+Shift+Z and Ctrl+Y redid; loading an image left Undo disabled; a scale drag undid and redid to the same size with the output reprocessed.
- Follow-up (Sonnet #317 strengths and Pullfrog thread PRRT_kwDOSWTOKc6m-TLy): store changes settle once per microtask burst and name the controls they touched. Quick repeats to one control merge, another control commits the pending edit, and effect layers compare by id. Pressed pointer ids hold a gesture open until the last lifts, and blur releases them. Restores call `scheduleProcessing(0)`. The shortcut respects `defaultPrevented`, keeps native undo in text/number inputs and contenteditable descendants, lets undo pass other controls, and ignores Ctrl+Shift+Y. Baseline at a7257567: 197 unit pass, check clean, build passes; lint fails on main only (Prettier: 513 files, ESLint: 7 errors in 4 files, all outside this PR). After: 201 unit pass, check clean, build passes, parent's external probes 12/12 (from 11/12).
