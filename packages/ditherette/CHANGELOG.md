# ditherette

## 1.0.0

### Major Changes

- 8aa3343: Release v1.0.0 with CPU and WebGPU processing through one factory, client-selected
  tiling and fallback, source-aware Adaptive placement, GPU effects and resize,
  all dither families, fast approximate GPU diffusion, and detailed processing timings.

### Minor Changes

- 5677848: Add optional asynchronous GPU dithering with complete Adaptive analysis, all palette-matching metrics and ordered/noise fields. Adaptive error diffusion uses GPU coverage with the existing CPU scan. Unsupported stages retain scalar fallback.
- 5bf4716: Add `diffusionBackend: 'gpu-approximate'`: error diffusion runs in parallel on the GPU and looks like the original scan, with different exact indices. Bands of rows carry their exact errors downward while row segments diffuse side by side, so serpentine no longer runs one pixel at a time. `gpu` keeps the original scan order and remains the GPU engine's default.
- ff9b06d: Add opt-in async GPU effects processing with exact channel tables, bounded staging/readback memory, and scalar fallback. Keep synchronous CPU processing unchanged.
- 29a6ca6: Compose GPU effects, resize and indexed processing through the package process API,
  with coverage-aware recipe-v2 filtering and prepared-source Adaptive reuse.
- 5671f7f: Add explicit GPU diffusion for all kernels, feedback modes and scan directions.
  Keep CPU diffusion as the default and preserve continuous error across dispatches.
- 0887255: Honor GPU row tiling for matching, independent fields and Yliluoma. Preserve global
  field phase and split large whole-image calls at the device dispatch limit.
- 2de4a29: Run mixed and masked pointwise effects, colour curves and explicit palette recipes
  on GPU with continuous colour between steps and client-selected image tiling.
- e0ac855: Add asynchronous GPU resize to the package for nearest, area, bilinear, trilinear, bicubic and both Lanczos filters. Preserve anchors and global filter halos across selectable image tiles, with mapped-buffer readback and explicit fallback policy.
- 059a318: Run every Yliluoma matrix size and matching policy on the optional GPU backend,
  including Adaptive placement, with bounded palette-tree storage and CPU fallback.
- e0ac855: Include CPU scalar, CPU threaded and GPU artifacts in normal package builds. GPU execution stays opt-in; importing and using CPU processing does not load GPU code. Validate all three artifacts and preserve the separate size-review and publication holds.
- 5c23184: Expose independent CPU row-band controls and explicit CPU-fallback or error policies on GPU facades. Clients can require GPU work and select automatic, whole-image or row-band CPU execution without changing rendering settings.
- a5df85d: Add versioned smooth-area placement with spatial sample size and perceptual contrast tolerance. Fine/coarse Oklab contrast distinguishes compact transitions from broad ramps. Source-aware reconstruction gives qualifying regions smooth contours and gradual dither density, suppresses isolated patches, and reaches the gradient side of lineart. Selected pixels retain each algorithm's ordinary texture. Existing Adaptive and Everywhere recipes remain unchanged.
- 27cacac: Expose standalone GPU perturbation and effect-mask previews with explicit CPU fallback and image tiling, preserving alpha and the existing synchronous CPU methods.
- 52ee928: Select CPU or GPU processing through `createDitherette({ engine: 'cpu' | 'gpu', tiling: true | false })`. Both engines provide the same processing operations. GPU calls are asynchronous and share one device across effects, resizing and dithering; CPU-only recipe analysis follows the explicit fallback policy. Existing CPU calls and compatibility GPU factories retain their defaults.

### Patch Changes

- 684afb0: Reduce smooth-area analysis work while preserving its exact mask, coverage decisions and rendered output. Skip already-minimal reconstruction distances and avoid repeated square roots in neighbourhood maxima. Memory limits, cancellation checks and legacy rendering stay unchanged.
- c101901: Fix explicit CPU row tiling for Adaptive fields and Yliluoma by sharing full-image coverage. Apply explicit row-worker settings to effects and mask previews, resolving recipes globally first. Support bounded dependent GPU diffusion batches without restarting error state. Existing automatic scheduling defaults remain unchanged.
- 8fb1e39: Keep smooth-area coverage nondecreasing at adjacent floating-point contrast tolerances by evaluating each smoothstep in double precision before rounding. Legacy Adaptive and Everywhere output stay unchanged.
- b470df6: Trim GPU Adaptive sweep work without changing coverage. Sweeps launch only tiles that touch the image, and each tile prepares its colour gates and neighbour distances before the serial steps.
- 1c75fa0: Expose host wall timings for GPU processing stages, Adaptive analysis and CPU fallback in execution metadata.
- 54f9336: Reduce Adaptive dithering CPU work while preserving its mask and rendered output.
- b680b9c: Reuse Adaptive coverage when changing dither algorithms, strength, palettes or matching spaces. Changed source pixels, image preparation and Adaptive controls invalidate it. Retained coverage uses one bit per pixel within the existing memory and cache limits.
- a465a44: Keep source Oklab coordinates in admitted temporary GPU storage during Adaptive analysis, avoiding repeated colour conversion in reconstruction. The coverage cache and CPU rendering stay unchanged.

## 0.1.0

### Minor Changes

- ce7d4f1: Add the `channel-curve` effect: any channel of any colour model on the x axis adjusts any channel on the y axis, like hue vs saturation or lightness vs saturation. A flat line through 0.5 is neutral, and a hue x axis wraps smoothly.
- b8bc04a: Add `indexColours`, `compileEffects`, and `applyCompiledEffects`: compile an effect chain once per distinct colour and map pixels through it, byte-exact with `applyEffects`, for live previews on the CPU or GPU.
- af5c082: Recipe version 2 now resizes by coverage. Filtering kernels weight colour by alpha, so colour hidden under transparent pixels no longer bleeds into edges, and bicubic and Lanczos no longer ring alpha into empty areas. Nearest and fully opaque images are unchanged, as is recipe version 1.
- a732e42: Add `effectMask` and `compileEffectMask`: show a step's mask as greys, read from the unrounded colour entering the step exactly as the step reads it.
- aed3ff3: Add grading effects: `curves`, `brightness-contrast`, `exposure`, `white-balance`, and `hue-saturation`. They compose with `levels` in any order through `applyEffects` and recipe version 2.
- c8798aa: Add `isEffect(value)`, which reports whether one effect step would be accepted, without loading Wasm. Hosts use it to vet saved steps before sending them.
- f9fb235: Add the `model-curves` effect: three curves, one per channel, in linear RGB, HSL, HSV, Oklab, OKLCH, CIELAB, CIELCh, or YCbCr. Each channel maps to 0–1 over a documented range, and hue curves wrap and fade out on near-greys.
- 6814494: Add ordered colour effects. `applyEffects` runs an effect chain and returns full-colour RGBA8. Recipe version 2 adds `effects` to `process`; they run on the source before resize, dithering, and quantization. The first built-in effect is `levels`. Version 1 recipes are unchanged.
- 640bb9a: Add palette-aware recolouring. `analyzeRecolour` derives an editable recipe from an image and a palette in the chosen working space, and the `recolour` effect applies it at any strength, or analyses automatically inside a chain. Analyses are cached, so later edits never re-analyse.
- a465fec: Add masks to every effect step. A step's optional `mask` holds up to 4 curves that read the colour entering the step; their values multiply into its strength per colour.
- 59c3054: Add two-input adjustment curves. Control grids select an adjustment from two original colour channels while keeping the existing one-input curve shape unchanged.
- fd11179: Replace `curves`, `model-curves`, and `channel-curve` with one ordered `curves` effect. Each curve now declares its remap or adjustment kind, input and output channels, and control points. Remaps use an open spline, while hue-input adjustments wrap across a closed seam.

### Patch Changes

- b47d5d1: Match colours with CIELAB ΔE2000 about 4.7× faster, with identical output. Each scan starts from the closest Lab colour and skips palette entries that exact lower bounds rule out.
- 797111c: Speed up Oklab Yliluoma dithering with an exact, memory-bounded mixture index.
- 6c84ac9: Apply hue and saturation about 5× faster on large photos with identical output. Pointwise effect chains now cache final bytes per colour in a table sized to the image, and no longer allocate a full-image float carrier.
- 0d8754c: Process new Yliluoma frames faster when the palette, matching, and matrix size stay the same. The processor keeps the mixture index between calls, so an 8×8 frame takes about 26% less time and a 16×16 frame about 44% less. Output is unchanged.
- 10055d1: Yliluoma mixing is fast in every colour space, with identical output: at 600×400 with the Wplace palette, 4×4 now takes about 140 ms in CIELAB (was 4.7 s), 380 ms in OKLCH (was 12.5 s), and 1.2 s in CIEDE2000 (was over a minute).
