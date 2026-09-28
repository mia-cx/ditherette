# Ditherette design

Ditherette turns an image into palette-limited pixel art in the browser. The interface is an editor studio: the preview owns the screen, and every processing stage is a window around it.

## Layout

- **Menu bar.** The top bar is the "ditherette" wordmark and a desktop-style menu bar: File (open, export, clear), Edit (crop), Image (adjustments, image size, resample, dither, color space), View (compare mode, zoom, theme), and Window (studio windows, scopes, reset layout). Every command already exists elsewhere in the UI; the menus gather them where editor users look. Command shortcuts use ⌘ on Apple devices and Ctrl elsewhere; canvas zoom uses bare `+`, `−`, `0`, and `1`, so browser zoom keeps working.
- **Studio, 1024px and wider.** Windows dock into splits and tabs, float, and dock back. The default layout puts the processing stages on the left (Dimensions and Dither as tabs above Palette and Color space), Preview in the middle, and Effects on the right. The side columns take 28% and 20% of the width, within 300–400px and 240–300px, so the preview keeps the rest.
- **Effect windows.** Each effect instance opens its own window, titled with the instance name. New effect windows join an open effect window as tabs, or open floating in the middle of the dock when none is open.
- **Floating.** The button in each tab bar floats the active window at 360×520 or docks a floating group back on the right edge. The tab context menu offers the same action plus maximize and close.
- **Collapsing.** Every docked column except the preview's is a sidebar, and the window at its top carries a sidebar button: it folds the whole column into a strip of vertical tabs, split evenly, and opens it again. A window that shares its column rolls up to its tab bar with its caret; a floating window rolls up to its title. Windows rolled up before a sidebar folds stay rolled up when it opens. Other side columns keep their widths while a sidebar folds or opens, so the preview takes up the difference. Collapse state persists with the layout, and Reset layout expands everything.
- **Persistence.** The layout saves to `localStorage` on every change and restores on load. Window > Reset layout rebuilds the default. Closing every window shows a Reset layout button.
- **Below 1024px.** The same controls stack: preview first, then an accordion of Effects, Dimensions, Dither, Color space, and Scopes, then Palette. Effect rows expand in place instead of opening windows.
- **Export** stays in a bar under the workspace at every width.

## Pipeline order

The Effects list reads top to bottom in run order: effects run on the source, then resize, then dither and quantize. Rows drag to reorder; the row menu offers Move up and Move down for keyboard use. The eye toggles an effect without removing it, like palette colors. Double-click a name, or choose Rename, to rename an instance. A renamed row also shows its effect type.

## Style

- **Type.** Geist Variable for everything. Numbers in fields use the monospace stack with tabular figures.
- **Shape.** Square corners throughout (`--radius: 0`), matching the pixel output.
- **Color.** Zinc neutrals with the yellow primary (`--primary`). The primary marks the active window's tab, sash hover, drop targets, and enabled toggles.
- **Dock theme.** `src/lib/components/dock/dock.css` maps every Dockview variable to an app token, so light and dark themes follow the app's `.dark` class. Floating windows sit at z-index 20, under menus and popovers (z-50).
- **Editors.** Sliders pair with a number field; the field shows the stored value after clamping. Levels reads in 0–255 byte units. Percent fields show -100–100% for arguments stored as -1–1. The curve editor snaps points to the 0–255 grid and draws the same monotone spline the Wasm effect applies.
- **Effects on the source.** The preview toolbar's sliders button, or View > Show effects on source, draws the Source half with the enabled effects applied, before resize and palette. Every effect maps colours independently, so a worker compiles the chain into a 52³ lookup table by running the real effects over a colour lattice, and WebGL2 draws the full-resolution source through it. Interpolation keeps the mean error near 0.15 bytes; exports always use the exact pipeline. The worker, the drawing, and the eight most recent tables stay until the image changes, so turning the preview or an effect back on reuses them.
- **Visuals.** Levels, Brightness and contrast, and Exposure plot their tone response over the dashed identity line. Hue and saturation shows the hue circle before and after the step. Hue, saturation, lightness, temperature, and tint sliders draw what they do as their track.
- **Curves.** A curves layer picks its model: RGB, linear RGB, HSL, HSV, OKLab, OKLCH, CIELAB, CIELCh, YCbCr, or Arbitrary XY, and switching starts the curves over. In a model, each channel keeps its own curve; checkboxes named after the model's channels, each in its own colour, pick which ones an edit writes to, starting on all three for RGB and on the lightness channel otherwise. Channels whose curve differs are drawn thin behind. Arbitrary XY reads one channel of any model on x and adjusts one on y; its neutral line is flat at the midpoint, and a hue x axis shows that model's spectrum and wraps, with end points fixed at 0 and 1 sharing one y.

## Scopes

Window > Scopes shows a histogram, waveform, parade, vectorscope, or CIE 1931 chromaticity diagram of the source, the output, or both. It reads any curves model plus CMY, on the same normalised axes as the curves editor.

- **Always dark.** Scopes read like a grading monitor in either theme: density glows on black, the graticule stays at 6–25% white, and each channel traces in its own colour. With both images, the source glows dim and neutral under the output.
- **Vectorscope.** It uses the model's chroma plane, or YCbCr's for the RGB models. It draws 100% target boxes, 75% dots and a skin-tone line, and has a 2× magnifier for low-saturation images.
- **Chromaticity.** The xy diagram draws the spectral locus from the CIE 1931 table, filled faintly with its own colours, plus the sRGB gamut and D65.
- **Sampling.** Scopes sample up to 262,144 pixels on a jittered grid, so a regular dither pattern can't alias. Output colours come from the processed indices, and canvases are only drawn to.

## Accessibility

- Every effect control has a visible label and a matching accessible name.
- Curve points are focusable: arrow keys move one step, Shift moves 16, Delete removes. The Input and Output fields edit the selected point.
- Menus open with Enter or Space and move with the arrow keys. The Window menu opens and closes every fixed window, so no window depends on a pointer-only close button.
- Bare view keys never fire while focus is in a text field.
