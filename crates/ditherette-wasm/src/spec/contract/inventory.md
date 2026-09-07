# Version-one contract and oracle inventory

This inventory includes S07 through S16 and the S17 executable-adapter references.
Its integration base is `4f4b48a04c0ccee51222b7e621ff4a566a495613`.
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
`decode_recipe` rejects unsupported typed tags and unknown fields at `recipe`; `parse_match` gives a caller-supplied match path.
Serde also accepts alternate map/sequence representations for enums. The package boundary rejects those raw JS property types before decoding.
Settings records must be objects, and scalar enum tags must be strings. The JS adapter supplies precise property-error paths.

| Method | Rust request | Result storage | Required reference composition |
| --- | --- | --- | --- |
| `process` | `ProcessRequest` | `image::contracts::IndexedImage` | `pipeline::process`; resize then dither/quantize |
| `resize` | `ResizeRequest` | `image::contracts::Rgba8Image` | `spec::resize::resize`, complete naive dispatch |
| `perturb` | `PerturbRequest` | `image::contracts::Rgba8Image` | `pipeline::perturb`; validated `dither::perturb::perturb` composition |
| `quantize` | `QuantizeRequest` | `image::contracts::IndexedImage` | `quantize::quantize`, `palette::PreparedPalette`, `quantize::matcher::PaletteMatcher` |
| `ditherAndQuantize` | `DitherQuantizeRequest` | `image::contracts::IndexedImage` | `pipeline::dither_and_quantize`; none, separable, diffusion, or Yliluoma |

The Rust fused request nests `QuantizeRequest` to reuse the shared fields. This is an internal representation, not a nested JS requirement.
The pipeline references and executable processor are integrated with these adapters.
Read `spec/pipeline/README.md` for stage and callback composition.
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
S11 audits those helpers and exports through the complete request. `tests/spec_resize_contract.rs` covers anchors, support policies, odd/fractional/anisotropic mips, and padded logical rows. Lanczos kernel fixtures independently check half-angle sine values.

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
S07/S08 complete inverse formulas, neutral hue, coordinate domains, clipping, and byte reconstruction.
Their per-pixel exports are `rgb8_to_<space>([u8; 3]) -> [f32; 3]` and `<space>_to_rgb8([f32; 3]) -> [u8; 3]`.
Each image inverse reads separate RGBA8 alpha and copies those bytes unchanged.
Public OKLCH/CIELCH forward conversion gives exact byte grays zero chroma and hue.

`color::coordinates_to_rgb8` dispatches those f32 inverse recipes.
`color::reconstruct::{coordinates_to_rgb8, coordinates_to_srgb}` instead accepts f64 scalar coordinates for perturbation.
It keeps legal finite strengths from overflowing intermediate inverse formulas, without allocating f64 image planes.
Source triples remain f32. The wide inverse and ordinary f32 inverse can differ at byte-rounding boundaries.
Their roles stay explicit; dispatch consolidation must not replace the wide field inverse with f32 arithmetic.

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
`Mulberry32::{new, next_u32, next_f32}` remains the inherited sequential generator for generic indexed kernels.
`random_noise::{random_u32_at, random_noise_at}` supplies the palette-free field from seed and global pixel index.
Each pixel owns one draw, including transparent and zero-strength pixels, without mutable draw state.
The field's f64 centering precedes f32 conversion; legacy `next_f32` rounds before centering in its callers.
These are distinct reference exports, not interchangeable implementations of one recipe.

`placement::{coordinate_domain, placement_distance, contrast_at, placement_mask_at}` supplies fixed-domain adaptive placement.
The domain depends only on `WorkingSpace`. Cylindrical placement uses minimum chroma times wrapped angle, not circular-chord matching.
`perturb::{perturb_into, perturb_rows_into, perturb_by_field_rows_into}` preserves global rows and the RGBA8 reconstruction boundary.
The field scale is one quarter of the fixed coordinate range. The output preserves alpha bytes and processes hidden RGB.
`blue_noise::blue_noise_at` reads the generated 32x32 tile. Its generator, asset, and analysis artifact belong to the freeze.

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
Yliluoma interpolates coordinates componentwise, including hue, and adapts its target before pair selection.
Pairs, ratios, and first ties retain inherited enumeration even at zero placement.
For palette `[black, gray128, gray64]` and source `gray64`, zero placement with Bayer2 yields indices `[1,0,0,1]`.
The earlier half-mixture ties the nearest entry. Zero placement therefore does not promise flat nearest output.

## Executable adapters and control

| Existing adapter/export | Named reference or completion obligation |
| --- | --- |
| `wasm::hello` | `adapters::legacy_hello`; remove from public npm exports in S30 |
| `wasm::convert_color_space` | `adapters::{legacy_convert_color_space, legacy_color_space, legacy_color_rows_into}`; legacy f32x4, normalized alpha |
| `wasm::resize_rgba8` | `adapters::{legacy_resize_rgba8, LegacyResize}`; legacy parsing, allocation, and direct resize references |
| `wasm::process_rgba8` | `adapters::legacy_process_rgba8`; resize-only app-JSON shell, distinct from the version-one indexed process |
| `wasm::benchmark_color_space` | `legacy_color_rows_into`, `diagnostic_color_copy_into`, or `diagnostic_noop`; timing remains bookkeeping |
| `wasm::benchmark_resize_rgba8` | `LegacyResize`, `diagnostic_resize_copy_into`, or `diagnostic_noop`; timing remains bookkeeping |
| `wasm_bindgen_rayon::init_thread_pool` | `lifecycle::initialize` selection; S17/S34 pool lifecycle model/implementation |
| `bench_subjects::bench_subjects` | Five callable typed processing references and seven packed-f32 color/inverse references, plus the inherited resize subjects |
| `prod/color::rgba8_to_color_space_f32`, `_into`, `_with_policy_into`, `_rows_into`, `_parallel_with_band_height_into` | `adapters::legacy_color_rows_into`; whole output or selected global rows in f32x4 |
| `prod/color::{ColorSpaceF32::parse, ColorTilingPolicy::for_request}` | `adapters::legacy_color_space` and the same global-row composition; empirical policy does not change reference pixels |
| `prod/tiling::{RowBand, RowBandPlan}` | `spec/tiling/contract::{RowBand, RowBandPlan}`, including `for_output_height` |
| `prod/tiling::{Tile, TileGrid, WorkerBudget, RowBandWorkPlan, RowBandWorkAssignment}` | Same-named `spec/tiling/execution` models; early workers receive remainder bands |
| `prod/tiling/executor::{for_each_row_band, for_each_tile}` | Same-named sequential spec visitors preserve order and return the first callback error |

Production color names in the table share the `rgba8_to_color_space_f32` prefix.
`wasm.rs` also has private scalar, pooled-direct, pooled-copy, and pooled-noop adapters, row dispatch, filter/support/anchor parsers, and plan scopes.
`LegacyResize::resize_rows_into` computes the complete naive image and copies the requested global rows to a local band buffer.
`LegacyResize::pooled_direct_into` visits those bands sequentially. Full-image and per-band prepared plans share those reference pixels.
It retains the inherited pooled-direct filter restriction to nearest, bicubic, and Lanczos2/3.
`LegacyResize::parse` preserves diagnostic support-string aliases and validation order.

The legacy color reference retains raw Cartesian-to-cylindrical arithmetic, rather than public neutral canonicalization.
RGB8 `[8,8,8]` has legacy CIELCH chroma about `8.024521e-6` and hue about `2.7610862` radians.
The public forward recipe gives that gray zero chroma and hue. Legacy storage has a fourth normalized alpha channel.

`diagnostic_color_copy_into` casts raw RGBA bytes to f32 without normalization.
`diagnostic_resize_copy_into` repeats raw source storage, including padding, using `(global_y * band_byte_len) % source_len`.
The shortened final band changes that offset. The inherited usize expression retains target/build overflow behavior.
`diagnostic_noop` preserves initialized output, including floating-point payload bits.
These diagnostics are not processing recipes and need not be invariant under band-size changes.
Calibration, clocks, sample aggregation, and checksums remain benchmark bookkeeping; their correctness does not establish processing conformance.

The inherited subject registry includes spec/prod nearest, area, bilinear, bicubic fixed/scale-aware, Lanczos2 fixed/scale-aware, and Lanczos3 fixed/scale-aware.
It also includes spec-only trilinear. These 19 inherited subjects map to the resize table above.
The complete registry adds 12 callable conformance subjects for the five methods and seven color/inverse pairs.
`bench_subjects/reference.rs` maps each typed request to this inventory and preserves every relevant setting in verification identity.
Color records retain packed coordinates, byte alpha, and actual inverse-rendered RGBA8.
Thread counts, row-band sizes, pooled modes, plan scopes, and batch sizes are execution parameters, never new semantic recipe tags.

`lifecycle.rs` is the readable S03 reference for initialization fallback, memory preflight, isolated instance state, disposal, and callback/publication ordering.
Stage changes report immediately; repeated-stage events wait at least 50 ms. Cache hits can skip stages.
Completion follows ready result bytes and metadata. A successful completion callback precedes successful cache publication.
Callback reentry and disposal fail without disrupting the active operation. Thrown callbacks end the call with `callback`.
Disposal releases instance-owned allocations; Wasm pages may remain at their high-water mark until module collection.
The host owns worker termination and stale-result rejection. Cancellation is not a hidden asynchronous package method.

## Remaining pre-freeze work

The five-method `pipeline` references and `pipeline::processor::Processor` are joined with these adapters.
The processor's focused fixtures cover composition equalities, reentry, disposal, callback failures, and runtime-error recovery.
The joined validation must retain palette order, transparency, ordered warnings, and the exact 50 ms progress boundary.

`contract/cache.rs` is assigned to the parallel S17 cache-model work and remains pending integration here.
It must cover normalized stage identity and digest, private capacity accounting, scratch-first eviction, and LRU retention.
Operation keys and intermediate content identities remain distinct so composed and standalone methods can share actual outputs.
Color content identities include separate alpha, including when RGB triples match but alpha differs.
Pending cache entries publish atomically only after successful completion callbacks; failed calls publish none.
Allocation failure, disposal, and retained-capacity limits remain cache-model integration checks.
`InitOptions::preflight` accepts a supplied byte count; it does not prove that an operation counted every allocation.
Production slices must demonstrate concrete allocation accounting against the frozen capacity/ownership model as their implementations change.

The final reference subjects now use S05's verifier and actual inverse-rendered color output.
Settings identity distinguishes perturb and matching spaces, feedback modes, palette order, and complete recipe settings.
Reference records remain pre-freeze; missing accepted/candidate implementations remain explicit.
S18 freezes that complete reference, including this mode inventory and semantic shared-storage dependencies.
These are assigned implementation obligations. They do not authorize production to call spec as its implementation.
