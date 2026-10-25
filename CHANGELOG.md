# ditherette-web

## 1.0.0

### Major Changes

- 8aa3343: Release v1.0.0 with CPU and WebGPU processing through one factory, client-selected
  tiling and fallback, source-aware Adaptive placement, GPU effects and resize,
  all dither families, fast approximate GPU diffusion, and detailed processing timings.

### Minor Changes

- 3a1e2e0: Scopes can inspect the image after effects, before palette matching, and the histogram shows pure-black and pure-white clipping for the inspected stage.
- 943b732: Show how many pixels each palette colour uses in the palette table, and the total pixels to paint, leaving out Transparent, in the export bar.
- 5fcb8ec: Adds guides to Wplace templates, Wplace colours, dithering and colour quantisation, with page descriptions, canonical URLs, social previews and a sitemap.

### Patch Changes

- 1dde608: Bound preview and saved-output memory for large images while keeping them available for display and PNG export. Release replaced preview resources and prevent superseded saves or restores from overwriting the current image.
- 4513b33: Error diffusion dithers on the GPU when the browser has WebGPU. The result looks the same as before, though individual pixels differ, and serpentine Sierra on a 10-megapixel image no longer waits on the CPU.
- a800b08: Update Typography's selector parser to address a security vulnerability while preserving generated prose styles.
- e35d4bb: Update SvelteKit, Svelte and their dependencies to address security vulnerabilities in request handling and serialization.
- b3c3328: Cap Adaptive sample size at 100 pixels so the useful range is easier to adjust.
- a9dbe74: Processing a newly loaded image keeps waiting for its effects when a reply for the previous image arrives.
- cd14e37: Keep indexed PNG downloads responsive by encoding in a dedicated worker. Cancel superseded exports while preserving the displayed result and exact palette transparency.
- 0d14783: Toggling effects on the Source half no longer recompiles a chain it just showed. The last eight successful results for the image are kept, and turning every effect off keeps the Source canvas and its uploaded colour index for when they come back on.
- f4e7b49: Reuse the saved-image database connection across reads and writes, and recover after connection closure or a failed open.
- 98d9860: Use exact GPU channel tables for eligible image effects in Source previews and output processing. Preserve alpha, fall back to CPU for unsupported chains or GPU failures, and discard results superseded by newer edits.
- 99aa38a: Match Wplace's Teal, #10AEA6, in the built-in Wplace palette. Saved Teal visibility and a Teal matte carry over to the corrected colour.
- Updated dependencies [684afb0]
- Updated dependencies [c101901]
- Updated dependencies [8aa3343]
- Updated dependencies [8fb1e39]
- Updated dependencies [5677848]
- Updated dependencies [b470df6]
- Updated dependencies [5bf4716]
- Updated dependencies [ff9b06d]
- Updated dependencies [29a6ca6]
- Updated dependencies [5671f7f]
- Updated dependencies [0887255]
- Updated dependencies [2de4a29]
- Updated dependencies [e0ac855]
- Updated dependencies [059a318]
- Updated dependencies [e0ac855]
- Updated dependencies [5c23184]
- Updated dependencies [1c75fa0]
- Updated dependencies [54f9336]
- Updated dependencies [b680b9c]
- Updated dependencies [a465a44]
- Updated dependencies [a5df85d]
- Updated dependencies [27cacac]
- Updated dependencies [52ee928]
  - ditherette@1.0.0

## 0.1.0

### Minor Changes

- 9958c4d: Offer every resize, dither, and color matching option the package supports: bicubic, trilinear, and resize anchors; blue noise, Atkinson, and Yliluoma ordered mixing; and YCbCr, CIEDE2000, OKLCH, and CIELCh matching variants.
- e4055aa: Every studio window collapses and expands from its tab bar, docked or floating, and stays collapsed across reloads. The default layout now puts Dimensions, Dither, Palette, and Color space on the left and Effects on the right.
- 6d28bd4: Curves work in any colour model: RGB, linear RGB, HSL, HSV, OKLab, OKLCH, CIELAB, CIELCh, or YCbCr, with checkboxes named after the model's channels. Arbitrary XY lets any channel on x adjust any channel on y, such as hue against saturation, with a wrapping hue axis.
- 9f762e2: Effect panels show what they do: tone graphs for Levels, Brightness and contrast, and Exposure, before and after hue strips for Hue and saturation, and gradient slider tracks. Curves keeps a curve per channel, edited through Red, Green, and Blue checkboxes, with all three curves drawn together.
- 6891994: Effects run once, through one exact table shared by the pipeline and the Source preview. The preview now shows exactly what the export uses, and editing mixing effects such as hue and saturation or palette fit is about 30% faster on large photos.
- e2c6956: Effects on the Source half update live while the output is still dithering. A worker of their own compiles each edit over the image's distinct colours, the output reuses that result, and the GPU draws only the visible part of the source at screen resolution.
- e757861: Mask any effect step. Each effect's window has a Mask section: Tones, Colours, and Saturation sliders write mask curves, and Curves edits them directly in the curves canvas. Show mask draws the step's mask in grey on the Source preview.
- 0bdd43b: Replace the top bar with a desktop-style menu bar. File, Edit, Image, View, and Window menus hold opening, exporting, cropping, adjustments, resample, dither, color space, zoom, theme, and window commands, with ⌘/Ctrl+O and ⌘/Ctrl+E shortcuts and bare keys for zoom. The logo is now the plain "ditherette" wordmark.
- 1d8f813: Add a Scopes window: histogram, waveform, parade, vectorscope, and CIE 1931 chromaticity, for the source, the output, or both. Scopes read RGB, CMY, HSL, HSV, YCbCr, OKLab, OKLCH, CIELAB, CIELCh, or linear RGB on the same axes as the curves editor.
- 170876a: Undo and redo processing settings with ⌘Z and ⇧⌘Z, or Ctrl+Z and Ctrl+Shift+Z / Ctrl+Y, and from the Edit menu. A slider or curve drag is one step. Opening an image or changing the crop starts a fresh history.
- 06310de: Show effects on the source: a preview toggle draws the Source half with the enabled effects applied at full resolution, so adjustments can be judged before the palette snaps them.
- aa9acda: Turn the website into an editor studio. Every processing stage is a window that docks, tabs, floats, and docks back, and the layout persists. Add an effect pipeline: each effect instance gets its own renamable window, and effects run on the source before resize and dithering.

### Patch Changes

- bf8ba75: The interface uses British spelling: colour, grey, centre, randomise, cancelled.
- bf2decf: Color space descriptions now say what each mode does to the image, including how LCh relates to Lab and why circular hue matches exactly like Lab. Formulas no longer crowd the cards.
- 92ec325: A sidebar button folds a whole sidebar into a strip, and the preview takes the freed width. Window carets only roll a window up to its tab bar. Effect windows open floating in the middle of the dock, and window titles follow the current names after a layout loads.
- e27f5fd: The dither dropdown describes each algorithm without its formula, and KaTeX is no longer bundled.
- f20494f: Dither previews show the real output: each one is the preview gradient dithered by the Wasm package with your current palette, color space, and dither settings. Atkinson, blue noise, and Yliluoma previewed as other algorithms before.
- 7370d63: The processing worker builds the output preview's zoom levels, so a new result no longer freezes the page while it is drawn.
- 698bafc: Show edits sooner. Each result is saved only once edits settle, a briefly busy worker keeps its loaded image and stage cache, and the zoomed-out preview is downsampled on the GPU.
- 1f1145d: Show effects on source updates in about 50 ms on a 24 MP photo, down from up to a second: the effect chain compiles into a 3D lookup table that the GPU applies to the full-resolution source.
- af5c082: Resizing images with transparency no longer leaves black halos, blobs, or bands around shapes with any kernel except nearest.
- Allow output scale and size inputs to upscale images within the output limits.
- Updated dependencies [ce7d4f1]
- Updated dependencies [b47d5d1]
- Updated dependencies [b8bc04a]
- Updated dependencies [af5c082]
- Updated dependencies [a732e42]
- Updated dependencies [797111c]
- Updated dependencies [aed3ff3]
- Updated dependencies [6c84ac9]
- Updated dependencies [c8798aa]
- Updated dependencies [f9fb235]
- Updated dependencies [6814494]
- Updated dependencies [640bb9a]
- Updated dependencies [a465fec]
- Updated dependencies [59c3054]
- Updated dependencies [fd11179]
- Updated dependencies [0d8754c]
- Updated dependencies [10055d1]
  - ditherette@0.1.0
