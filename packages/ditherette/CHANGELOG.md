# ditherette

## 0.1.0

### Minor Changes

- ce7d4f1: Add the `channel-curve` effect: any channel of any colour model on the x axis adjusts any channel on the y axis, like hue vs saturation or lightness vs saturation. A flat line through 0.5 is neutral, and a hue x axis wraps smoothly.
- af5c082: Recipe version 2 now resizes by coverage. Filtering kernels weight colour by alpha, so colour hidden under transparent pixels no longer bleeds into edges, and bicubic and Lanczos no longer ring alpha into empty areas. Nearest and fully opaque images are unchanged, as is recipe version 1.
- aed3ff3: Add grading effects: `curves`, `brightness-contrast`, `exposure`, `white-balance`, and `hue-saturation`. They compose with `levels` in any order through `applyEffects` and recipe version 2.
- c8798aa: Add `isEffect(value)`, which reports whether one effect step would be accepted, without loading Wasm. Hosts use it to vet saved steps before sending them.
- f9fb235: Add the `model-curves` effect: three curves, one per channel, in linear RGB, HSL, HSV, Oklab, OKLCH, CIELAB, CIELCh, or YCbCr. Each channel maps to 0–1 over a documented range, and hue curves wrap and fade out on near-greys.
- 6814494: Add ordered colour effects. `applyEffects` runs an effect chain and returns full-colour RGBA8. Recipe version 2 adds `effects` to `process`; they run on the source before resize, dithering, and quantization. The first built-in effect is `levels`. Version 1 recipes are unchanged.
- 640bb9a: Add palette-aware recolouring. `analyzeRecolour` derives an editable recipe from an image and a palette in the chosen working space, and the `recolour` effect applies it at any strength, or analyses automatically inside a chain. Analyses are cached, so later edits never re-analyse.
- 59c3054: Add two-input adjustment curves. Control grids select an adjustment from two original colour channels while keeping the existing one-input curve shape unchanged.
- fd11179: Replace `curves`, `model-curves`, and `channel-curve` with one ordered `curves` effect. Each curve now declares its remap or adjustment kind, input and output channels, and control points. Remaps use an open spline, while hue-input adjustments wrap across a closed seam.

### Patch Changes

- b47d5d1: Match colours with CIELAB ΔE2000 about 4.7× faster, with identical output. Each scan starts from the closest Lab colour and skips palette entries that exact lower bounds rule out.
- 797111c: Speed up Oklab Yliluoma dithering with an exact, memory-bounded mixture index.
- 6c84ac9: Apply hue and saturation about 5× faster on large photos with identical output. Pointwise effect chains now cache final bytes per colour in a table sized to the image, and no longer allocate a full-image float carrier.
- 10055d1: Yliluoma mixing is fast in every colour space, with identical output: at 600×400 with the Wplace palette, 4×4 now takes about 140 ms in CIELAB (was 4.7 s), 380 ms in OKLCH (was 12.5 s), and 1.2 s in CIEDE2000 (was over a minute).
