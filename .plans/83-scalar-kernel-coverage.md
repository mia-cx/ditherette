# S41 scalar kernel coverage audit

This audit reads source at `5da82d1221f79c2ddcb517eba9f621206468bfdc` and retained slice reports. It runs no builds, tests, or measurements.
All requested algorithm families have frozen and production callable paths. Exhaustive native spec-versus-production timing remains incomplete.
The largest evidence-backed opportunity is repeated converter construction in field placement and Yliluoma target reads.
Those earlier optimization candidates remain unselected. Their reported gains are not shipped gains.

Paths below are relative to `crates/ditherette-wasm/src/`, unless another root appears.
`S` means `spec::`; `P` means `prod::`. Function names describe actual callable paths, not proposed exports.
The frozen revision is `cef2b60a635fd43c3b8e7cb880b5c92fe77d640b`.
The recorded frozen content digest is `17ba3be371e8491de2cb3faf51aef474868fd93391f8c77850a755b92cddbebe`.
This audit does not rerun the freeze guard or rehash historical artifacts.

## How to read timing coverage

| Comparison | Meaning | Existing examples |
| --- | --- | --- |
| Spec / production | Frozen algorithm and production algorithm both timed with declared comparable ownership | Generic resize registry supports this directly; typed native operations currently do not |
| Literal / production | Historical literal production checkpoint compared with a prepared or optimized production checkpoint | S23 trilinear, S24 quantize, S28 diffusion |
| Production / production | Different preparation, candidate, or identical-code control; inspect actual revisions | S21/S22 preparation overhead, S25 rejected dispatch, S26 rejected converter reuse, S27 self-pair, S29 rejected converter reuse |
| Public / public | Complete installed-package calls, including their boundary costs | S41 scalar release matrix; its roles use the same package within each attempt |

A `spec:*` oracle registration proves callable verification coverage. It does not prove the worker times that reference.
In `crates/ditherette-bench/src/paired_native.rs:173`, typed native validation accepts production IDs for all non-resize components.
Its timed workload also hardcodes production converters, component execution, and score functions.
Frozen `Conformance` subjects therefore remain untimed references in this worker.
The ordinary resize worker accepts both `spec:*` and `prod:*` `Resize` subjects.

## Resize inventory

Seven algorithms expand to ten filter/support configurations. Bicubic, Lanczos2, and Lanczos3 each support fixed and scale-aware filters.
Frozen paths live in `spec/resize/scalar/`; production paths live in `prod/resize/scalar/`.

| Algorithm | Frozen callable | Selected production callable | Registry suffix after `spec:resize:` / `prod:resize:` |
| --- | --- | --- | --- |
| Nearest | `nearest::resize_nearest_into` | `nearest::resize_nearest_rgba8_into`; prepared execution uses `resize_nearest_rgba8_with_plan_into` | `nearest:scalar` |
| Area | `area::resize_area_into` | `area::resize_area_rgba8_into`; `AreaResizePlan` and `resize_area_rgba8_with_plan_and_scratch_into` | `area:scalar` |
| Bilinear | `bilinear::resize_bilinear_into` | `bilinear::resize_bilinear_rgba8_into`; `BilinearResizePlan` and `resize_bilinear_rgba8_with_plan_and_scratch_into` | `bilinear:scalar` |
| Bicubic | `bicubic::resize_bicubic_into` | `bicubic::resize_bicubic_rgba8_into`; `BicubicResizePlan` and plan/scratch writer | `bicubic:catmull-rom`, `bicubic:catmull-rom-scale-aware` |
| Lanczos2 | `lanczos::resize_lanczos2_into` | `lanczos::resize_lanczos2_rgba8_into`; `LanczosResizePlan::try_new2` and shared plan/scratch writer | `lanczos2:fixed`, `lanczos2:scale-aware` |
| Lanczos3 | `lanczos::resize_lanczos3_into` | `lanczos::resize_lanczos3_rgba8_into`; `LanczosResizePlan::try_new3` and shared plan/scratch writer | `lanczos3:fixed`, `lanczos3:scale-aware` |
| Trilinear | `trilinear::resize_trilinear_into` | `trilinear::resize_trilinear_into`; bounded pipeline uses `PreparedTrilinear` | `trilinear:mip-area` |

`bench_subjects.rs` registers all ten configurations on both sides.
`bench_subjects/trilinear.rs` times fallible preparation and scratch allocation with caller-owned output.
`bench_subjects/resize_budgeted.rs` adds eight `candidate:resize:*:budgeted*` adapters for area through Lanczos3.
They time fallible plan/scratch preparation and execution. Output allocation remains outside timing.
`bench_subjects/resize_calls.rs` adds nine `candidate:resize:*:complete-call` adapters, covering nearest through Lanczos3.
They include processor creation, source snapshot, hashing, preparation, boundary output copy, and cleanup.
Trilinear lacks that legacy complete-call ID, but the typed Processor resize adapter supports its request.
`candidate:resize:nearest:legacy` is another ID for the landed packed kernel, not a distinct current candidate.

Production also exports row writers, plan constructors, capacity queries, and caller-owned scratch variants.
These reuse each row's algorithm. They are not additional filter variants or independently measured kernels.
Frozen generic convolution and radius-parametric Lanczos remain callable helpers. Production shares its convolution plans and execution.
There is no standalone generic convolution timing registration beyond the six named convolution recipes.

| History | Current interpretation |
| --- | --- |
| Nearest literal `0ede7f6c`; measured literal `89b570e0` versus incremental `f9b51e45` | Historical replacement experiment only. Restoration removes that candidate from current production. See [S19](60-nearest-measurement.md). |
| Area/bilinear literal `88b3162900f547837a596b49f0b0199c32b42cda` | Superseded historical copy. Restored landed arithmetic remains selected. |
| Convolution literal `97ca0996d67dc0a20a8058cc35bbe1e05515c8a3` | Superseded historical copy. Restored landed arithmetic remains selected. |
| [Restoration](108-restore-landed.md) `d17e323d` and integration | Preserves optimized packed nearest, area/bilinear, convolution, and shared helpers. No new speedup claim accompanies restoration. |
| [S21](62-measurement.md), [S22](63-measurement.md) | Restored production versus fallible preparation around the same loops. Some frozen comparisons remain incorrect. No new scalar optimization selected. |
| Trilinear literal `fba85a94afca921a9df15286c26ef4bd22281389`; selected shared mip `07d528a6` | [S23](64-measurement.md) selects reuse of exact rounded mip levels after native and public comparisons pass. |

Trilinear's private exact f64 area/bilinear dependency is required by its frozen recipe.
It does not replace the landed general area/bilinear kernels.
Current frozen-exact release coverage must retain known bilinear differences. Equal production roles do not establish frozen equality.

## Color conversion and reconstruction inventory

All seven spaces have forward, f32 inverse, and f64 reconstruction paths.

| Space | Frozen forward under `S color` | Frozen and production f32 inverse leaf | Production packed selector |
| --- | --- | --- | --- |
| sRGB | `srgb::rgb8_to_srgb`, `rgba8_to_srgb32_into` | `srgb::srgb_to_rgb8`, `srgb32_to_rgba8_into` | `PackedSpace::Srgb` |
| Linear RGB | `linear::rgb8_to_linear_rgb`, `rgba8_to_linear_rgb32_into` | `linear::linear_rgb_to_rgb8`, `linear_rgb32_to_rgba8_into` | `PackedSpace::LinearRgb` |
| Oklab | `oklab::rgb8_to_oklab`, `rgba8_to_oklab32_into` | `oklab::oklab_to_rgb8`, `oklab32_to_rgba8_into` | `PackedSpace::Oklab` |
| OKLCH | `oklch::rgb8_to_oklch`, `rgba8_to_oklch32_into` | `oklch::oklch_to_rgb8`, `oklch32_to_rgba8_into` | `PackedSpace::Oklch` |
| CIELAB | `cielab::rgb8_to_cielab`, `rgba8_to_cielab32_into` | `cielab::cielab_to_rgb8`, `cielab32_to_rgba8_into` | `PackedSpace::Cielab` |
| CIELCH | `cielch::rgb8_to_cielch`, `rgba8_to_cielch32_into` | `cielch::cielch_to_rgb8`, `cielch32_to_rgba8_into` | `PackedSpace::Cielch` |
| YCbCr | `ycbcr::rgb8_to_ycbcr`, `rgba8_to_ycbcr32_into` | `ycbcr::ycbcr_to_rgb8`, `ycbcr32_to_rgba8_into` | `PackedSpace::Ycbcr` |

Frozen scalar dispatch is `S color::rgb8_to_coordinates` and `S color::coordinates_to_rgb8`.
Production uses `P color::packed::Converter::{new,coordinates,rgba8_into}` and `P color::inverse::coordinates_to_rgb8`.
`P color::packed::rgb8_to_coordinates` creates a fresh converter for each call.
`S/P color::reconstruct::{coordinates_to_srgb,coordinates_to_rgb8}` accepts f64 coordinates and dispatches all seven spaces.
That wide reconstruction retains different precision from the f32 inverse. Its coverage cannot be inferred from inverse-only timings.

The landed `P color::rgba8_to_color_space_f32*` API also writes legacy four-channel float buffers, including normalized alpha.
Packed conversion writes three channels and canonicalizes cylindrical gray/hue values. Those are distinct output contracts.
Legacy scalar `_into`, policy, and row-band methods remain present. `wasm::convert_color_space` exposes the legacy conversion.
The v1 package exposes the five coarse operations, not seven direct color methods.

| Current registration | What actually runs | Historical evidence and gap |
| --- | --- | --- |
| `spec:color:{space}:f32-roundtrip-v1`, `prod:color:{space}:packed-forward` | All seven conformance outputs; typed `ColorForward` times the prepared production triplet writer | S24 covers five ordinary forwards; S25 covers two cylindrical forwards. Neither establishes current spec/prod timing. |
| `spec/prod:color:inverse:f32-image-v1` | `FieldComponent::Inverse` accepts all seven spaces; source coordinates and output storage prepare before timing | S26 covers seven inverse controls. Typed timing runs production only. |
| `spec/prod:color:source:construction-inclusive-v1` | `FieldComponent::SourceConversion` includes per-pixel converter construction on the production side | S26 samples sRGB and Oklab. All seven settings are callable, but exhaustive seven-space historical timings are absent here. |
| No wide reconstruction subject | f64 reconstruction runs within perturbation and diffusion composition | Add a typed f64 input batch for direct coverage; retain out-of-gamut and cylindrical cases. |
| No legacy f32x4 subject in this typed registry | Legacy materialization retains its separate API | Packed-forward results do not measure legacy f32x4 writing. |

S24 reuses existing landed formulas/tables. S25 adds literal missing metric and cylindrical contract behavior.
S26 copies seven inverse paths and reconstruction, documented by [its literal manifest](67-literal-manifest.json).
No separate inverse or reconstruction optimization is selected in those records.

## Matching, quantization, and palette inventory

All fifteen policies dispatch through `S/P quantize::metric::distance_score`.

| Policy | Coordinate domain | Scoring leaf | Direct score registration |
| --- | --- | --- | --- |
| `SrgbEuclidean` | sRGB | `euclidean3_squared` | `euclidean`, sRGB fixture |
| `LinearRgbEuclidean` | Linear RGB | `euclidean3_squared` | Same formula; no separate domain fixture |
| `OklabEuclidean` | Oklab | `euclidean3_squared` | Same formula; no separate domain fixture |
| `OklchEuclidean` | OKLCH | `euclidean3_squared` | Same formula; no separate domain fixture |
| `CielabEuclidean` | CIELAB | `euclidean3_squared` | Same formula; no separate domain fixture |
| `CielchEuclidean` | CIELCH | `euclidean3_squared` | Same formula; no separate domain fixture |
| `YcbcrEuclidean` | YCbCr | `euclidean3_squared` | Same formula; no separate domain fixture |
| `OklchCircularHue` | OKLCH | `circular_hue3_squared` | `chord`, OKLCH fixture |
| `CielchCircularHue` | CIELCH | `circular_hue3_squared` | Same formula; no CIELCH fixture |
| `OklchHueArc` | OKLCH | `hue_arc3_squared` | `arc`, OKLCH fixture |
| `CielchHueArc` | CIELCH | `hue_arc3_squared` | Same formula; no CIELCH fixture |
| `SrgbCompuphase` | sRGB | `weighted_rgb_squared(..., CompuPhase)` | `compuphase` |
| `SrgbRec601` | sRGB | `weighted_rgb_squared(..., Rec601)` | `rec601` |
| `SrgbRec709` | sRGB | `weighted_rgb_squared(..., Rec709)` | `rec709` |
| `CielabCiede2000` | CIELAB | `ciede2000_distance` → `color::lab_ciede2000::ciede2000` | `ciede2000` |

`bench_subjects/scores.rs` registers `spec/prod:metric:{family}:cyclic-scores-v1` for seven formula families.
Pairs use frozen conversion before timing. Production leaf selection also occurs before timing.
This measures formula batches, not fifteen policy dispatch paths or full palette matching.
All fifteen policies are expressible through complete quantize, diffusion, Yliluoma, and process settings.

| Layer | Frozen callable | Selected production callable | Native timing coverage |
| --- | --- | --- | --- |
| Palette preparation | `palette::PreparedPalette::new` | `palette::PreparedPalette::try_new`; capacity query; `prepare_pixel`; `into_indexed` | Included in complete quantize/dither; no standalone preparation subject |
| Nearest scan | `quantize::matcher::PaletteMatcher::{new,nearest}` | `PaletteMatcher::prepare` internally; public `nearest` retains ordered scan and per-entry `distance_score` | Included in complete quantize/dither; no prepared nearest batch subject |
| Quantize | `quantize::quantize` | `quantize::quantize(request, memory_limit)` | Current selected ID remains `candidate:quantize:request:prepared` |
| Prepared writing | Frozen `quantize/nearest_color.rs` generic image and nearest helpers | `PreparedQuantizer::{try_new,quantize_into,quantize_rows_into,quantize_bands_into,into_indexed}` | No isolated prepared writer ID; Processor cache/stage cases include surrounding work |

The frozen generic helpers cover Euclidean, circular, weighted RGB, and CIEDE2000 image/nearest operations.
They have no one-to-one production generic API. Production routes those semantics through the prepared matcher and typed request.
Palette tests cover order, duplicates, first ties, transparency, fallback, truncation warnings, matte, and premultiplied behavior.
Standalone metric timing does not cover those palette contracts.

Literal palette/quantize checkpoint is `a23260edec0452fd17c13073636f548b07804230`.
[S24](65-measurement.md) compares a literal native artifact against bounded preparation and records 13 passing native cases.
Public roles are same-package controls. This does not establish a new scalar speedup.
[S25](66-measurement.md) retains baseline `0085972a05a3dbdbbef6d47351d6e37bdd8625d2`.
The dispatch candidate `230046ff` is rejected, including a required Euclidean-score regression.
Its individual faster metric cases are not selected production gains.

## Fields, adaptive placement, diffusion, and Yliluoma

| Family or variant | Frozen and production callable | Registration and measured scope |
| --- | --- | --- |
| Bayer2 | `dither::ordered::{bayer_value,bayer_noise_at}`, size Two | `spec/prod:field:thresholds:global-v1`, `Field::Bayer`; S26 grid and composed cases |
| Bayer4 | Same functions, size Four | Same typed subject; S26 |
| Bayer8 | Same functions, size Eight | Same typed subject; S26 |
| Bayer16 | Same functions, size Sixteen | Same typed subject; S26 |
| Random | `dither::random_noise::{random_u32_at,random_noise_at}` | Same typed subject, seed and absolute pixel index; S26 |
| Blue noise | `dither::blue_noise::blue_noise_at`, retained `BLUE_NOISE_32X32` | Same typed subject, global x/y modulo 32; S27 |
| Adaptive placement | `dither::placement::{coordinate_domain,placement_distance,contrast_at,placement_mask_at}` | `spec/prod:placement:adaptive:mask-v1`; all spaces/settings callable; S26 directly samples Oklab/radius1 and OKLCH/radius2 |

The shared production field loop is `P dither::perturb::perturb_by_field_rows_into` and its band variants.
It applies source conversion, field scaling, adaptive mask, wide reconstruction, and preserved alpha.
Frozen `S dither::perturb::{perturb,perturb_into,perturb_rows_into,perturb_by_field_rows_into,quantize_after_perturb}` owns the corresponding recipe.
Frozen Bayer/random/blue-noise generic `dither_*_into` and `*_by_nearest_into` helpers also remain callable.
Production exposes the shared field recipe instead of duplicating those generic coordinate-buffer APIs.
Random `Mulberry32` and blue-noise generation are frozen support/tooling, not additional runtime field variants.

S26 literal checkpoint is `e156cfbfe0d3dfb598e2f5746bca1ddc46a5f69a`.
[S26](67-measurement.md) retains `3915f605` and leaves converter-reuse candidate `b237b746` unselected due to browser noise gates.
Five historical threshold controls needed a verifier repair. Their original `incorrect` records remain historical.
[S27](68-benchmark-results.md) compares independent builds of the same literal blue-noise implementation `44cbe435`.
Its five native cases pass. It establishes baseline repeatability, not an optimized lookup.

| Diffusion kernel | Feedback variants, both callable and represented by S28 native recipes |
| --- | --- |
| Floyd-Steinberg | sRGB bytes; matching space |
| Sierra | sRGB bytes; matching space |
| Sierra Lite | sRGB bytes; matching space |
| Atkinson | sRGB bytes; matching space |

Frozen complete callable is `S dither::error_diffusion::diffuse`.
Production retains literal `P dither::error_diffusion::diffuse` and generic `dither_error_diffusion_*` helpers.
The public pipeline selects `P dither::error_diffusion::prepared::{diffuse,PreparedDiffusion}` with three work rows.
Current benchmark IDs bind literal `prod:dither-and-quantize:diffusion:full-image-v1` and selected `candidate:dither-and-quantize:diffusion:three-row-v1`.
Both include native preparation/output allocation and destruction while borrowing source. Neither includes the public source copy.
Literal checkpoint is `91cd9320`; adapter checkpoint is `c47419e4`; selected candidate is `90909b93`.
[S28](69-measurement.md) selects the three-row implementation after eight passing native literal/production comparisons.
The native report does not isolate ring storage from converter reuse. Public measurements compare the ring package with itself.
Two WebKit public controls remain inconclusive. The eight native recipes are not every metric/alpha/scan Cartesian combination.

Yliluoma uses `S/P dither::yiluoma::{adaptive_target,best_matched_mix,ordered_mix_index}`.
Frozen also exports generic `best_ordered_mix`, `best_ordered_mix_by_distance`, and image/by-distance writers.
Production complete execution is `P dither::yiluoma::{dither_yiluoma,dither_yiluoma_into}` plus row-band execution.
It preserves exhaustive palette-pair/ratio order, strict ties, Bayer2/4/8/16 selection, and all fifteen matching policies.
`prod:dither-and-quantize:yliluoma:literal-v1` calls the actual bounded native request.
There is no independent mix-search or target-adaptation subject.
Literal math is `9718b168`; literal public checkpoint is `4e0c134d`.
[S29](70-benchmark-results.md) retains the literal baseline. Converter-reuse candidate `abe8241` remains unselected.
The native and three browser overall timing gates are inconclusive despite exact measured outputs.
Target-local frozen Wasm verification is necessary for recorded CIELAB/CIEDE2000 native/Wasm differences.

## Composed exports

| Public method | Frozen callable | Selected native entry | Existing native subjects |
| --- | --- | --- | --- |
| `resize` | `S resize::resize` | `P pipeline::processor::Processor::resize` | `prod:resize:request:processor-v1`, plus resize subjects above |
| `quantize` | `S quantize::quantize` | `Processor::quantize` | `prod:quantize:request:processor-v1`, plus direct prepared quantize ID |
| `perturb` | `S pipeline::perturb` | `Processor::perturb` | `prod:perturb:request:processor-v1` |
| `ditherAndQuantize` | `S pipeline::dither_and_quantize` | `Processor::dither_and_quantize` | `prod:dither-and-quantize:request:processor-v1` for separable; direct diffusion/Yliluoma IDs above |
| `process` | `S pipeline::process` | `Processor::process` → `P pipeline::process` | `prod:process:request:processor-v1`, `prod:process:request:staged-v1` |

`S pipeline::execute` and frozen `Processor::execute` provide generic/lifecycle dispatch over those five operations.
Private ABI modules are `wasm/{processor,quantize,fields,process}.rs`.
The public wrapper is `packages/ditherette/src/scalar.ts`, with the contract in that package's types.
Legacy `wasm::resize_rgba8`, `process_rgba8`, and `convert_color_space` remain separate coarse/materialization exports.
They are not additional v1 package methods or evidence of direct kernel timing.
Process preserves resize RGBA8 rounding, then the selected dither/quantize recipe.
Separable composition preserves its rounded/clipped RGBA8 boundary before palette preparation and matching.

`bench_subjects/reference.rs` registers `spec:{resize,quantize,perturb,dither-and-quantize,process}:request:v1`.
Process/staged subjects compare production composition strategies. Processor subjects exercise explicit cache capabilities and preparation state.
They do not isolate the arithmetic kernel from hashing, import, preparation, allocation, cache lookup, or copying.
S30-S33 reports provide composition/preparation/stage/progress evidence; S41 provides public release-cell evidence.

| Composed-call history | Comparison and retained conclusion |
| --- | --- |
| [S30](71-benchmark-results.md) | Same production revision, staged resize+dither versus Process. All four overall gates remain incorrect from inherited area differences. |
| [S31](72-benchmark-results.md) | Uncached S30 versus required preparation reuse. Warm Lanczos3 meets the bounded target; native and WebKit overall gates remain inconclusive. |
| [S32](73-benchmark-results.md) | Preparation-only S31 versus required image-stage reuse. All four runtime gates report regression; seven confirmed cold regressions remain. |
| [S33](74-benchmark-results.md) | Public progress support and callback overhead. There is no native timing in this slice; callback-disabled Chromium has two inconclusive controls. |

At historical report commit `ffc6ea82`, `.plans/83-scalar-completion.md` records 45 complete scalar browser cells.
Their gates are 40 pass, three incorrect bilinear cells, and two inconclusive warm controls.
Those cells span two package artifacts and compare the same package within each attempt.
They cover release recipes, not every kernel export or fresh native spec/production timing.
This audit does not change their gates or recompute their ratios.

## Most valuable next scalar work

These priorities follow inspected source and retained evidence. They are not new performance measurements.

1. Add genuine frozen timed dispatch for existing typed native components and complete calls.
   Preserve separate prepared-forward, construction-inclusive-forward, inverse, metric, and complete-call scopes.
   Add wide f64 reconstruction batches and direct palette/prepared-nearest coverage where attribution requires them.
   Cover all fifteen policy domains explicitly while retaining the seven shared formula families.
   Completion means every requested family has a named callable, timing scope, and honest spec/production or literal/production comparison.
2. Revisit call-owned conversion in field placement and perturbation, using the retained S26 candidate as a starting point.
   `packed::rgb8_to_coordinates` rebuilds two 256-entry tables per read, including the linear transfer table.
   `placement::contrast_at` invokes source conversion for the center and eight neighbors per adaptive pixel.
   The current field writer separately converts its source pixel. The source explains why this deserves first attention.
   Retain exact arithmetic, original source bytes for placement, global coordinates, and bounded memory accounting.
   Previous candidate gains justify investigation, not automatic promotion.
3. Revisit Yliluoma converter reuse before changing exhaustive search.
   The current request owns `PreparedQuantizer`, yet target reads still call the construction-inclusive helper.
   Reuse existing preparation with exact output checks and a new declared comparison against selected production.
   If pair/ratio work remains dominant, evaluate bounded preparation separately and preserve enumeration/tie order.
4. Attribute matching cost before another dispatch rewrite.
   Direct scans remain proportional to pixels times visible palette entries; CIEDE2000 also repeats expensive distance arithmetic.
   S25 rejects the previous dispatch candidate. Its failure warrants focused prepared-nearest and score controls before another choice.
5. Keep resize optimization separate from correctness and boundary cost.
   Preserve restored kernels, shared helpers, and selected exact shared mips.
   Existing bilinear drift requires an explicit correctness decision; a public self-pair cannot clear it.
   Native arithmetic timing and complete-call timing can then distinguish kernel cost from wrapper/cache/copy cost.

## Audit completion

The report accounts for seven resize algorithms, seven forward/inverse/reconstruction spaces, fifteen metrics, and palette/quantize layers.
It also accounts for six fields, adaptive placement, eight diffusion kernel/feedback combinations, Yliluoma, and five composed methods.
Extra generic, row-band, prepared, legacy, and tooling exports are classified where they differ from those runtime families.
Only this report changes. No runtime, frozen source, package, benchmark registration, timing evidence, or release ledger changes.
No owned jobs remain after the read-only inspection commands finish.
