# #20 Add undo/redo keybinds for fast A/B setting toggles

## Summary

Keyboard undo and redo for processing settings, so recent adjustments can be A/B compared. History holds small settings snapshots only. The worker's cache is separate and stays bounded on its own.

## Acceptance criteria

- [x] Cmd/Ctrl+Z undoes and Cmd/Ctrl+Shift+Z (or Ctrl+Y off macOS) redoes a committed settings edit.
- [x] Output, dither, colour space, effects, active palette, palette colours and enabled state are covered.
- [x] Undo and redo reprocess through the normal auto-processing path, without the slider debounce.
- [x] A slider drag is one entry, not one per pointermove tick.
- [x] Text fields keep native undo. Sliders, checkboxes and the page itself use the app's undo.
- [x] Preview pan and zoom are not part of history.
- [x] Source and crop changes reset history, so undo never shows a stale frame.
- [x] History is bounded and holds no image buffers.

## TODOs

- [x] History store: snapshot, coalesce and undo/redo the tracked settings, with unit tests.
- [x] Key handling and Edit menu items, with a browser test.
- [x] Start history from the page; run checks.

## Notes

- Snapshots share references with the settings atoms, which the app never mutates. A 100-entry cap bounds memory.
- Changes in one tick form one entry. Changes to the same fields merge until 400 ms after the last one, or until the pointer is released, so one drag is one entry.
- Crop and source changes start a fresh history rather than being undoable.
- The package's stage cache lives in the worker and is keyed by content. History does not hold or evict it.
- Validation: `vitest --run` (208 tests, including history and AppBar keybind browser tests), `pnpm check` (0 errors), eslint and prettier on changed files. Repo-wide `prettier --check` already fails on untouched files. Headless Chromium against a dev server: Image > Dither change, Ctrl+Z restored `none`, Ctrl+Shift+Z restored the choice. The user's Chromium CDP (9228) was unreachable, so no visual check there.
