# Private field bindings

These module functions share the existing instance slot, lifecycle, error codes, and caught copy helpers.
They introduce no generated class, owned slice input, or owned JsValue return.
All numeric parameters enter as f64 for validation before narrowing. Inputs and result sinks are borrowed externrefs.

```text
privatePerturb(input, width, height,
  field, parameter, space, strength, placement, radius, threshold, softness,
  resultSink) -> u32
privateDitherAndQuantize(input, width, height,
  palette, matching, alphaMode, alphaThreshold, matte, family,
  field, parameter, space, strength, placement, radius, threshold, softness,
  resultSink) -> u32
```

Field 0 is Bayer, with parameter 2, 4, 8, or 16. Field 1 is random, with a u32 seed parameter.
Field 2 is blue noise and requires parameter zero for the frozen 32×32 rank tile.
Spaces 0 through 6 are sRGB, linear RGB, Oklab, OKLCH, CIELAB, CIELCH, and YCbCr, respectively.
Placement 0 is everywhere and requires zero unused controls. Placement 1 is adaptive.
Family 0 is direct quantization and requires all eight field controls to be zero. Family 1 is separable perturbation.
Family 2 is scalar diffusion. It uses field for kernel 0 Floyd-Steinberg, 1 Sierra, 2 Sierra Lite, or 3 Atkinson.
For family 2, parameter selects feedback 0 sRGB bytes or 1 matching coordinates, and space selects serpentine 0 false or 1 true.
Strength and placement retain their existing positions. Numeric tags require exact integer values before narrowing.
Palette, matching, alpha mode, alpha threshold, and matte retain the existing privateQuantize encodings.

Status zero guarantees complete publication into the private plain resultSink.value.
Nonzero status uses the existing allocation-free status table and privateErrorPath read immediately afterward.
Paths 17 through 25 append perturb, field, space, strength, placement, radius, threshold, softness, and dither.
Paths 26 through 33 append dither strength, placement, radius, threshold, softness, kernel, feedback, and serpentine.
Paths 34 and 35 both map publicly to dither.arithmetic, distinguishing non-finite work from non-finite matching distance.
The public adapter prefixes perturb paths with dither for the combined method.
No earlier error code or path changes.

Capacity checks include source/output buffers, owned records, and one temporary existing Converter with its tables.
Separable calls also count the RGBA8 intermediate and the existing prepared quantizer capacities.
Diffusion calls count the existing prepared quantizer, three source-initialized f32 work rows, and scalar scan controls.
All processing buffers are reserved before the caught source import. Failure drops temporary ownership and restores Ready.
The final caught void helpers construct the complete durable JS result before assigning the sink.
No cache or callback publication is added here.

Private fixtures derive exact budgets from the current artifact, not a hardcoded structure size.
The repeated-failure fixture verifies constant externref slots, live handles, and Wasm memory high-water capacity across 512 cycles.
The separable literal baseline still constructs Converter for each source-color read. Its call-owned reuse remains a separate candidate.
