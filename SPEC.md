# Ditherette Spec

## Goal

Build a fully client-side SvelteKit app that converts a user-selected image into a resized, palette-constrained image. The user can tune dimensions, color matching math, palette contents, and optional dithering, then export the result without uploading the image or calling a backend.

## Non-goals

- No server-side image processing, persistence, auth, or file uploads.
- No AI/image-generation features.
- No project/gallery sharing in the first version.
- No automatic scraping of palette sites at runtime; built-in palettes are bundled source data.

## Product shape

Ditherette is a mobile-first single-page image workbench at `/` for the MVP. The spec prescribes layout and interaction. Accessibility, semantic Svelte/HTML, and maintainability are primary implementation priorities. Visual identity comes from the existing shadcn-svelte components in `src/lib/components/ui`; use those components as the mandatory style foundation rather than inventing a separate design system.

1. A compact top app bar with brand, local-processing reassurance, and the upload/replace image action.
2. One large plain source/output comparison preview at the top of the page.
3. Mobile-first controls stacked below the preview, expanding to a two-column control area on wider screens:
   - left column on desktop: dithering controls and color-space controls,
   - right column on desktop: palette preset selector and palette editor.
4. A bottom export area with output metadata and primary download action.

The preview is the hero. The educational mini-canvases and explanations are core product features. Specific aesthetics—dark mode, light mode, gradients, pixel branding, glassy panels, shadows, or accent colors—must not become a separate style direction; shadcn-svelte components and project theme tokens dictate the app’s style and identity.

## MVP scope summary

MVP includes:

- Single-route app at `/`.
- Mandatory shadcn-svelte UI components from `src/lib/components/ui`.
- Mobile-first layout with top hero compare preview and controls below.
- Upload/replace image, drag/drop, local-only processing, and indefinite local restore until user clears data.
- Side-by-side and A/B reveal preview modes; mobile defaults to A/B reveal.
- Shared zoom/pan, pointer scroll-to-zoom, touch pinch zoom, drag-to-pan, and crop mode from the preview toolbar.
- Manual crop, crop-to-content using current alpha threshold, locked-aspect output bounds, and resize algorithms: nearest, bilinear, Lanczos3, and area/box.
- Wplace default palette with 63 visible colors plus Transparent, immutable except enabled/disabled state.
- Custom palettes, palette grid/list modes, JSON import/export, duplicate prevention, and max 256 entries.
- Color matching modes: OKLab default, sRGB, linear RGB, CompuPhase weighted RGB, Rec.601 weighted RGB, Rec.709 weighted RGB, CIELAB/Lab76, and OKLCH.
- Dithering modes: none default, Bayer 2×2/4×4/8×8/16×16, Floyd–Steinberg, Sierra, Sierra Lite, and seeded random.
- Dither strength, simple MVP coverage modes, serpentine for error diffusion, and deterministic random seed with randomize action.
- Alpha modes live with the Transparent palette swatch: Preserve transparency default, Premultiplied, Matte; alpha threshold only in Preserve mode.
- Static educational canvas/explainer for each MVP color space and dithering algorithm.
- Worker-backed async processing orchestrated with Effect v4 beta, cancellation, progress, retry policy, and stale-output handling.
- Typed-array hot loops, exact deterministic output tests, and 8-bit indexed PNG export with `PLTE`/`tRNS`.

Future planned:

- `.wplace` template export with target coordinates and documented JSON schema.
- Histograms, gamma/dynamic range/levels/curves, and per-channel adjustments.
- Additional color spaces and priority-weighted matching modes.
- Additional dithering algorithms and fuller coverage/threshold controls.
- Palette gravity/weight controls and custom palette reordering.
- Uploaded-image dither preview samples and interactive color-space visualizers.
- Preview tile cache for huge outputs.
- 1/2/4-bit PNG optimization for tiny palettes.
- Worker pools, WASM SIMD, WebGL, and WebGPU acceleration after profiling.

## Primary user flow

1. User opens `/`. No other app routes are required for MVP.
2. User clicks **Upload Image** in the top app bar or drags an image onto the preview area.
3. App decodes the image in the browser and displays source metadata.
4. The upload button changes to **Replace Image** while an image is loaded.
5. Output width and height default to the source dimensions with aspect-ratio lock enabled.
6. User compares source and processed output in either:
   - **Side-by-side** mode, or
   - **A/B Reveal** mode with a draggable reveal slider.
7. User adjusts:
   - resize/output settings,
   - color distance/color-space math,
   - palette preset and active colors,
   - optional dithering algorithm and strength.
8. App updates the processed canvas preview.
9. User downloads or copies the converted PNG.

## Requirements

### Client-only behavior

- Use SvelteKit in browser mode for all image work.
- Do not add form actions, server endpoints, or remote processing.
- Use Canvas 2D APIs for decode, resize, pixel reads, visualizers, and export.
- Processing must tolerate Cloudflare Pages/Workers static deployment.
- Show a short reassurance near the app bar: “Everything runs in your browser” / “Your image never leaves your device.”

### Image input

- Accept PNG, JPEG, WebP, and GIF static first frame where browser decoding supports it.
- Support file picker and drag/drop on the preview region.
- Drag/drop behavior:
  - if no image is loaded, dropping an image uploads immediately,
  - if an image is already loaded, dropping a new image opens a confirmation dialog before replacing,
  - replacement keeps compatible settings and recalculates output dimensions from the new source within bounds.
- Reject non-image files with a clear UI error.
- Show source filename, file size, MIME type, natural dimensions, and aspect ratio.
- Revoke object URLs after replacement/removal.
- Top-right primary button label:
  - no image: **Upload Image**,
  - image loaded: **Replace Image**.
- Replacement keeps current conversion settings when possible, but output dimensions reset from the new source dimensions and are auto-fit within the output pixel bounds unless the user has pinned custom output dimensions.
- Source images may exceed output limits. Only requested output dimensions are bounded.

### Top comparison preview

The preview component is not inside a card. It is a large plain canvas comparison surface at the top of the page. Use only enough styling to make the two previews, divider, labels, and controls clear.

Required preview controls:

- Segmented toggle above or overlaying the preview:
  - **Side-by-side**,
  - **A/B Reveal**.
- Side-by-side mode:
  - source on the left,
  - processed output on the right,
  - center divider,
  - labels for “Source / Original” and “Output / Dithered / Processed”.
- A/B Reveal mode:
  - one stacked comparison surface,
  - original and processed canvases aligned exactly,
  - draggable horizontal reveal divider,
  - keyboard-operable slider semantics,
  - visible handle at the divider.
- Default preview mode:
  - mobile/narrow screens default to **A/B Reveal**,
  - desktop/wide screens default to **Side-by-side**.
- Touch interaction inside the preview:
  - pinch zoom zooms around the gesture midpoint,
  - one-finger drag pans when zoomed,
  - double tap/click may reset to fit,
  - preview gestures must not trap keyboard or screen-reader users.
- Pointer/mouse/trackpad interaction inside the preview:
  - scroll wheel / trackpad scroll zooms around the pointer location,
  - click-and-drag pans when zoomed,
  - double click may reset to fit.
- Preview zoom/pan state is shared between source and output:
  - side-by-side mode uses the same zoom level and equivalent pan position in both panes,
  - A/B reveal uses identical transforms for source and output so the reveal aligns exactly,
  - crop mode uses the same source transform state.
- Floating preview metadata/actions:
  - zoom level,
  - output dimensions,
  - active palette color count,
  - crop tool button,
  - optional fit/fullscreen button.
- Resizable preview height on desktop:
  - on wider screens (`lg+`), a draggable horizontal handle sits between the preview and the controls so users can rebalance vertical space between the hero preview and the control panels,
  - the handle is keyboard-operable: `ArrowUp`/`ArrowDown` resize in small steps, `Home`/`End` snap to min/max,
  - the handle is implemented with the existing shadcn-svelte resizable primitive (`paneforge`); do not roll a custom drag implementation,
  - sensible bounds: minimum 25% and maximum 80% of the available main height for either pane so neither pane can fully collapse,
  - mobile (< `lg`) keeps natural page scroll and does not expose the resize handle, because vertical resizing is meaningless when the page already scrolls freely,
  - resize state must not affect image processing — only the visible preview canvas size changes.
- Crop mode:
  - entered from the preview toolbar,
  - hides the processed output preview so the user can crop the source accurately,
  - preserves current preview zoom/pan when entering crop mode,
  - allows zoom and pan while cropping,
  - shows source image with crop overlay/handles,
  - provides pointer/touch crop handles for direct manipulation,
  - provides numeric crop fields for accessible precise editing,
  - crop handles are an enhancement; keyboard users must be able to complete cropping with numeric fields and buttons,
  - includes **Crop to content**, **Reset crop**, **Apply**, and **Cancel** actions,
  - crop edits are draft state while crop mode is open,
  - exits back to comparison preview after apply/cancel,
  - only **Apply** commits crop settings and triggers processing,
  - dragging or editing crop fields does not process live.
- Empty state:
  - large drag/drop target in the same preview area,
  - reuse the existing top-right **Upload Image** button as the primary upload action rather than adding a competing button inside the preview,
  - concise drag/drop instructions,
  - concise privacy/local-processing copy,
  - supported format hint.
- Output preview uses crisp rendering when the resize algorithm is `nearest`; otherwise it uses the browser's default interpolation. Display scaling is dictated by the chosen resize algorithm — there is no separate user-facing pixel-perfect toggle.

### Control availability before image upload

Controls available before image upload:

- palette controls,
- color-distance mode,
- dithering algorithm/settings,
- alpha mode/settings,
- resize algorithm.

Controls disabled before image upload:

- output dimensions,
- export,
- process/cancel,
- preview zoom/pan/reveal interactions.

Disabled controls must explain why they are disabled where it is not obvious.

### Resize and output controls

- Output dimensions:
  - width: integer only, min `1`, max `16384`, default source width auto-fit within output bounds,
  - height: integer only, min `1`, max `16384`, default source height auto-fit within output bounds,
  - decimal input is not allowed.
- Maximum output pixel count: `67_108_864` pixels (`8192 × 8192`).
- Maximum output side length: `16_384` pixels in either direction.
- The source image may be larger than these limits; only the processed output must fit within bounds.
- On upload, initialize output dimensions to the source aspect ratio scaled down only if needed to fit within max output pixels and max side length. This allows wide/tall outputs larger than `8192` on one axis as long as the other axis is small enough and the total pixel count fits.
- If a requested output exceeds the max pixel or side-length limit, block processing and explain the limit. The max may become configurable in a future version for users creating very large templates.
- Output aspect ratio is always locked to the current source/crop aspect ratio so exported pixels remain square:
  - changing width derives height from the source/crop aspect ratio,
  - changing height derives width from the source/crop aspect ratio,
  - after crop is applied, provide a **Reset aspect to crop** button that restores the crop rectangle aspect ratio and recalculates dimensions,
  - before any crop, the equivalent reset action uses the source natural aspect ratio,
  - reset preserves the last edited dimension and recalculates the other dimension; default to preserving width if neither has been edited.
- Fit mode is not exposed in the MVP UI; processing maps the source/crop region to the locked-aspect output dimensions without distortion.
- Manual crop is an MVP requirement:
  - user enters crop mode from the preview toolbar,
  - while cropping, the output preview is hidden and the source/crop overlay is shown,
  - user can define a crop rectangle before resizing,
  - crop rectangle is constrained to the source image bounds,
  - crop controls must work accessibly without requiring drag-only interaction,
  - crop coordinates persist with the image/session.
- Auto-crop-to-content is an MVP requirement:
  - button: **Crop to content** / **Crop to bounds**,
  - if alpha channel is present, crop to the bounding box of pixels with alpha above the current configured alpha threshold,
  - if no alpha channel is present, disable the button or explain that no transparent bounds were found,
  - after auto-crop, output dimensions keep the current entered-dimensions aspect ratio unless the user clicks **Reset aspect to crop**.
- Resize algorithm is user-configurable.
- Default resize algorithm: `lanczos3`, because this app is primarily converting natural/generated images into palette-constrained output and should preserve detail when downscaling.
- MVP resize algorithms:
  - `nearest` — nearest neighbor; hard pixel edges, best for already-pixel-art sources.
  - `bilinear` — fast 2×2 interpolation; good baseline smoothing.
  - `lanczos3` — high-quality windowed sinc with detail preservation; default.
  - `area` / `box` — area averaging for strong downscales where preserving overall color energy matters.
- Anti-aliasing beyond resize filtering is not MVP. Future planned: optional prefilter/anti-alias controls if they solve real artifacts not handled by resize filters or tone controls.
- Future planned resize algorithms:
  - `bicubic` — smoother cubic interpolation; good general-purpose enlargement/downscaling.
  - `lanczos2` — high-quality windowed sinc with smaller radius.
  - `trilinear` / mipmap-linear — blend between mip levels; especially relevant for future WebGL/WebGPU backends.
  - `mitchell-netravali` — balanced cubic filter with controlled ringing.
  - `catmull-rom` — sharper cubic filter.
  - `hermite` — smooth resampling often used for downscaling.
  - `gaussian` — blurrier but useful as an anti-aliasing prefilter.
  - `multi-step` — repeated downscale passes using a simpler filter for quality/performance tradeoffs.
- Browser canvas `imageSmoothingQuality` may be used only as an implementation fallback. The app should expose named algorithms with documented behavior, not vague browser-dependent “high quality”.
- Export strip exposes output format `PNG`, dimensions, scale, and active color count.
- PNG is the only image export format in the MVP.
- Exported PNG must be an indexed palette PNG when the output uses ≤256 palette entries: PNG color type 3 with `PLTE` chunk and `tRNS` chunk when Transparent is present/enabled.
- PNG `PLTE` includes all enabled palette entries in stable palette order, not disabled entries and not only colors used in the output.
- Output pixel indices reference that enabled-entry palette.
- If Transparent is enabled, keep its stable palette-order position, emit placeholder RGB `(0, 0, 0)` in `PLTE`, and emit `tRNS[transparentIndex] = 0`; visible entries in `tRNS` are `255` where needed.
- Do not export the processed result as a full RGBA/truecolor PNG unless indexed PNG encoding fails and the user explicitly accepts a fallback.

### Built-in palettes

- Default active palette: Wplace 64-entry palette from <https://wplacepaint.com/colors/> and <https://wplace.wiki/wplace-colors-palette>.
- The Wplace palette contains 63 visible colors plus a 64th transparent entry; `wplace.wiki` explicitly lists the transparent entry.
- Store Wplace colors as static source data with labels and `premium` flags:
  - Free: `#000000`, `#3c3c3c`, `#787878`, `#d2d2d2`, `#ffffff`, `#600018`, `#ed1c24`, `#ff7f27`, `#f6aa09`, `#f9dd3b`, `#fffabc`, `#0eb968`, `#13e67b`, `#87ff5e`, `#0c816e`, `#10ae82`, `#13e1be`, `#60f7f2`, `#28509e`, `#4093e4`, `#6b50f6`, `#99b1fb`, `#780c99`, `#aa38b9`, `#e09ff9`, `#cb007a`, `#ec1f80`, `#f38da9`, `#684634`, `#95682a`, `#f8b277`.
  - Premium: `#aaaaaa`, `#a50e1e`, `#fa8072`, `#e45c1a`, `#9c8431`, `#c5ad31`, `#e8d45f`, `#4a6b3a`, `#5a944a`, `#84c573`, `#0f799f`, `#bbfaf2`, `#7dc7ff`, `#4d31b8`, `#4a4284`, `#7a71c4`, `#b5aef1`, `#9b5249`, `#d18078`, `#fab6a4`, `#dba463`, `#7b6352`, `#9c846b`, `#d6b594`, `#d18051`, `#ffc5a5`, `#6d643f`, `#948c6b`, `#cdc59e`, `#333941`, `#6d758d`, `#b3b9d1`.
  - Transparent: special non-hex transparent entry named `Transparent`.
- Palette colors have:
  - name/label,
  - hex for visible colors,
  - transparent marker for transparent entries,
  - source/type metadata.
- Palette size limit: maximum 256 entries including Transparent. This keeps export compatible with indexed palette PNG output.
- Enabled state is user preference state, not intrinsic palette data.
- Enabled state is stored by stable color key, not row id:
  - visible color key: canonical uppercase `#RRGGBB`,
  - transparent color key: `transparent`.

### Palette controls and editing

The palette panel lives in the right column below the hero preview.

Required UI:

- Panel header: **Palette** plus short helper text.
- **Preset** selector at the top, defaulting to `Wplace (Default)`.
- Built-in status text/badge for Wplace: “Built-in”, “Immutable”, or equivalent.
- Built-in palettes block Add Color and Delete actions. Customization requires duplicating the built-in palette or creating a custom palette.
- Toolbar buttons:
  - **Select All** — selects all rows for bulk actions,
  - **Deselect All** — clears row selection,
  - **Toggle Selected** — if any selected color is disabled, enable all selected colors; otherwise disable all selected colors,
  - **Add Color**,
  - **Delete** for selected custom colors.
- Do not include a toolbar-level **Modify** action. Modify/edit is row-level only.
- Palette supports both grid and list modes.
- Default palette view mode: list.
- User-selected palette view mode persists.
- Palette panel uses vertical scrolling (`overflow-y`) for large palettes rather than expanding the whole page indefinitely.
- Dense list/table mode has columns:
  - row number,
  - row selection checkbox,
  - enabled checkbox/switch,
  - swatch,
  - color name/label,
  - hex,
  - free/premium/custom status,
  - per-row actions menu/buttons, including row-level **Edit**.
- On mobile, list mode should collapse secondary columns/actions into a clear **Edit** button or row action that opens a shadcn-svelte `Dialog` with details and edit controls.
- Grid mode shows compact swatches with enabled/selected state, name/hex accessibly available, and edit/details actions via shadcn-svelte `Dialog`.
- Rows/items must support:
  - toggling enabled state,
  - selecting rows for bulk modify/delete,
  - editing a single custom color from that row’s **Edit** action,
  - duplicating a color,
  - deleting a custom color.
- List footer shows active count: `64 / 64 colors active`.

Editing rules:

- User can create a new palette.
- User can duplicate a built-in palette into a custom palette.
- User can rename custom palettes.
- User can export selected custom palettes or all custom palettes as JSON.
- User can import custom palettes from JSON.
- User can add visible colors by `#RRGGBB` or shorthand `#RGB` hex input and browser color picker.
- Adding/importing colors is blocked when it would exceed 256 palette entries including Transparent.
- User can edit custom visible color name and hex.
- Shorthand `#RGB` is accepted as input and normalized to canonical uppercase `#RRGGBB` for storage.
- Duplicate visible color hex values are not allowed within the same palette.
- Only one transparent entry is allowed within the same palette.
- MVP does not support `#RRGGBBAA` or semi-transparent palette colors. Transparent is a special palette entry.
- User can remove colors from custom palettes. Delete means actual removal from the palette, not disabling; enabled toggles handle inclusion/exclusion without removal. All delete actions require confirmation, including single-color, bulk-color, custom-palette, clear-data, and factory-reset deletes.
- User can toggle any active palette color on/off for conversion.
- Future planned: palette colors can have user-adjustable weight/gravity values that bias nearest-color matching toward preferred colors.
- The default Wplace palette cannot be customized. Its colors cannot be edited, deleted, reordered, or renamed.
- Reordering custom palette colors is future planned, not MVP.
- Individual Wplace colors can be turned on/off for conversion, and that enabled/disabled state should persist.
- Wplace defaults to all 64 entries enabled.
- New custom palettes and newly imported palettes default to all colors enabled.
- Overwritten palettes preserve enabled state by exact/remapped color key.
- To customize Wplace colors, the user must first create a new custom palette or duplicate the Wplace palette.
- If the user clicks add/edit/delete on an immutable built-in palette/color, explain the constraint and offer **Duplicate palette to edit**.
- Persist custom palettes, active palette choice, and per-palette enabled states in persistent nanostores.
- Use persistent nanostores for app persistence. Keep validation explicit and do not leak storage details into components.
- If every palette entry is disabled, disable processing and show a clear error. At least one enabled entry is required; this may be a visible color or Transparent.

Palette import/export:

- JSON import/export is an MVP requirement.
- Export supports:
  - selected custom palette,
  - all custom palettes.
- Import validates schema, palette names, visible hex colors, duplicate hex values, transparent entries, palette size ≤ 256 entries, and future gravity values. Enabled state is not imported as palette data.
- Palette name is the palette identifier. Do not maintain separate `id` and `name` fields for palettes.
- Palette names are user-visible, customizable, and case-sensitive.
- Palette names are unique keys by exact case-sensitive match within stored palettes.
- Empty/whitespace-only palette names are invalid.
- Renaming a custom palette to an existing exact name is blocked unless the user confirms replacing the existing custom palette.
- Import behavior:
  - built-in palettes are never overwritten or mutated,
  - if an imported custom palette has the same exact case-sensitive name as an existing custom palette, the existing custom palette is overwritten with the imported palette after summary confirmation,
  - imported palettes with no matching custom palette are created as new custom palettes,
  - importing a palette with a built-in palette name is blocked or requires renaming because built-ins are immutable,
  - overwrite confirmation shows palette name, existing color count, imported color count, and warning that replacement changes the palette colors,
  - enabled state is preserved across palette overwrite by matching colors exactly by hex/transparent first, then by nearest color using sRGB squared Euclidean distance if needed,
  - if the overwritten palette is active, keep it active, mark the current output stale, and reprocess with the imported palette.
- Future planned import UX: full diff view before overwrite/merge.
- Add/delete/import-into-current actions are blocked while a built-in palette is active unless the user duplicates it first.
- Invalid import files show actionable inline errors and a toast.
- Future planned import/export formats: GIMP GPL, Adobe ASE/ACO where practical, Lospec palette URLs/files, and PNG palette strips.

Future palette gravity/weighting:

- Each palette color may have a `gravity` value, default `1.0`.
- Higher gravity makes a color more likely to win when source colors are between nearby palette colors.
- Gravity must not make a very distant color win arbitrarily; it should bias the distance score, not replace distance matching.
- Candidate scoring model:

```ts
adjustedDistance = rawDistance / sqrt(gravity)
```

- Alternative scoring models may be tested, but must be documented and deterministic.
- UI should expose gravity as an advanced custom-palette control, not in the default simple palette table.
- Built-in Wplace palette gravity values remain immutable unless the palette is duplicated.
- Gravity values should persist for custom palettes.
- Exact behavior needs visual tests with colors near decision boundaries.

Persistence schema:

- Use persistent nanostores for all restorable app state.
- Preferences/settings storage key: `ditherette:prefs:v1`.
- Stored preferences include:
  - `version: 1`,
  - `activePaletteName`,
  - `paletteEnabled: Record<string, string[]>` mapping palette name to enabled color keys; the effective enabled lookup key is `(paletteName, colorKey)`,
  - `customPalettes: Palette[]`,
  - future custom palette color order once reordering exists,
  - all conversion settings,
  - preview mode and non-image preview state where useful, including the desktop preview-pane size (a number in `0..1` representing the preview pane's share of the resizable split) so a user's chosen layout balance survives reload.
- Persist uploaded image state and processed output state indefinitely until the user explicitly clears it:
  - file metadata in persistent nanostores,
  - source image binary data in IndexedDB or another browser storage layer suitable for blobs/ArrayBuffers,
  - processed output indices, export palette, and dimensions in IndexedDB/blob storage,
  - persistent nanostores store references/ids to binary records rather than huge base64 strings in localStorage.
- Do not persist RGBA preview data by default; reconstruct preview from persisted indices/palette/dimensions on restore unless performance proves this insufficient.
- Do not store large images or processed previews directly in localStorage.
- Provide clear, accessible data actions:
  - **Clear image**: removes source image, output preview, file metadata, and processing state; keeps settings and palettes.
  - **Reset settings**: resets conversion and preview settings to defaults; keeps palettes and stored image unless the user also clears it.
  - **Manage palettes**: deletes custom palettes individually with confirmation.
  - **Factory reset**: clears all Ditherette local data after confirmation.
- Communicate that images and previews are stored locally on this device and never uploaded.
- If browser quota/storage fails, degrade gracefully to settings-only persistence and notify the user.
- On reload, restore the latest uploaded image and output preview when available, then validate that stored settings still match the restored output.
- If the output preview was generated from older settings, mark it stale and auto-process a fresh output.
- Validate and sanitize all persisted data before use.
- Unknown versions fall back to defaults and preserve raw data only if a future migration path is added.

Accessibility for palette controls:

- Swatch buttons expose color name, hex, status, and enabled state.
- Bulk actions announce how many rows/colors are affected and whether they selected rows, changed enabled state, or deleted colors.
- Row action menus are keyboard-operable.
- Future reordering must provide accessible alternatives such as move up/down row actions; do not ship drag-only reordering.

### Color matching math

The processor maps each output pixel to the nearest enabled palette color using the selected distance function.

Prioritized first implementation distance modes:

1. `oklab` — Euclidean distance in OKLab. This is the default color matching mode. The UI should label it OKLab rather than “perceptual”.
2. `srgb` — Euclidean distance in gamma-encoded sRGB.
3. `linear-rgb` — Euclidean distance after sRGB linearization.
4. `weighted-rgb` — default weighted RGB distance using the CompuPhase formula. The UI/explainer must explicitly say this is CompuPhase weighted RGB for transparency.
5. `weighted-rgb-rec601` — fixed-channel weighted RGB using Rec.601 luma coefficients.
6. `weighted-rgb-rec709` — fixed-channel weighted RGB using Rec.709/sRGB luma coefficients.
7. `cielab` / `lab-76` — CIELAB ΔE76.
8. `oklch` — OKLCH distance with circular hue handling.

Future planned distance modes:

- `hsl` — weighted hue/saturation/lightness distance with circular hue handling.
- `hsv` — weighted hue/saturation/value distance with circular hue handling.
- `xyz` — Euclidean distance in CIE XYZ after sRGB → linear RGB → XYZ conversion.
- `lab-94` — CIELAB ΔE94.
- `delta-e-2000` — CIEDE2000.
- `lchab` — cylindrical CIELAB lightness/chroma/hue distance.
- Additional weighted RGB variants:
  - `weighted-rgb-rec2020` — fixed-channel weighted RGB using Rec.2020 coefficients.
  - `weighted-rgb-rec2100` — fixed-channel weighted RGB using Rec.2100 coefficients. This does not imply full HDR processing support; it only reuses channel weighting coefficients unless an HDR pipeline is explicitly added.
  - `weighted-rgb-custom` — user-editable R/G/B channel weights.
- Priority-weighted matching presets:
  - `gamma-priority` — tune matching to preserve perceived luminance/gamma behavior.
  - `luma-priority` / `lightness-priority` — overweight brightness/lightness differences.
  - `chroma-priority` — overweight saturation/chroma differences.
  - `hue-priority` — overweight hue differences with circular hue math.
  - `shadow-priority` — overweight dark-region differences.
  - `highlight-priority` — overweight light-region differences.
  - `skin-tone-priority` — future specialized preset only if clearly documented and tested.
- `ycbcr`, `yiq`, `luv`, `lchuv`, `din99`, `cam16-ucs`, `jzazbz`, `ictcp`.
- Wplace’s closed-source “perceptual”, “CIEDE2000”, and “weighted RGB” options may inspire labels/explanations, but must not be represented as exact compatibility modes unless an exact implementation is documented or verified. Wplace likely uses OKLab or a similar perceptual model for “perceptual”, but that is not guaranteed.
- Do not add future modes to the UI until each has tests, a useful explanation, and a visualizer story.

Implementation notes:

- Convert palette colors once per settings change and cache converted coordinates.
- Nearest-color ties are resolved by stable palette order. Exact equal distances should be rare, but tie-breaking must be deterministic.
- Convert pixels into the selected color space during processing.
- Alpha handling is user-configurable; see Alpha/transparency handling below.

### Color-space educational UI

The color-space panel lives below dithering in the left column.

Required UI:

- Panel header: **Color Space** plus short helper text.
- Color-space selector or compact tabs.
- A small 3D canvas visualizer for the selected color space.
- A concise explanation of the selected color space and why it changes palette matching.
- A short “math in brief” section with the actual model, not marketing copy.
- Optional “Learn more” link target can be internal placeholder until docs exist.

Visualizer requirements:

- Use a canvas-based 3D-looking plot, not an external rendering dependency for MVP.
- MVP visualizers are static 3D-looking canvases.
- Future planned: interactive rotate/zoom visualizers with accessible keyboard alternatives.
- Plot should visually communicate the selected coordinate system:
  - sRGB / Linear RGB / Weighted RGB: RGB cube, with weighting noted in the explainer for weighted modes.
  - XYZ: CIE XYZ volume or projected gamut with X/Y/Z axes.
  - HSL: hue cylinder/cone style representation.
  - HSV: hue cone/cylinder with value axis.
  - CIELAB / Lab ΔE modes: L/a/b axes with perceptual cloud/gamut boundary.
  - LCHab: L/C/h cylindrical representation derived from CIELAB.
  - OKLab: L/a/b axes with perceptual gamut blob.
  - OKLCH: L/C/h cylindrical representation.
  - OKLab may be described as perceptual-feeling, but do not expose a separate “perceptual” mode.
- Include axis labels visible in the canvas or adjacent legend.
- Canvas has accessible text fallback describing the visualization.

Per-mode explanation examples:

- sRGB: browser display space; Euclidean distance is fast but not perceptually uniform because gamma-encoded values do not match human vision.
- Linear RGB: removes sRGB gamma before distance math, making light mixing more physically meaningful.
- Weighted RGB: fast RGB-space matching with channel weights; default `weighted-rgb` uses the CompuPhase formula and must be labeled as such. Rec.601 and Rec.709 variants use fixed luma coefficients.
- HSL/HSV: separate hue, saturation, and lightness/value; hue distance wraps around the color wheel.
- XYZ: device-independent intermediate color space used as the bridge into CIELAB-family spaces.
- Lab ΔE76/94/2000: approximates human color difference after conversion through XYZ; later ΔE formulas add weighting for chroma/hue/lightness.
- LCHab: CIELAB expressed as lightness/chroma/hue, making hue/chroma behavior easier to explain.
- OKLab/OKLCH: modern perceptual spaces designed for smoother gradients and better hue behavior; OKLCH uses cylindrical lightness/chroma/hue coordinates.
- OKLab: use as the default perceptual-feeling mode; do not imply exact Wplace behavior because Wplace is closed source.

### Alpha/transparency handling

Transparency is a first-class setting because some workflows need PNG alpha, while Wplace-style palettes include/use a transparent color.

Required alpha modes:

1. `preserve` — preserve the source transparency mask using the alpha threshold, but output non-thresholded pixels as opaque. This is the default.
2. `premultiplied` — premultiply RGB by alpha before color matching. This is mutually exclusive with alpha-threshold masking.
3. `matte` — composite pixels over a user-selected matte color before palette matching and output opaque pixels.

Alpha controls live in the palette panel attached to the mandatory `Transparent` swatch, not in the Dimensions panel.

- Alpha mode select labels: `Preserve transparency`, `Premultiplied`, `Matte`.
- `Preserve transparency` uses source RGB for color matching, then applies the alpha threshold. Pixels above threshold become opaque; pixels at/below threshold become Transparent if enabled.
- Alpha threshold slider `0–255`, default `0`, enabled only for `preserve` mode. It is disabled for `premultiplied` and `matte` because threshold masking is preserve-mode-only.
- If the source image contains alpha/transparency, the app may suggest raising the threshold for antialiased edges in `preserve` mode, but must not silently change it.
- Matte color input, enabled only for matte mode.
- Default matte color is the darkest enabled visible palette color, defined as the enabled visible color with the lowest `r + g + b`; ties use stable palette order.
- When `usePaletteDarkestMatte = true`, matte color automatically updates if enabled palette colors change.
- User can override matte color manually by selecting one of the enabled visible palette colors; manual selection sets `usePaletteDarkestMatte = false`.
- Matte color cannot be an arbitrary color outside the active enabled palette.
- If the selected matte color becomes disabled or removed, fall back to the next closest enabled visible palette color using sRGB squared Euclidean distance, update `matteColorKey` to that fallback, and notify the user subtly. If no visible palette color is enabled, show a validation warning.
- Provide a **Use darkest palette color** action to restore automatic darkest-palette matte behavior.
- Transparent palette color behavior:
  - Every palette includes exactly one `Transparent` swatch; imports/create/duplicate flows enforce it.
  - Users can enable/disable Transparent like any other palette entry, but cannot edit/delete it.
  - If Transparent is enabled, pixels with alpha at or below the threshold may map to that transparent palette entry.
  - Transparent palette entries must be shown distinctly in the palette UI with a checkerboard swatch and accessible label.
  - The default Wplace palette includes `Transparent` as its 64th entry.
  - Transparent is enabled by default.
  - With the default `alphaThreshold = 0`, opaque images are unaffected and only fully transparent pixels map to Transparent.

Processing rules:

- Color mapping and dithering operate on visible colors; Transparent does not participate in RGB/color-distance nearest matching.
- Alpha mode affects the RGB values used for visible-color mapping before dithering:
  - `preserve`: match RGB using source RGB, then apply alpha threshold/mask after quantization/dithering,
  - `premultiplied`: match using premultiplied RGB/alpha math; do not apply alpha threshold/mask,
  - `matte`: composite over matte, then match the composited RGB.
- `preserve` logical order is: quantize with optional dithering → alpha threshold/mask application.
- `premultiplied` logical order is: premultiply RGB → quantize with optional dithering → opaque export.
- `matte` logical order is: matte composite → quantize with optional dithering → opaque export. Alpha threshold is not applied in matte mode.
- Preserve-mode alpha threshold rules:
  - if `alpha <= alphaThreshold` and Transparent is enabled, overwrite the output pixel with transparent palette color as RGBA `(0, 0, 0, 0)` for deterministic PNG output,
  - if `alpha <= alphaThreshold` and Transparent is disabled, overwrite the output pixel with the darkest enabled visible color in the palette,
  - darkest is defined cheaply as the enabled visible color with the lowest `r + g + b`; ties use stable palette order,
  - if `alpha > alphaThreshold`, keep the mapped/dithered visible color.
- Output alpha:
  - preserve-mode transparent palette pixels output alpha `0`,
  - preserve-mode non-thresholded pixels output alpha `255`,
  - premultiplied and matte modes output alpha `255` for all pixels.
- If all visible colors are disabled and only Transparent is enabled, all output pixels become transparent and the UI should warn that only Transparent is enabled.
- If Transparent plus one visible color are enabled, output behaves like a transparent/solid silhouette: alpha-thresholded pixels map to Transparent and non-transparent pixels map to the single enabled visible color.
- All alpha behavior must be deterministic and covered by fixtures for fully transparent, partially transparent, and opaque pixels.

### Histogram and tone/color adjustment roadmap

Future versions should add a pre-quantization adjustment stage before palette matching and dithering. This is not required for the first implementation, but the processing pipeline should leave a clear insertion point:

```txt
Decode → Crop/Fit → Resize → Tone/color adjustments → Alpha RGB preparation → Quantize with optional dithering → Optional alpha threshold/mask → Export
```

Crop/fit and resize happen before color mapping/quantization because mapping the full source image before resize is too expensive. Dithering wraps color mapping rather than happening strictly after it: ordered/random dithering perturbs the working color before nearest-color lookup, and error diffusion adds accumulated error before lookup then propagates the quantization error. Alpha threshold/mask is applied after quantization/dithering only in `preserve` mode; `premultiplied` and `matte` modes are mutually exclusive with alpha thresholding. Future source histograms may inspect the original decoded image separately, but conversion operates on cropped/resized output pixels.

Planned histogram features:

- Histogram panel showing source and/or adjusted image distribution before quantization.
- Histogram channel modes where they make sense:
  - RGB: R, G, B, combined luminance.
  - Linear RGB: linear R, G, B, luminance.
  - HSL/HSV: hue, saturation, lightness/value.
  - CIELAB: L, a, b.
  - OKLab: L, a, b.
  - OKLCH: L, chroma, hue.
- Circular hue histograms are allowed and should explicitly indicate wraparound at 0°/360°.
- Histogram should support hover/focus readouts for bin range and count.
- Histogram calculations should use typed arrays and reusable bin buffers.

Planned tone/color adjustment controls:

- Global gamma adjustment.
- Dynamic range / levels:
  - black point,
  - midpoint/gamma,
  - white point,
  - output black/white clamp.
- Curves:
  - global curve,
  - per-channel curves.
- Per-channel levels and curves in supported spaces/channels:
  - RGB / Linear RGB channels,
  - HSL/HSV channels where useful,
  - CIELAB L/a/b,
  - OKLab L/a/b,
  - OKLCH L/chroma/hue, with hue controls respecting circular hue.
- Optional saturation/chroma and hue rotation controls.
- Adjustment presets should be explicit and reversible.

Implementation constraints for future adjustments:

- Adjustments must be nondestructive settings applied during processing; never mutate the decoded source.
- Adjustment math must be deterministic and testable.
- UI should preview histogram changes before final quantization when performance allows.
- The adjustment stage should share color conversion functions with color matching to avoid duplicate math.

### Dithering

Dithering is optional and defaults to off (`none`).

Prioritized first implementation algorithms, in UI order:

1. None / direct nearest-color quantization. This is the default.
2. Ordered Bayer 2×2.
3. Ordered Bayer 4×4.
4. Ordered Bayer 8×8.
5. Ordered Bayer 16×16.
6. Floyd–Steinberg error diffusion.
7. Sierra error diffusion, using the most common standard Sierra kernel.
8. Sierra Lite error diffusion, using the most common standard Sierra Lite kernel.
9. Seeded random/noise threshold dithering.

Future planned algorithms:

- False Floyd–Steinberg error diffusion.
- Jarvis–Judice–Ninke error diffusion.
- Stucki error diffusion.
- Atkinson error diffusion.
- Burkes error diffusion.
- Full Sierra-family options beyond MVP, including Two-row Sierra and any other well-documented Sierra variants.
- Stevenson–Arce error diffusion.
- Clustered-dot ordered dithering.
- Blue-noise threshold dithering, using a deterministic bundled threshold tile.
- Void-and-cluster threshold dithering, if a deterministic threshold tile is bundled.
- Pattern/hatch dithering as a future educational mode.

Controls:

- Algorithm select.
- Strength slider `0–100%`, default `100%` when dithering is enabled.
- When algorithm is `none`:
  - strength control is disabled but its value is retained,
  - coverage control is disabled,
  - serpentine control is disabled,
  - random seed control is hidden.
- When user selects Bayer, Floyd–Steinberg, Sierra, Sierra Lite, or Random:
  - `ditherStrength = 1.0` by default,
  - `ditherCoverage = 'full'` by default,
  - `ditherCoverageThreshold = 0.5` by default,
  - `serpentine = true` by default for error diffusion algorithms only,
  - `randomSeed` is generated on first image/session and persisted.
- Dither coverage/threshold control determining where dithering is applied:
  - `Full image` — apply dithering everywhere.
  - `Transitions` — apply dithering mostly near color boundaries, gradients, and areas where the nearest palette choice is ambiguous.
  - `Edges only` — restrict dithering to stronger color/luminance transitions.
- The coverage/threshold control may be implemented as a mode select plus sensitivity slider, or as one labeled slider from `Edges` → `Transitions` → `Full` if that is clearer on mobile.
- Serpentine scan toggle for error diffusion, default enabled.

Rules:

- MVP error diffusion operates in linear RGB working space, regardless of selected color matching mode.
- Palette matching still uses the selected color distance mode.
- Ordered and random dithering perturb RGB before palette matching.
- Future planned: experiment with dither working spaces other than linear RGB, such as selected-space diffusion or OKLab-space diffusion, but only if documented and tested against artifacts.
- Clamp channels after error propagation.
- Dither strength scales perturbation/error rather than blending final colors:
  - error diffusion multiplies propagated quantization error by `strength × coverageMask`,
  - ordered/random dithering multiplies threshold/noise perturbation by `strength × coverageMask`.
- Ordered dithering perturbs source color before palette matching, scaled by strength.
- `ditherStrength = 0` must produce exact pixel-identical output to `none` for the same resize/color/palette/alpha settings.
- MVP dither coverage/threshold uses a simple deterministic gradient-based mask:
  - compute local linear-luma gradient from a small neighborhood,
  - normalize the gradient to `0..1`,
  - multiply dither perturbation/error strength by the mask for `Transitions` / `Edges only`,
  - `Full image` coverage uses the algorithm normally and ignores the mask.
- `Transitions` uses a softer gradient mask; `Edges only` uses a stronger thresholded mask.
- Full coverage/threshold behavior is explicitly a next-version area: future versions should add a more complete implementation using nearest-vs-second-nearest palette ambiguity, local contrast, and better user-facing threshold controls.
- The exact MVP coverage function must be documented and tested with small fixtures before exposing it as stable behavior.

### Dithering educational UI

The dithering panel lives at the top of the left control column.

Required UI:

- Panel header: **Dithering** / **Dither Controls** plus short helper text.
- Algorithm selector.
- Strength slider with numeric value and semantic labels such as subtle/balanced/bold.
- Coverage/threshold control with semantic labels explaining whether dithering affects the whole image or mostly color transitions/edges.
- Serpentine scan toggle shown only or disabled-helpfully for algorithms where it applies.
- Small preview canvas showing the selected algorithm’s typical pattern on a fixed deterministic synthetic tile.
- MVP dither preview tile should be `64×64` or `128×128` and include a luminance gradient, color ramp, and hard edge so differences between algorithms are visible.
- Future planned: optional preview using a crop/sample from the uploaded image.
- Concise educational paragraph explaining the selected algorithm.
- For kernel-based algorithms, show the error diffusion kernel as a mini matrix.
- For ordered dithering, show the Bayer threshold matrix.
- For random dithering, show that per-pixel noise perturbs the quantization threshold.

Per-algorithm explanation expectations:

- None: direct nearest-color quantization; fastest but can create flat bands.
- Floyd–Steinberg: distributes quantization error to four future neighbors with weights `7/16`, `3/16`, `5/16`, `1/16`.
- False Floyd–Steinberg: cheaper three-neighbor approximation that creates stronger directional texture.
- Jarvis–Judice–Ninke: spreads error across a wider 3-row neighborhood, producing smoother texture at higher blur/noise cost.
- Stucki: similar wide error diffusion with different normalization, often sharper than JJN.
- Atkinson: distributes only part of the error, preserving contrast with a distinct pixel-art texture.
- Burkes: two-row diffusion kernel; faster than JJN/Stucki while smoother than Floyd–Steinberg.
- Sierra / Two-row Sierra / Sierra Lite: related kernels with decreasing neighborhood size and cost.
- Stevenson–Arce: larger sparse kernel intended to reduce directional artifacts.
- Bayer 2×2/4×4/8×8/16×16: deterministic threshold matrices create ordered patterns; larger matrices reduce obvious repetition.
- Clustered-dot: ordered thresholding that forms print-like clustered dot structures.
- Blue-noise / void-and-cluster: deterministic threshold textures designed to reduce low-frequency pattern visibility.
- Random: adds seeded noise before quantization, avoiding patterning but producing grain. Include a **Randomize seed** button. Use Mulberry32 as the deterministic MVP PRNG; seed is a uint32, randomize via `crypto.getRandomValues`, and never use `Math.random` in processing.

### Preview and export

- Show the hero comparison preview before all controls.
- Show source and output preview side by side on desktop by default.
- Default to A/B Reveal on mobile/narrow screens.
- Pinch zoom and drag-to-pan should work naturally inside the preview area on touch devices.
- Output preview uses crisp rendering when the resize algorithm is `nearest`; otherwise it uses default interpolation. The preview always renders the actual processed pixels — there is no separate pixel-perfect toggle.
- Show output dimensions and active color count. Transparent counts as an active color when enabled.
- Bottom export strip/card:
  - output metadata: dimensions, palette color count, format,
  - optional scale controls,
  - primary **Download PNG** button.
- Do not include a Copy PNG button in the MVP, even on browsers that support image clipboard writes, because Wplace does not support pasting images as templates.
- PNG is the only MVP image export format.
- MVP PNG export should be an actual 8-bit indexed-color palette PNG: 8-bit palette indices plus `PLTE`, with `tRNS` for transparent index when needed, rather than a 32-bit RGBA canvas PNG.
- Always use 8-bit indexed PNG for MVP, even when the active palette has fewer than 256 entries. Future planned: optional 1/2/4-bit indexed PNG optimization for tiny palettes.
- Filename format: `<original-name>-ditherette-<width>x<height>.png`.
- Future planned export: Wplace template file with `.wplace` extension. `.wplace` is a custom JSON file using a documented Wplace-oriented schema for template metadata, target coordinates, dimensions, palette/color placement data, and settings needed to recreate/use the template. Coordinate fields are required for `.wplace`, but the exact schema is deferred.

## UI requirements

Using the existing shadcn-svelte components already installed in `src/lib/components/ui` is mandatory. These components dictate Ditherette’s style, interaction patterns, accessibility baseline, and visual identity. Do not replace them with hand-rolled equivalents when a matching shadcn-svelte component exists. Custom components should compose shadcn-svelte primitives and only add behavior/layout specific to this app.

Recommended components:

- `Button` for upload, replace, bulk palette actions, and export.
- `Input` for dimensions, palette names, and hex values.
- `Label` for form fields.
- `Switch` for aspect-ratio lock, serpentine scan, and per-row enablement if preferred.
- `Checkbox` for row selection and per-color enablement.
- `Select` for resize mode, palette preset, color math, and dithering algorithm.
- `Slider` for dithering strength and reveal slider semantics where useful.
- `Badge` for free/premium/custom/built-in metadata.
- `Tabs` or segmented button group for preview mode and color-space mode.
- `ScrollArea` for large palette grids.
- `Dialog` for create/rename palette and edit color flows.
- `DropdownMenu` for per-row palette actions.
- `Table` or `DataTable` for the palette list when it improves keyboard and screen-reader behavior.
- `Sonner` toast for import/export and validation feedback.
- Use `Card` for lower control panels and export strip only; do not put the top comparison preview inside a card.

Component usage rules:

- Prefer imports from `$lib/components/ui/...` for every standard control, panel, table, dialog, menu, tab, badge, slider, switch, checkbox, and input.
- Hand-written HTML controls are acceptable only for canvas surfaces and behaviors not covered by shadcn-svelte, such as the A/B reveal handle, preview canvases, color-space visualizer canvas, and dither pattern canvas.
- Even custom canvas-adjacent controls should use shadcn-svelte `Button`, `Tabs`/segmented controls, `Slider`, `Tooltip`, and `Badge` where applicable.
- Do not introduce another component library or independent visual design system.

Layout direction:

- Design mobile-first. Start with the narrow-screen layout and progressively enhance to tablet/desktop.
- Single-page app; style is intentionally unspecified beyond shadcn-svelte components and theme tokens.
- Top app bar spans the page and must work at phone widths without overflowing.
- Preview compare component is full-width and visually dominant, with controls reachable on touch screens.
- Below preview: stacked single-column layout by default; switch to a two-column grid only when width comfortably supports it.
  - Left column on desktop: Dithering panel, then Color Space panel.
  - Right column on desktop: Palette panel, taller than left panels if needed.
- On `lg+`, the preview and the control area are split by a draggable vertical-resize handle so users can claim more space for either side. Mobile keeps natural scroll and no handle.
- Bottom export strip spans the page; on mobile, keep the primary export action easy to reach.
- Palette editor should be dense and scannable in both grid and list modes: swatches with enabled state, hex, name, metadata, and quick actions.
- Prioritize spatial layout, readable controls, touch targets, and the educational canvases/explainers over decorative styling.
- Mobile layout order:
  1. app bar,
  2. preview,
  3. export primary action if image exists,
  4. dithering,
  5. color space,
  6. palette,
  7. full export strip.

## Accessibility and semantics

Accessibility is a release requirement, not polish. Prefer semantic Svelte/HTML and shadcn-svelte components over div-heavy custom controls.

- Mobile accessibility is a first-class requirement, not a desktop adaptation.
- Use semantic landmarks: `header`, `main`, `section`, `figure`, `figcaption`, `fieldset`, `legend`, `table`, `thead`, `tbody`, `tr`, `th`, `td`, `button`, `label`, and `output` where appropriate.
- Use real form controls for settings. Avoid clickable `div`s.
- All inputs have labels.
- Canvas previews have descriptive labels and metadata text.
- A/B reveal handle is keyboard-operable and exposes a `0–100` slider value.
- Side-by-side/reveal toggle uses accessible pressed/selected state.
- Drag/drop upload also has keyboard-accessible file picker.
- Do not rely on color alone for enabled/disabled or premium/free state.
- Swatches expose color name, hex value, and enabled state to screen readers.
- Educational canvases have text alternatives matching the current selected color-space/dither mode.
- Icon-only actions have accessible names.
- Touch targets should be at least 44px where practical, especially upload/replace, reveal handle, palette row actions, toggles, and export buttons.
- Keep heading levels logical and stable.
- Use `aria-*` only when native semantics or shadcn-svelte semantics are insufficient.
- Any custom keyboard interaction, especially the A/B reveal handle and canvas-adjacent controls, must support keyboard, pointer, and screen-reader use.

## Maintainability

- Keep image-processing algorithms in pure TypeScript modules, separate from Svelte components.
- Keep Svelte components focused on one responsibility: preview, dithering controls, color-space controls, palette editor, export strip.
- Prefer small typed data tables for color-space and dithering educational metadata over hard-coded conditionals in components.
- Use named constants for limits, defaults, dither kernels, Bayer matrices, and persistence keys.
- Avoid `any`, `@ts-ignore`, and cross-component global state shortcuts.
- Validate at trust boundaries: file input, persisted nanostore/IndexedDB data, user-entered hex colors, dimensions.
- Keep built-in palette data immutable; derive editable state separately.
- Write exported functions with clear names and unit tests before wiring deeply into UI.

## Performance

- Auto-process after setting changes with debounce.
- Debounce delay scales with output pixels so larger images wait longer before processing:
  - small outputs should feel live,
  - large outputs should avoid repeated processing during rapid control changes.
- Use named constants for debounce bounds and document the function, e.g. `PROCESS_DEBOUNCE_MIN_MS`, `PROCESS_DEBOUNCE_MAX_MS`, and `getProcessDebounceMs(pixelCount)`.
- Prefer `ImageBitmap` for decoding where available.
- Use typed-array-first image processing:
  - operate directly on `ImageData.data` (`Uint8ClampedArray`) for source/output pixels,
  - use `Float32Array` buffers for error diffusion accumulators,
  - use compact numeric palette buffers (`Uint8Array`/`Float32Array`) for compiled palette coordinates,
  - avoid per-pixel object allocation inside hot loops,
  - prefer `for` loops and precomputed offsets over iterator helpers in pixel loops.
- Precompute lookup tables where useful:
  - sRGB → linear channel table,
  - selected color-space coordinates for palette colors,
  - Bayer threshold matrices flattened into typed arrays,
  - dither kernel offsets/weights flattened into typed arrays.
- SIMD strategy:
  - TypeScript MVP should be structured so hot loops can later move to WASM SIMD without changing UI contracts,
  - use browser-native SIMD only through WASM in the acceleration phase; do not rely on non-standard JavaScript SIMD APIs,
  - keep hot-loop data contiguous and numeric to make SIMD/WASM/WebGL ports straightforward.
- Cache palette conversions per color space.
- Cache educational dither preview tiles per algorithm/settings.
- Web Worker processing, multithreading-ready architecture, and async job orchestration are MVP requirements.
- Use Effect v4 beta for async orchestration, cancellation, structured errors, progress reporting, and resource cleanup.
- Keep pure processing functions framework-independent and testable; wrap them in Effect at orchestration boundaries rather than embedding Effect throughout hot loops.
- Processing hot loops run off the main thread in a Web Worker for MVP.
- Main-thread fallback may exist for tests/dev diagnostics, but production UI should use the Worker path.
- Guard maximum output pixel count with `MAX_OUTPUT_PIXELS = 67_108_864` (`8192 × 8192`) and maximum side length with `MAX_OUTPUT_SIDE = 16_384`. Show friendly blocking errors to avoid browser lockups. Source images may exceed these limits; processing dimensions may not.
- Reveal slider, zoom, pan, and preview mode must not affect image processing at all.
- Keep full-resolution processed output data available for preview interactions, but do not size the visible canvas backing store to the full output dimensions for huge images.
- The visible preview canvas renders at display/viewport scale. Pan, zoom, reveal, and comparison redraw from already-processed full-size buffers/indices using transforms/source rectangles.
- MVP preview rendering converts only the visible viewport from `indices + palette` into a display-sized RGBA buffer on redraw, rather than materializing a huge full-canvas preview.
- Cache the last rendered viewport when transform/source/settings are unchanged.
- Future planned: tile cache (`256×256` or `512×512` tiles) with cache eviction for faster repeated pan/zoom on huge outputs.
- Reveal, zoom, pan, and comparison mode changes use canvas drawing, clipping, CSS/DOM transforms, or lightweight redraws from already-processed buffers only.
- Do not reprocess the image when only preview interaction state changes.
- Processing must be asynchronous, Worker-backed, cancellable, progress-reporting, and resilient:
  - use Effect v4 beta to model processing jobs, cancellation, progress, errors, retry policy, and cleanup,
  - bridge Effect cancellation to `AbortController` / worker cancel messages,
  - hot loops check cancellation every row or chunk,
  - track a processing generation/job id,
  - discard late results unless the generation matches current state,
  - report stage + percent progress.
- Processing progress stages:
  - `decode`,
  - `resize`,
  - `prepare-palette`,
  - `quantize`,
  - `dither`,
  - `finalize`.
- UI progress/error behavior:
  - show a progress bar near the preview or export strip while processing,
  - show the current stage label and percent where space allows,
  - provide a Cancel button while processing,
  - keep the previous output visible until the new output completes,
  - mark the preview as updating/stale during processing with a small `Updating…` badge,
  - disable Download while the visible output is stale unless a future UI explicitly labels the action as downloading the previous result,
  - show inline errors near the preview or responsible control,
  - show toast notifications for unexpected failures,
  - preserve the last successful output after failures,
  - include a recovery action/message such as reduce output size, try a different algorithm, replace image, reset settings, or retry.
- Retry policy:
  - use Effect retry for transient failures such as worker startup race, worker crash/restart, or temporary decode/canvas failures,
  - do not retry deterministic validation failures such as invalid dimensions, unsupported file type, all palette entries disabled, or output over max limits,
  - retry at most a small bounded number of times with backoff,
  - expose a manual Retry action after failure when retry is reasonable.

## State model

Core app state:

```ts
type PreviewMode = 'side-by-side' | 'ab-reveal';
type FitMode = 'stretch' | 'contain' | 'cover';

type CropRect = {
  x: number;
  y: number;
  width: number;
  height: number;
};

type ResizeMode =
  | 'nearest'
  | 'bilinear'
  | 'bicubic'
  | 'lanczos2'
  | 'lanczos3'
  | 'box'
  | 'area'
  | 'trilinear'
  | 'mitchell-netravali'
  | 'catmull-rom'
  | 'hermite'
  | 'gaussian'
  | 'multi-step'
  | 'mipmap-linear';
type ColorDistanceMode =
  | 'srgb'
  | 'linear-rgb'
  | 'weighted-rgb'
  | 'weighted-rgb-rec601'
  | 'weighted-rgb-rec709'
  | 'hsl'
  | 'hsv'
  | 'xyz'
  | 'cielab'
  | 'lab-76'
  | 'lab-94'
  | 'delta-e-2000'
  | 'lchab'
  | 'oklab'
  | 'oklch';

type DitherAlgorithm =
  | 'none'
  | 'floyd-steinberg'
  | 'false-floyd-steinberg'
  | 'jarvis-judice-ninke'
  | 'stucki'
  | 'atkinson'
  | 'burkes'
  | 'sierra'
  | 'two-row-sierra'
  | 'sierra-lite'
  | 'stevenson-arce'
  | 'bayer-2'
  | 'bayer-4'
  | 'bayer-8'
  | 'bayer-16'
  | 'clustered-dot'
  | 'blue-noise'
  | 'void-and-cluster'
  | 'random'
  | 'pattern-hatch';

type PaletteColor = {
  name: string;
  /** Present for visible colors. Omitted for transparent entries. */
  hex?: string;
  transparent?: boolean;
  premium?: boolean;
  source: 'wplace' | 'custom';
  /** Future advanced control. Defaults to 1.0. */
  gravity?: number;
};

type Palette = {
  /** User-visible, customizable, case-sensitive stable identifier/display name. */
  name: string;
  readonly: boolean;
  colors: PaletteColor[];
};

type AlphaMode = 'preserve' | 'premultiplied' | 'matte';

type ConversionSettings = {
  width: number;
  height: number;
  lockAspectRatio: boolean;
  lockedAspectRatio: number;
  lastEditedDimension: 'width' | 'height';
  fitMode: FitMode;
  cropRect: CropRect | null;
  /** Defaults to 'lanczos3'. */
  resizeMode: ResizeMode;
  paletteName: string;
  /** Defaults to 'oklab'. */
  colorDistanceMode: ColorDistanceMode;
  /** Defaults to 'none'. */
  ditherAlgorithm: DitherAlgorithm;
  ditherStrength: number;
  ditherCoverage: 'full' | 'transitions' | 'edges';
  ditherCoverageThreshold: number;
  randomSeed: number;
  serpentine: boolean;
  /** Defaults to 'preserve'. */
  alphaMode: AlphaMode;
  /** Defaults to 0. */
  alphaThreshold: number;
  /** Canonical hex of an enabled visible palette color. */
  matteColorKey: string;
  usePaletteDarkestMatte: boolean;
};

type ProcessingStage = 'decode' | 'resize' | 'prepare-palette' | 'quantize' | 'dither' | 'finalize';

type ProcessingProgress = {
  jobId: string;
  stage: ProcessingStage;
  completed: number;
  total: number;
  percent: number;
};

type ProcessedImage = {
  width: number;
  height: number;
  /** One enabled-export-palette index per output pixel. Primary export data and full-resolution preview source. */
  indices: Uint8Array;
  /** Enabled entries in PNG PLTE order. */
  palette: PaletteColor[];
  /** Optional full-resolution or cached display preview buffer generated from indices/palette. */
  preview?: ImageData | ImageBitmap;
};

type PreviewState = {
  mode: PreviewMode;
  revealPercent: number;
  zoom: number;
  panX: number;
  panY: number;
  /**
   * Desktop-only preview pane size as a fraction in `0.25..0.80`,
   * representing the preview pane's share of the resizable
   * preview/controls split. Ignored on mobile, which uses natural
   * page scroll without a resize handle.
   */
  previewPaneSize: number;
};
```

Educational metadata:

```ts
type ColorSpaceInfo = {
  id: ColorDistanceMode;
  label: string;
  shortDescription: string;
  mathSummary: string;
  visualizer:
    | 'rgb-cube'
    | 'hsl-cylinder'
    | 'hsv-cone'
    | 'xyz-volume'
    | 'lab-cloud'
    | 'lch-cylinder'
    | 'oklab-gamut'
    | 'oklch-cylinder';
};

type DitherInfo = {
  id: DitherAlgorithm;
  label: string;
  family: 'none' | 'error-diffusion' | 'ordered' | 'blue-noise' | 'noise' | 'pattern';
  shortDescription: string;
  mathSummary: string;
  kernel?: number[][];
  matrix?: number[][];
};
```

Suggested module shape:

- `src/lib/effect/runtime.ts` — Effect v4 beta runtime/layers for app-side async orchestration.
- `src/lib/workers/processing.worker.ts` — Worker entrypoint for image processing jobs.
- `src/lib/workers/processing-client.ts` — typed main-thread client wrapping worker messages in Effect.
- `src/lib/workers/protocol.ts` — typed worker request/progress/result/error/cancel message contracts.
- `src/lib/palettes/wplace.ts` — Wplace palette data with source comments for WplacePaint and Wplace Wiki, including the explicit transparent entry.
- `src/lib/persistence/stores.ts` — persistent nanostores for settings, palettes, image session references, and preview state.
- `src/lib/persistence/blob-store.ts` — IndexedDB/blob persistence for uploaded images and processed output previews.
- `src/lib/persistence/schema.ts` — persistence validation, versioning, and migrations.
- `src/lib/palettes/storage.ts` — palette persistence helpers backed by the persistence layer.
- `src/lib/image/color.ts` — color parsing and color-space conversion.
- `src/lib/image/distance.ts` — palette nearest-color logic, including future gravity/weighted scoring.
- `src/lib/image/dither.ts` — dither kernels and ordered matrices.
- `src/lib/image/crop.ts` — crop rectangle, fit mode, and alpha-content bounds helpers.
- `src/lib/image/resize.ts` — resize helpers.
- `src/lib/image/process.ts` — conversion orchestration that produces palette index data as the primary output plus preview RGBA data for canvas display.
- `src/lib/image/png-indexed.ts` — deterministic indexed PNG encoder adapter for ≤256-color palette output (`PLTE`/`tRNS`). Use a focused dependency only if it demonstrably supports browser-side indexed PNG export with explicit palette/transparent chunks; otherwise implement our own encoder.
- `src/lib/image/buffers.ts` — typed-array buffer helpers for hot-loop image processing.
- `src/lib/image/lookup-tables.ts` — sRGB, color-space, Bayer, and kernel lookup tables.
- `src/lib/image/preview-patterns.ts` — sample tiles for dithering previews.
- `src/lib/education/color-spaces.ts` — color-space labels, explanations, and visualizer metadata.
- `src/lib/education/dithering.ts` — algorithm labels, explanations, kernels, and matrices.
- `src/lib/components/ComparisonPreview.svelte` — semantic hero source/output preview with side-by-side and A/B reveal modes.
- `src/lib/components/DitherPanel.svelte` — accessible controls, preview tile, and explanation.
- `src/lib/components/ColorSpacePanel.svelte` — accessible selector, 3D canvas visualizer, and explanation.
- `src/lib/components/PalettePanel.svelte` — preset selector and accessible palette editor.
- `src/lib/components/ExportStrip.svelte` — output metadata and download action.

## Correctness policy

- The Worker-backed TypeScript MVP is the reference implementation.
- Deterministic processing must be exact pixel-for-pixel for the same input/settings/palette:
  - no dither,
  - ordered dithering,
  - deterministic threshold-tile dithering such as blue-noise/void-and-cluster,
  - error diffusion algorithms.
- Correctness delta is `0` for deterministic algorithms, including future accelerated backends.
- Random/noise dithering is seeded by default and uses a precise reference algorithm.
- Seeded random output must be exact pixel-for-pixel for the same seed/input/settings/palette. MVP random dithering uses Mulberry32.
- Include a **Randomize seed** action that changes the seed and triggers reprocessing.

## Browser support

- Target modern evergreen browsers only:
  - current Chrome/Edge,
  - current Firefox,
  - current Safari,
  - current iOS Safari plus recent major versions as practical.
- No IE or legacy browser support.
- Use feature detection for APIs with uneven support, especially OffscreenCanvas, ImageBitmap, worker module behavior, IndexedDB/blob storage, and touch/pointer gestures.
- If OffscreenCanvas is unavailable, keep processing in the Worker and use main-thread canvas only for display rendering.
- Degrade gracefully when optional browser APIs are unavailable; do not silently lose user data.

## Dependency policy

- Focused dependencies are allowed when they materially reduce implementation risk.
- Limit dependency graph depth; prefer small, maintained packages with few/no transitive dependencies.
- Required/expected additions:
  - Effect v4 beta for async orchestration,
  - nanostores plus persistent nanostores for persistence.
- Indexed PNG export dependency policy:
  - search/evaluate existing browser-compatible PNG libraries for explicit indexed-color `PLTE`/`tRNS` export support,
  - candidates to evaluate include `@lunapaint/png-codec`, `UPNG.js`/maintained forks, and `fast-png`,
  - `pngjs` appears Node-oriented and commonly documents writing grayscale/RGB/RGBA rather than indexed color; do not assume it satisfies this requirement,
  - if no focused dependency clearly supports our exact indexed PNG contract, write a small deterministic indexed PNG encoder and use tiny helper dependencies only for CRC32/deflate if needed.

## Async processing and acceleration roadmap

The first implementation should be correct, accessible, maintainable TypeScript using typed arrays, Web Workers, and Effect v4 beta orchestration. After the Worker-backed MVP is working and covered by tests, add hardware-accelerated backends only where profiling shows clear wins.

MVP async processing requirements:

1. **Web Worker processing**
   - `processImage` hot loops run in a Worker.
   - Transfer `ImageData`/`ArrayBuffer`s where possible instead of copying.
   - Worker reports stage + percent progress by rows/chunks.
   - Worker supports cancel messages and abort checks.
   - Keep the same public processing contract so the UI does not care whether processing is worker-backed or later WASM/GPU-backed.
2. **Effect v4 beta orchestration**
   - Add Effect v4 beta as a project dependency.
   - Use Effect to model jobs as cancellable async effects.
   - Use typed errors for decode, validation, processing, worker, cancellation, and export failures.
   - Use scoped resource cleanup for object URLs, workers, transferred buffers, and pending jobs.
   - Keep hot loops free of Effect overhead; cancellation checks use lightweight signals passed into pure functions.
3. **Multithreading-ready design**
   - MVP may start with one processing Worker, but the protocol should not prevent a future worker pool.
   - Split work into chunks/rows where practical so progress, cancellation, and future parallelism are natural.
   - Error diffusion algorithms have scanline dependencies; do not parallelize them until a correct chunking strategy exists.

Post-MVP acceleration targets:

1. **WASM + SIMD for CPU hot loops**
   - Port nearest-color lookup, color-space conversion, and error diffusion loops to Rust/C/Zig/WASM if profiling identifies them as bottlenecks.
   - Enable WASM SIMD for batched channel math and palette-distance calculations.
   - Keep Worker TypeScript implementation as the reference path and fallback.
2. **WebGL/WebGPU for massively parallel stages**
   - Consider shader-based resize, color-space transforms, direct quantization, ordered dithering, and preview rendering.
   - Be cautious with error diffusion algorithms because they have scanline dependencies and are less naturally parallel.
   - Use GPU paths only behind feature detection with deterministic Worker TypeScript fallback.
3. **Backend selection layer**
   - Introduce a small processing backend interface only after a second backend exists:

```ts
type ProcessingBackend = {
  id: 'worker-typescript' | 'wasm-simd' | 'webgl' | 'webgpu';
  supports(settings: ConversionSettings): boolean;
  process(input: ImageData, settings: ConversionSettings, palette: CompiledPalette): Promise<ImageData>;
};
```

Acceleration acceptance criteria:

- Worker TypeScript backend remains tested and available as fallback.
- Correctness delta is `0` for deterministic algorithms: no dither, ordered dithering, blue-noise with deterministic tiles, and all deterministic error-diffusion algorithms must produce exact pixel-for-pixel matches to the TypeScript reference backend.
- Random/noise dithering is seeded by default and must use a precise reference implementation with exact output matching for the same seed/input/settings/palette.
- UI contracts and Svelte components do not change when acceleration is added.
- Hardware acceleration is chosen by measured performance, not aesthetics or novelty.

## Testing plan

Unit tests:

- Hex parsing accepts valid `#RRGGBB` and shorthand `#RGB`, normalizes to uppercase `#RRGGBB`, and rejects invalid values. MVP visible palette colors are stored as `#RRGGBB`; transparent is represented as a special palette entry, not `#RRGGBBAA`.
- Palette validation rejects palettes over 256 entries.
- Indexed PNG encoder emits valid `PLTE` and `tRNS` chunks and round-trips expected palette indices for fixtures.
- sRGB linearization and core color-space conversion known values.
- Nearest-color matching picks expected palette colors in each distance mode for representative samples.
- Dither kernels conserve expected weight sums.
- Ordered dither matrices are stable and normalized.
- Aspect-ratio dimension derivation rounds predictably.
- Crop/fit math clamps to source bounds and produces expected source-to-output mappings.
- Auto-crop-to-content finds alpha bounds using the configured threshold.
- Palette import/export validates JSON and rejects malformed palette data safely.
- Persistence storage migrates/validates persistent nanostore and IndexedDB data.
- Educational metadata exists for every exposed color-space and dithering option.
- Hot-loop helpers operate on typed arrays without per-pixel object allocation in tested paths.
- Alpha handling fixtures assert exact output for preserve, premultiplied, matte, threshold, and transparent-palette behavior.
- Deterministic processing fixtures assert exact pixel output for no dither and ordered dither.
- Dither strength `0` fixtures assert exact pixel identity with no-dither output.
- Mulberry32 PRNG tests assert deterministic sequences for fixed seeds.
- Seeded random/noise dither tests assert exact pixels for fixed seed/input/settings/palette.

Component/E2E tests:

- User can load a fixture image and see dimensions.
- Upload button changes to Replace Image after load.
- Mobile viewport defaults to A/B Reveal; desktop viewport defaults to Side-by-side.
- Side-by-side and A/B reveal modes can be toggled.
- Pinch zoom and drag-to-pan work in the preview area on touch devices.
- A/B reveal slider changes the visible split and supports keyboard input and screen-reader semantics.
- Aspect-ratio lock updates paired dimension.
- Changing color-space selection changes the explanation and visualizer label.
- Changing dithering algorithm changes the preview pattern and explanation.
- Toggling palette colors changes active color count.
- Disabling all palette entries blocks conversion with an error; enabling only Transparent is allowed but warns the user.
- Built-in palette edit/delete prompts duplication instead of mutating source data.
- Export button produces an indexed palette PNG download for ≤256-color outputs.
- Future `.wplace` template export schema has unit tests once implemented.

## Acceptance criteria

- App runs with `pnpm dev` and builds with `pnpm build`.
- Effect v4 beta is used for async processing orchestration.
- Image processing runs through a cancellable Web Worker path in the MVP.
- All image processing happens in the browser.
- Top app bar upload action changes from Upload Image to Replace Image after image load.
- Top hero comparison preview supports Side-by-side and A/B Reveal modes.
- Wplace palette is the default active palette, contains 63 visible colors plus Transparent, and cannot be edited/deleted/reordered directly.
- Wplace color enabled/disabled state, custom palettes, conversion settings, uploaded image session, and output preview state persist via persistent nanostores plus browser blob storage for binary data.
- Users can resize with aspect ratio maintained by default, up to `67_108_864` output pixels and `16_384` pixels on either side. Source images may be larger; output dimensions are auto-fit within bounds on upload.
- OKLab is the default color-distance mode.
- Users can choose multiple color-distance modes.
- Color-space UI includes a 3D canvas-style visualizer and math explanation for each mode.
- Users can create/customize palettes and toggle active colors.
- Palette controls include preset selection, select, deselect, add, selected delete, and row-level edit actions.
- Users can choose the implemented dithering algorithms from the catalog, including off/no dither.
- Dithering UI includes a representative preview and math explanation for each algorithm.
- Users can preview and download the converted image as PNG.
- UI is built with semantic Svelte/HTML and shadcn-svelte components.
- Core processing modules are isolated, typed, and maintainable.
- MVP image processing uses typed arrays and precomputed lookup tables in hot loops.
- Code structure leaves a clear path to worker pools, WASM SIMD, WebGL, or WebGPU backends after profiling.
- Deterministic processing paths have exact pixel-output reference tests.
