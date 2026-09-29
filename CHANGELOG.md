# ditherette-web

## 0.1.0

### Minor Changes

- 9958c4d: Offer every resize, dither, and color matching option the package supports: bicubic, trilinear, and resize anchors; blue noise, Atkinson, and Yliluoma ordered mixing; and YCbCr, CIEDE2000, OKLCH, and CIELCh matching variants.
- e4055aa: Every studio window collapses and expands from its tab bar, docked or floating, and stays collapsed across reloads. The default layout now puts Dimensions, Dither, Palette, and Color space on the left and Effects on the right.
- 6d28bd4: Curves work in any colour model: RGB, linear RGB, HSL, HSV, OKLab, OKLCH, CIELAB, CIELCh, or YCbCr, with checkboxes named after the model's channels. Arbitrary XY lets any channel on x adjust any channel on y, such as hue against saturation, with a wrapping hue axis.
- 9f762e2: Effect panels show what they do: tone graphs for Levels, Brightness and contrast, and Exposure, before and after hue strips for Hue and saturation, and gradient slider tracks. Curves keeps a curve per channel, edited through Red, Green, and Blue checkboxes, with all three curves drawn together.
- 0bdd43b: Replace the top bar with a desktop-style menu bar. File, Edit, Image, View, and Window menus hold opening, exporting, cropping, adjustments, resample, dither, color space, zoom, theme, and window commands, with ⌘/Ctrl+O and ⌘/Ctrl+E shortcuts and bare keys for zoom. The logo is now the plain "ditherette" wordmark.
- 1d8f813: Add a Scopes window: histogram, waveform, parade, vectorscope, and CIE 1931 chromaticity, for the source, the output, or both. Scopes read RGB, CMY, HSL, HSV, YCbCr, OKLab, OKLCH, CIELAB, CIELCh, or linear RGB on the same axes as the curves editor.
- 06310de: Show effects on the source: a preview toggle draws the Source half with the enabled effects applied at full resolution, so adjustments can be judged before the palette snaps them.
- aa9acda: Turn the website into an editor studio. Every processing stage is a window that docks, tabs, floats, and docks back, and the layout persists. Add an effect pipeline: each effect instance gets its own renamable window, and effects run on the source before resize and dithering.

### Patch Changes

- bf8ba75: The interface uses British spelling: colour, grey, centre, randomise, cancelled.
- bf2decf: Color space descriptions now say what each mode does to the image, including how LCh relates to Lab and why circular hue matches exactly like Lab. Formulas no longer crowd the cards.
- 92ec325: A sidebar button folds a whole sidebar into a strip, and the preview takes the freed width. Window carets only roll a window up to its tab bar. Effect windows open floating in the middle of the dock, and window titles follow the current names after a layout loads.
- e27f5fd: The dither dropdown describes each algorithm without its formula, and KaTeX is no longer bundled.
- f20494f: Dither previews show the real output: each one is the preview gradient dithered by the Wasm package with your current palette, color space, and dither settings. Atkinson, blue noise, and Yliluoma previewed as other algorithms before.
- 698bafc: Show edits sooner. Each result is saved only once edits settle, a briefly busy worker keeps its loaded image and stage cache, and the zoomed-out preview is downsampled on the GPU.
- 1f1145d: Show effects on source updates in about 50 ms on a 24 MP photo, down from up to a second: the effect chain compiles into a 3D lookup table that the GPU applies to the full-resolution source.
- af5c082: Resizing images with transparency no longer leaves black halos, blobs, or bands around shapes with any kernel except nearest.
- Allow output scale and size inputs to upscale images within the output limits.
- Updated dependencies [ce7d4f1]
- Updated dependencies [b47d5d1]
- Updated dependencies [af5c082]
- Updated dependencies [797111c]
- Updated dependencies [aed3ff3]
- Updated dependencies [6c84ac9]
- Updated dependencies [c8798aa]
- Updated dependencies [f9fb235]
- Updated dependencies [6814494]
- Updated dependencies [640bb9a]
- Updated dependencies [59c3054]
- Updated dependencies [fd11179]
- Updated dependencies [10055d1]
  - ditherette@0.1.0
