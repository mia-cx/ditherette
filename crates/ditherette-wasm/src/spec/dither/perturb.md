# Palette-free RGBA8 perturbation

`perturb` allocates owned RGBA8. `perturb_into` writes an equally sized view.
`perturb_rows_into` writes only its `RowBand`, retaining full-image source coordinates and neighborhoods.
These kernels accept request-validated inputs and policies. The public processor owns request validation and allocation limits.
Input and output image views may have different row strides; neither source nor output padding participates in the recipe.

## Recipe

Read source RGB bytes without alpha compositing. Evaluate the [Bayer](ordered.md) or [random](random_noise.md) field at global x/y.
Evaluate the palette-free [placement mask](placement.md) on the original source image.
Let f be the centered field threshold, s the normalized nonnegative strength, and m the mask.
Calculate f64 `amount=f*s*m*0.25`, in that multiplication order.
The 0.25 factor is the website's `COLOR_SPACE_THRESHOLD_SCALE` in [quantize-shared.ts](../../../../../src/lib/processing/quantize-shared.ts).

For zero amount, retain the original RGB bytes exactly.
Otherwise convert source RGB to the selected f32 working-space triple.
Promote each coordinate to f64 and add `amount*fixed_axis_range` to all three axes.
The scalar threshold is shared across axes; ranges supply the axis units.
Apply [wide scalar reconstruction](../color/reconstruct.md), clipping only the encoded RGB and rounding to byte values.
Copy the source alpha byte unchanged, including zero alpha and its perturbed hidden RGB.

Negative cylindrical chroma reconstructs as neutral, and positive-chroma hue wraps in radians.
Source gamut boxes supply field ranges, not intermediate clipping limits.
Finite strengths above one remain supported. Scalar f64 intermediates avoid overflow without f64 image planes.

Weighted RGB choices belong to matching. Their perturb policy selects sRGB working coordinates.
No palette, prepared palette, matching metric, matte, or alpha threshold is an input to this module.

## Explicit composition boundary

`quantize_after_perturb` materializes RGBA8 before calling its quantization callback.
The callback cannot inspect floating coordinates. It receives the actual clipped and rounded bytes, with unchanged source alpha.
Indexed alpha policies, palette normalization, matching, result metadata, and warnings belong to that later quantization step.
S17 supplies the completed S10 quantization function after joining its validated dependency.

Production separable fusion must equal this composition, including the RGBA8 boundary.
Passing unclipped field coordinates directly to a nearest matcher is a different operation.
The older generic indexed Bayer/random adapters remain separate reference exports with their original unscaled coordinate offsets.

## Global identity and field registration

The shared helper is `perturb_by_field_rows_into(source,output,space,strength,placement,rows,field)`.
The callback has type `Fn(u32,u32,u64)->f32`, receiving global x, global y, and `y*full_width+x`.
It returns a centered scalar threshold in `[-0.5,0.5]` and runs once for every written pixel, including zero-effect pixels.
Band boundaries, traversal order, source alpha, and placement never alter random draw assignment.

The `BlueNoise` branch temporarily reads the inherited 8x8 table, which is transposed Bayer and is not certified blue noise.
S14 owns the corrected asset and `blue_noise::blue_noise_at(x,y)->f32`.
At the validated S13/S14 join, replace the isolated legacy lookup with that helper.
The scalar composition and byte boundary remain the same; S14 provides its own asset and registration fixtures.

## Website adapter handoff

The website's raw-byte path uses a 64-byte noise scale, while this normalized-space recipe uses `255*0.25=63.75` bytes.
When S38 adapts that historical raw-RGB strength, multiply its normalized factor by `64/63.75` before choosing sRGB perturbation.
The color-space path already uses the 0.25 scale. It needs no scale correction.
This is frontend parameter conversion, not another public field mode.
The fixed-domain change and explicit byte boundary still apply; the adapter cannot restore palette-driven ranges or bypass reconstruction.
