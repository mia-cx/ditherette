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

- [ ] Full-workspace, preview-centred layout on large screens.
- [ ] Dimensions, Dither, Color space, Palette, and Pipeline are separate windows.
- [ ] Windows dock into splits and tabs, float, and dock back; floating windows move and resize.
- [ ] Every effect instance opens its own window; two instances of one effect get two windows.
- [ ] Effect instances can be renamed; the window title follows the name.
- [ ] Layout persists across reloads; a reset action restores the default docking.
- [ ] Upload, processing, preview, and export still work; effects run through recipe v2.
- [ ] Small screens get a stacked fallback with the same controls.
- [ ] New panels register once; window chrome is not duplicated.

## TODOs

- [ ] Effect layer store: persisted instances with id, name, and step; add, remove, move, rename, update; neutral defaults; unit tests.
- [ ] Processing: send enabled steps to the worker, hash them, build recipe v2 when any exist; worker schema and adapter tests.
- [ ] Effect editors: shared slider field, levels, curves (spline editor), brightness-contrast, exposure, white balance, hue-saturation, recolour strength.
- [ ] Pipeline panel: add menu, reorder, enable, rename, open, remove; inline editors when no dock exists.
- [ ] Dock workspace: dockview-core with vendored core CSS and a token theme, panel registry, default layout, persistence, reset, float/dock action, title sync.
- [ ] Page: studio on large screens, stacked fallback below; Windows menu in the app bar.
- [ ] DESIGN.md: record the studio's layout, tokens, and interaction choices.
- [ ] Browser tests for docking, renaming, and persistence; render and inspect both widths and themes.

## Notes

- dockview-core 8.3.1 does not publish its stylesheet. The core rules (themes removed) are copied from the `dockview` package into `src/lib/components/dock/dockview-core.css`.
- Popout browser windows are out: bits-ui overlays portal into the main document, so selects and popovers would break in a popout.
- Editable recolour recipes (tone, chroma, shift, hue groups) need an analysis round trip through the worker; the first version exposes strength and automatic fitting.
