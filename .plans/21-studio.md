# #21 Overhaul UI into image-editor workspace

## Summary

Turn the page-with-controls layout into an editor studio. The preview owns the screen. Every
processing stage is a window that docks, tabs, floats, and docks back: Dimensions, Dither, Color
space, Palette, and the effect Pipeline. Each effect instance in the pipeline opens its own window,
so two Levels steps get two windows. Instances are renamable, and the window title follows the name.
The layout has a default dock arrangement, persists across reloads, and resets on demand.

Mia widened the issue on 2026-09-27: docking is in scope (the issue deferred it), and the effects
chain from #202/#106/#201 is wired into the website so the pipeline has something to show.
This branch stacks on #224.

## Acceptance criteria

- [x] Full-workspace, preview-centred layout on large screens.
- [x] Dimensions, Dither, Color space, Palette, and Pipeline are separate windows.
- [x] Windows dock into splits and tabs, float, and dock back; floating windows move and resize.
- [x] Every effect instance opens its own window; two instances of one effect get two windows.
- [x] Effect instances can be renamed; the window title follows the name.
- [x] Layout persists across reloads; a reset action restores the default docking.
- [x] Upload, processing, preview, and export still work; effects run through recipe v2.
- [x] Small screens get a stacked fallback with the same controls.
- [x] New panels register once; window chrome is not duplicated.

## TODOs

- [x] Effect layer store: persisted instances with id, name, and step; add, remove, move, rename, update; neutral defaults; unit tests.
- [x] Processing: send enabled steps to the worker, hash them, build recipe v2 when any exist; worker schema and adapter tests.
- [x] Effect editors: shared slider field, levels, curves (spline editor), brightness-contrast, exposure, white balance, hue-saturation, recolour strength.
- [x] Pipeline panel: add menu, reorder, enable, rename, open, remove; inline editors when no dock exists.
- [x] Dock workspace: dockview-core with vendored core CSS and a token theme, panel registry, default layout, persistence, reset, float/dock action, title sync.
- [x] Page: studio on large screens, stacked fallback below; Windows menu in the app bar.
- [x] DESIGN.md: record the studio's layout, tokens, and interaction choices.
- [x] Browser tests for docking, renaming, and persistence; render and inspect both widths and themes.

## Notes

- dockview-core 8.3.1 does not publish its stylesheet. The core rules (themes removed) are copied from the `dockview` package into `src/lib/components/dock/dockview-core.css`.
- Popout browser windows are out: bits-ui overlays portal into the main document, so selects and popovers would break in a popout.
- Editable recolour recipes (tone, chroma, shift, hue groups) need an analysis round trip through the worker; the first version exposes strength and automatic fitting.
- dockview's keyboard navigation module is enterprise-only, so tabs rely on pointer and the Windows menu; panel content stays in DOM tab order.
- The dev server could not load `packages/ditherette` (Vite `fs.allow` 403, surfacing as "Wasm could not initialize"); fixed in `vite.config.ts`.
- Validation: `pnpm check` (0 errors), `pnpm exec vitest run` (21 files, 150 tests), `pnpm build:web`, `pnpm exec playwright test` (1 passed), ESLint and Prettier on changed files.
- Rendered with headless Chromium at 1440×900 (light and dark), 1024×768, and 390×844: upload, add, rename, float, curve drag, reload restore, export enabled.
