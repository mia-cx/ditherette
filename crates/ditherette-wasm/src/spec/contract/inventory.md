# Version-one contract and oracle inventory

This is the S03 inventory at anchor `a213effed4b426c5c432c9ccc7062b7016dd5c1b`.
Paths below are relative to `crates/ditherette-wasm/src/` unless they start with `src/lib/`.
The inventory records inherited kernels and the references required before the S18 freeze.
An existing kernel is not evidence that a complete public operation already works.

## Authority

[Decision 23](https://github.com/mia-cx/ditherette/issues/23) defines ownership and exact reference conformance.
[Decision 34](https://github.com/mia-cx/ditherette/issues/34) includes every website mode and coherent existing Rust mode.
[Decision 35 and its addenda](https://github.com/mia-cx/ditherette/issues/35) define typed requests, results, errors, and callbacks.
[Decision 40](https://github.com/mia-cx/ditherette/issues/40) replaces `dither` with `perturb` and permits palette truncation.
The old process-contract prototype's rejection of palettes above 256 entries is superseded.
[Decisions 24](https://github.com/mia-cx/ditherette/issues/24), [36](https://github.com/mia-cx/ditherette/issues/36), and [37](https://github.com/mia-cx/ditherette/issues/37) define control, loading, and memory behavior.

## Requests and storage

`request.rs` defines explicit version-one settings. Each operation accepts cropped, tightly packed RGBA8.
The source remains borrowed and immutable. Successful results own separate storage through `image::ImageBuf`.
The JS boundary must validate raw property types and copy returned storage into durable JS-owned arrays.
Rust enum tags reject malformed palette forms and incoherent color/metric combinations.
`decode_recipe` rejects malformed recipe structure at `recipe`; `parse_match` gives a precise caller-supplied match path.
The later JS adapter supplies precise paths for raw JS property-type errors before constructing these typed requests.

| Method | Rust request | Result storage | Required reference composition |
| --- | --- | --- | --- |
| `process` | `ProcessRequest` | `image::contracts::IndexedImage` | S17 resize then fused dither/quantize |
| `resize` | `ResizeRequest` | `image::contracts::Rgba8Image` | S11 resize dispatch |
| `perturb` | `PerturbRequest` | `image::contracts::Rgba8Image` | S07/S08 conversion, S12 placement, S13/S14 field, inverse RGBA8 |
| `quantize` | `QuantizeRequest` | `image::contracts::IndexedImage` | S09 palette/alpha, S10 matching, S17 result composition |
| `ditherAndQuantize` | `DitherQuantizeRequest` | `image::contracts::IndexedImage` | S13/S14 perturb then quantize, or S15/S16 feedback/mixing |

The Rust fused request nests `QuantizeRequest` to reuse the shared fields. This is an internal representation, not a nested JS requirement.
Every processing call may receive an optional `onProgress` callback at execution time.
`InstanceModel::begin(progress_enabled)` models its presence. Callbacks, thread policy, cache keys, and frontend identifiers are absent from recipes.
There is no public stage graph, crop operation, or palette generator.

Indices address the retained input palette order, including duplicate colors.
`NormalizedPalette.rgba` contains four bytes per retained entry. `transparent_index` is `Option<u8>` and maps to JS `number | null`.
`IndexedImage` contains dimensions through its indexed buffer and includes ordered warnings.
RGBA8 results preserve their alpha bytes unless the operation itself resamples them.
S09 implements palette truncation, transparent-only output, and darkest-visible fallback with existing warning wording.
`WarningCode` names those three cases as `palette-truncated`, `transparent-only`, and `transparent-fallback`.

## Validation and units

| Field | Version-one boundary |
| --- | --- |
| Version | Exactly `1`; staged Rust requests also carry an explicit version |
| Source dimensions | Positive `u32`, at most 32,768 per side and 67,108,864 pixels |
| Output dimensions | Positive `u32`, at most 16,384 per side and 67,108,864 pixels |
| Methods without resize | Source dimensions must also satisfy output limits |
| Source storage | Exactly width × height × 4 bytes; no row padding at the package boundary |
| Palette | Nonempty ordered color/Transparent entries; retain first 256 with a warning in S09 |
| Palette/matte RGB | Three integer bytes; metadata labels and enabled flags belong to the caller |
| Preserve alpha threshold | Finite `0..=255`, including fractional values accepted by the website |
| Field/diffusion strength | Finite nonnegative factor; website percentage divided by 100 |
| Adaptive radius | Integer `1..=32768` source pixels; website adapter rounds and clamps its existing setting |
| Adaptive threshold/softness | Finite nonnegative values on the existing contrast × 100 scale |
| Random seed | Explicit `u32`; exact global-index sequence belongs to S13 |
| Initialization memory limit | Integer bytes in `1..=2 GiB`; default 1.5 GiB |

Dimension checks reject excess sizes. Host-side output sizing/clamping remains a browser responsibility.
The 75 MiB encoded-file limit is a decode constraint and does not apply to supplied RGBA8 bytes.
Settings and palette validation precede image output allocation. Successful validation creates only borrowed views and dimensions.
The memory preflight counts allocated private capacity, retained data, scratch, and boundary copies.
It excludes original and returned JS buffers and fixed module overhead.
Cache retention is at most 128 entries and `min(256 MiB, memoryLimitBytes / 4)`.

`error.rs` fixes these serialized codes:

```text
invalid-request  invalid-image  invalid-palette  invalid-settings
unsupported-operation  capability  initialization  memory-limit
wasm-memory-unavailable  disposed  reentrant-call  callback  runtime
```

Errors carry a stable path and explanatory message. Examples include `recipe.version`, `source.data`, `palette`, and `alpha.threshold`.
Processing failures expose no partial result. A callback failure also publishes no new cache entries.
Expected failures leave the instance usable; processing after disposal fails with `disposed`.

## Resize modes and exported kernels

Each row identifies the direct scalar oracle in `spec/resize/scalar/`.
All anchored modes support top-left, top, top-right, left, center, right, bottom-left, bottom, and bottom-right.
Area integrates pixel footprints and therefore has no anchor.

| Recipe | Website or extra | Kernel and support |
| --- | --- | --- |
| Nearest | Website | `nearest::resize_nearest_into`; explicit anchor |
| Area | Website | `area::resize_area_into` |
| Bilinear | Website | `bilinear::resize_bilinear_into`; explicit anchor |
| Catmull-Rom bicubic | Extra | `bicubic::resize_bicubic_into`; fixed and scale-aware |
| Lanczos2 | Website | `lanczos::resize_lanczos2_into`; fixed and scale-aware |
| Lanczos3 | Website | `lanczos::resize_lanczos3_into`; fixed and scale-aware |
| Trilinear | Extra | `trilinear::resize_trilinear_into`; area mip pyramid, ceil-halving, max-axis LOD, bilinear levels, final blend |

`lanczos::resize_lanczos_into` is the radius-parameterized adapter for Lanczos2/3.
`convolution::resize_convolution_into` directly applies the public `ReconstructionKernel` recipe.
`common/alignment::{map_axis_coordinate, ResizeAnchor::axes}` and `common/coordinates::map_axis_position` define discrete and continuous mapping.
`common/sample::ResizeSample` owns sample conversion, including rounding.
S11 audits those helpers, exported kernels, and strided internal images before freeze.

Inherited production resize adapters map to the same complete-image oracle, or the corresponding output rows:

| Production module | Exported adapter names | Reference obligation |
| --- | --- | --- |
| `prod/resize/scalar/nearest` | `resize_nearest_rgba8_into`, `resize_nearest_rgba8_rows_into`, `resize_nearest_rgba8_with_plan_into`, `resize_nearest_rgba8_rows_with_plan_into` | Nearest oracle; planned and row variants preserve global coordinates |
| `prod/resize/scalar/area` | `resize_area_rgba8_into`, `resize_area_rgba8_rows_into`, `resize_area_rgba8_with_plan_into`, `resize_area_rgba8_rows_with_plan_into` | Area oracle |
| `prod/resize/scalar/bilinear` | `resize_bilinear_rgba8_into`, `resize_bilinear_rgba8_rows_into`, `resize_bilinear_rgba8_with_plan_into`, `resize_bilinear_rgba8_rows_with_plan_into` | Bilinear oracle |
| `prod/resize/scalar/bicubic` | `resize_bicubic_rgba8_into`, `resize_bicubic_rgba8_rows_into`, `resize_bicubic_rgba8_with_plan_into`, `resize_bicubic_rgba8_rows_with_plan_into` | Catmull-Rom oracle and selected support |
| `prod/resize/scalar/lanczos` | `resize_lanczos_rgba8_into`, `resize_lanczos_rgba8_rows_into`, `resize_lanczos_rgba8_with_plan_into`, `resize_lanczos_rgba8_rows_with_plan_into` | Parameterized Lanczos oracle |
| `prod/resize/scalar/lanczos` | `resize_lanczos2_rgba8_into`, `resize_lanczos3_rgba8_into`, `resize_lanczos2_rgba8_rows_into`, `resize_lanczos3_rgba8_rows_into` | Lanczos2/3 oracle and selected support |
| `prod/resize/scalar/convolution` | `resize_convolution_rgba8_into`, `resize_convolution_rgba8_rows_into`, `resize_convolution_rgba8_with_plan_into`, `resize_convolution_rgba8_rows_with_plan_into` | Direct convolution recipe |

`NearestResizePlan`, `AreaResizePlan`, `BilinearResizePlan`, `BicubicResizePlan`, `LanczosResizePlan`, and `ConvolutionResizePlan` are private preparation adapters.
Their reference composition is the matching direct filter without preparation. Their cache ownership belongs to the control model and later runtime slices.
Production alignment helpers map to the spec coordinate helpers above. Storage assertions under `prod/resize/common/rgba8` map to shared image layout validation.

## Color spaces and matching

The public `MatchPolicy` tag fixes both coordinates and metric. Every reversible space also works as a perturbation space.
All color buffers use packed f32 triples with separate byte alpha in the completed reference.

| Space | Valid match tags | Forward oracle in `spec/color/`; inverse owner |
| --- | --- | --- |
| Gamma sRGB | `srgb-euclidean`, `srgb-compuphase`, `srgb-rec601`, `srgb-rec709` | `srgb::rgba8_to_srgb32_into`; S07 |
| Linear sRGB | `linear-rgb-euclidean` | `linear::rgba8_to_linear_rgb32_into`; S07 |
| Oklab | `oklab-euclidean` | `oklab::rgba8_to_oklab32_into`; S08 |
| OKLCH | `oklch-euclidean`, `oklch-circular-hue`, `oklch-hue-arc` | `oklch::rgba8_to_oklch32_into`; S08 |
| D65 CIELAB | `cielab-euclidean`, `cielab-ciede2000` | `cielab::rgba8_to_cielab32_into`; S08 |
| CIELCH | `cielch-euclidean`, `cielch-circular-hue`, `cielch-hue-arc` | `cielch::rgba8_to_cielch32_into`; S08 |
| Full-range BT.601 YCbCr | `ycbcr-euclidean` | `ycbcr::rgba8_to_ycbcr32_into`; S07 |

The website's `weighted-rgb`, `weighted-rgb-601`, and `weighted-rgb-709` map to CompuPhase, Rec.601, and Rec.709 sRGB matching.
Its CIELAB mode uses Euclidean matching; CIEDE2000 is an extra. Its OKLCH mode maps to `oklch-hue-arc`.
The website uses a minimum-chroma weighted shortest arc. The existing `*-circular-hue` tags retain the geometric-mean chroma chord.
Euclidean LCH compares the existing three coordinates directly. It is distinct from circular hue and is never silently substituted.
CIEDE2000 accepts CIELAB only. Weighted RGB metrics accept gamma sRGB only.

`spec/quantize/metric` exports `euclidean3_squared`, `circular_hue3_squared`, `hue_arc3_squared`, `weighted_rgb_squared`, and `ciede2000_distance`.
`spec/color/lab_ciede2000::ciede2000` contains the latter formula.
`spec/quantize/nearest_color` exports `quantize_euclidean3_into`, `quantize_circular_hue3_into`, `quantize_weighted_rgb_into`, and `quantize_ciede2000_into`.
The single-color adapters are `nearest_euclidean3_index`, `nearest_circular_hue3_index`, `nearest_weighted_rgb_index`, and `nearest_ciede2000_index`.

S10 adds complete typed `spec::quantize::quantize` composition and `matcher::PaletteMatcher` for all 15 valid pairs.
`metric::distance_score` selects the exact metric recipe, including both chord and arc hue behavior.
`color::rgb8_to_coordinates` and `color::coordinates_to_rgb8` dispatch the seven per-space byte conversions.
The matching adapters retain the first palette entry on an exact distance tie. S09/S10 exclude Transparent while retaining original output indices.

`spec/color/common` exports the component formulas `srgb8_to_unit`, `srgb_unit_to_linear`, `linear_to_srgb_unit`, `srgb8_to_linear`, `linear_srgb_to_xyz`, `xyz_to_cielab`, `cartesian_to_cylindrical`, `srgb8_to_oklab`, `linear_srgb_to_oklab`, and `srgb8_to_cielab`.
S07/S08 complete inverse formulas, neutral hue, coordinate domains, clipping, and byte reconstruction before freeze.

## Dither, placement, and feedback

| Mode | Website or extra | Existing oracle in `spec/dither/` | Completion owner |
| --- | --- | --- | --- |
| None | Website | `spec/quantize/nearest_color` | S10 |
| Bayer 2, 4, 8, 16 | Website | `ordered::{dither_bayer_into, dither_bayer_by_nearest_into, bayer_value}` | S13 palette-free RGBA8 field |
| Seeded random | Website | `random_noise::{dither_random_noise_into, dither_random_noise_by_nearest_into, Mulberry32}` | S13 exact global-index sequence and draw count |
| Blue noise | Extra | `blue_noise::{dither_blue_noise_into, dither_blue_noise_by_nearest_into}` | S14 repaired fixed asset and generator |
| Floyd-Steinberg, Sierra, Sierra Lite | Website | `error_diffusion::{dither_error_diffusion_into, dither_error_diffusion_by_nearest_into, ErrorDiffusionKernel}` | S15 |
| Atkinson | Extra | Same diffusion oracle | S15 |
| Two-color ordered Yliluoma, Bayer sizes 2/4/8/16 | Extra | `yiluoma::{dither_yiluoma_into, dither_yiluoma_by_distance_into, best_ordered_mix, best_ordered_mix_by_distance}` | S16 |
| Everywhere/adaptive placement | Website, extended to Yliluoma | Existing website `src/lib/processing/quantize-shared.ts::placementMask` | S12 fixed domain ranges and eight clamped neighbors |

`spec/dither/common` exports `assert_dither_inputs`, `read_color`, `add_scaled_noise`, `add_error`, `sub_color`, `nearest_euclidean`, `squared_distance`, and `write_index`.
These compose the named formulas above. They are semantic helpers and belong to the frozen content audit.
`Mulberry32::{new, next_u32, next_f32}` currently describes a sequential generator. S13 must fix global pixel identity before freeze.

Separable `DitherPolicy` contains its own `WorkingSpace`, independently of matching.
Weighted RGB matches use sRGB perturbation. `perturb` accepts no palette or diffusion/mixing modes.
Its result crosses the explicit rounded/clipped RGBA8 boundary before matching.
S17 must compare indices, normalized palette, transparency, and warnings for both equalities:

```text
process(input) = ditherAndQuantize(resize(input))
ditherAndQuantize(input, separable) = quantize(perturb(input))
```

Diffusion uses explicit `feedback: "srgb-bytes" | "matching"` tags.
Byte feedback rounds/clips sRGB before matching and residual calculation; matching feedback keeps unrounded coordinates.
The former `space` field was ambiguous when matching also used sRGB and is rejected before freeze.
With palette red bytes 0/2 and source red bytes 1/1, Floyd strength 1 produces `[0,0]` for bytes and `[0,1]` for matching.
This distinction preserves both website `useColorSpace` paths and the inherited unrounded coordinate kernels.
Read `src/lib/processing/quantize-shared.ts::supportsVectorDither` and `src/lib/processing/quantize-algorithms/error-diffusion-runner.ts::quantizeErrorDiffusion` when implementing S15 or S28.
The latter calls `matcher.nearestIndexByteRgb` after byte clamping while scattering sRGB error when vector dithering is disabled.
`quantizeVectorErrorDiffusion` instead matches and scatters in working coordinates.
Adaptive placement in the website reads the selected matching coordinates even when diffusion uses sRGB.
The reference completion must preserve this distinction while making placement palette-independent.

Diffusion scan reversal is explicit. Transparent preserved pixels drop incoming error and emit none.
The spec may use full-image scratch. Production uses bounded three-row error scratch and remains scalar.
Yliluoma interpolates coordinates componentwise, including hue. S16 applies adaptive placement to its target before pair selection.

## Executable adapters and control

| Existing adapter/export | Named reference or completion obligation |
| --- | --- |
| `wasm::hello` | Literal greeting sanity export; remove from public npm exports in S30 |
| `wasm::convert_color_space` | Selected forward color oracle plus allocation/copy composition; inherited f32x4 output needs S24's packed-triple correction |
| `wasm::resize_rgba8` | Matching resize oracle, settings validation, allocation, output ownership |
| `wasm::process_rgba8` | Currently resize-only; S17 complete process composition, then S30 production boundary |
| `wasm::benchmark_color_space` | Same color operation plus calibration/timing/checksum; not a public package method |
| `wasm::benchmark_resize_rgba8` | Same resize operation plus calibration/timing/checksum; not a public package method |
| `wasm_bindgen_rayon::init_thread_pool` | `lifecycle::initialize` selection; S17/S34 pool lifecycle model/implementation |
| `bench_subjects::bench_subjects` | Every descriptor selects its named spec oracle; S05 extends beyond resize |
| `prod/color::rgba8_to_color_space_f32`, `_into`, `_with_policy_into`, `_rows_into`, `_parallel_with_band_height_into` | Forward conversion oracle; complete output or selected global rows; S17 execution composition |
| `prod/color::{ColorSpaceF32::parse, ColorTilingPolicy::for_request}` | Typed space selection and scalar-equivalent row partition; thread count cannot alter semantics |
| `prod/tiling::{RowBand, RowBandPlan}` | `spec/tiling/contract::{RowBand, RowBandPlan}` covers each output row exactly once |
| `prod/tiling::{Tile, TileGrid, WorkerBudget, RowBandWorkPlan, RowBandWorkAssignment}` | S17 reference partition/work-assignment model; coordinates and bounds derive from complete output |
| `prod/tiling/executor::{for_each_row_band, for_each_tile}` | S17 sequential application of each disjoint output partition; compare with whole-image oracle |

Production color names in the table share the `rgba8_to_color_space_f32` prefix.
`wasm.rs` also has private scalar, pooled-direct, pooled-copy, and pooled-noop adapters, row dispatch, filter/support/anchor parsers, and plan scopes.
The scalar/direct adapters compose the corresponding complete-image or row oracle.
Pooled copy preserves input storage and pooled noop preserves initialized output. Neither is an image-processing mode.
S17 gives these executable compositions named references before S18 audits the exported and indirect semantic dependencies.
Calibration, clocks, sample aggregation, and checksums remain benchmark bookkeeping; their correctness does not establish processing conformance.

The current subject registry includes spec/prod nearest, area, bilinear, bicubic fixed/scale-aware, Lanczos2 fixed/scale-aware, and Lanczos3 fixed/scale-aware.
It also includes spec-only trilinear. This is 19 subjects, all mapped to the resize table above.
Thread counts, row-band sizes, pooled modes, plan scopes, and batch sizes are execution parameters, never new semantic recipe tags.

`lifecycle.rs` is the readable S03 reference for initialization fallback, memory preflight, isolated instance state, disposal, and callback/publication ordering.
Stage changes report immediately; repeated-stage events wait at least 50 ms. Cache hits can skip stages.
Completion follows ready result bytes and metadata. A successful completion callback precedes successful cache publication.
Callback reentry and disposal fail without disrupting the active operation. Thrown callbacks end the call with `callback`.
Disposal releases instance-owned allocations; Wasm pages may remain at their high-water mark until module collection.
The host owns worker termination and stale-result rejection. Cancellation is not a hidden asynchronous package method.

## Remaining pre-freeze work

S07/S08 define inverse reconstruction and coordinate domains. S09/S10 complete palette/alpha/metric compositions.
S11 completes resize edges and the export audit. S12 defines fixed placement ranges and preserves metric/feedback distinctions.
S13 fixes random identity. S14 replaces the transposed-Bayer blue-noise table with its documented generator and accepted asset.
S15/S16 complete feedback and adaptive mixing. S17 connects all methods and finishes executable adapter/control references.
S18 freezes that complete reference, including this mode inventory and semantic shared-storage dependencies.
These are assigned implementation obligations. They do not authorize production to call spec as its implementation.
