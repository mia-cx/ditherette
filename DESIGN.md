# Ditherette design

Ditherette turns an image into palette-limited pixel art in the browser. The interface is an editor studio: the preview owns the screen, and every processing stage is a window around it.

## Layout

- **Menu bar.** The top bar is the "ditherette" wordmark and a desktop-style menu bar: File (open, export, clear), Edit (crop), Image (adjustments, image size, resample, dither, color space), View (compare mode, zoom, theme), and Window (studio windows, reset layout). Every command already exists elsewhere in the UI; the menus gather them where editor users look. Command shortcuts use ⌘ on Apple devices and Ctrl elsewhere; canvas zoom uses bare `+`, `−`, `0`, and `1`, so browser zoom keeps working.
- **Studio, 1024px and wider.** Windows dock into splits and tabs, float, and dock back. The default layout puts the processing stages on the left (Dimensions and Dither as tabs above Palette and Color space), Preview in the middle, and Effects on the right. The side columns take 28% and 20% of the width, within 300–400px and 240–300px, so the preview keeps the rest.
- **Effect windows.** Each effect instance opens its own window, titled with the instance name. New effect windows join the other effect windows as tabs, or dock below the Effects list when none is open.
- **Floating.** The button in each tab bar floats the active window at 360×520 or docks a floating group back on the right edge. The tab context menu offers the same action plus maximize and close.
- **Collapsing.** The caret in each tab bar collapses a window and expands it back to its previous size. A window with a neighbour above or below rolls up to its tab bar; a window alone in its column folds into a narrow strip of vertical tabs; a floating window rolls up to its title. Collapse state persists with the layout, and Reset layout expands everything.
- **Persistence.** The layout saves to `localStorage` on every change and restores on load. Window > Reset layout rebuilds the default. Closing every window shows a Reset layout button.
- **Below 1024px.** The same controls stack: preview first, then an accordion of Effects, Dimensions, Dither, and Color space, then Palette. Effect rows expand in place instead of opening windows.
- **Export** stays in a bar under the workspace at every width.

## Pipeline order

The Effects list reads top to bottom in run order: effects run on the source, then resize, then dither and quantize. Rows drag to reorder; the row menu offers Move up and Move down for keyboard use. The eye toggles an effect without removing it, like palette colors. Double-click a name, or choose Rename, to rename an instance. A renamed row also shows its effect type.

## Style

- **Type.** Geist Variable for everything. Numbers in fields use the monospace stack with tabular figures.
- **Shape.** Square corners throughout (`--radius: 0`), matching the pixel output.
- **Color.** Zinc neutrals with the yellow primary (`--primary`). The primary marks the active window's tab, sash hover, drop targets, and enabled toggles.
- **Dock theme.** `src/lib/components/dock/dock.css` maps every Dockview variable to an app token, so light and dark themes follow the app's `.dark` class. Floating windows sit at z-index 20, under menus and popovers (z-50).
- **Editors.** Sliders pair with a number field; the field shows the stored value after clamping. Levels reads in 0–255 byte units. Percent fields show -100–100% for arguments stored as -1–1. The curve editor snaps points to the 0–255 grid and draws the same monotone spline the Wasm effect applies.
- **Visuals.** Levels, Brightness and contrast, and Exposure plot their tone response over the dashed identity line. Hue and saturation shows the hue circle before and after the step. Hue, saturation, lightness, temperature, and tint sliders draw what they do as their track.
- **Curves.** Each Curves instance keeps a curve per channel. The Red, Green, and Blue checkboxes, all checked by default, pick which channels an edit writes to; the edit starts from the first checked channel's curve. Channels whose curve differs are drawn thin in their colour behind it. Matching curves run as one RGB step.

## Accessibility

- Every effect control has a visible label and a matching accessible name.
- Curve points are focusable: arrow keys move one step, Shift moves 16, Delete removes. The Input and Output fields edit the selected point.
- Menus open with Enter or Space and move with the arrow keys. The Window menu opens and closes every fixed window, so no window depends on a pointer-only close button.
- Bare view keys never fire while focus is in a text field.
