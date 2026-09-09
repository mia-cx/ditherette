# ditherette

An MIT-licensed browser ESM image processor with scalar execution and optional threaded acceleration.
The `0.x` version signals beta status; releases use ordinary versions such as `0.1.0`, without a prerelease suffix.
For browser applications and bundlers, depend on `ditherette` at `^0.1.0`. Node and CommonJS are not supported consumer targets.
Publication remains held while the recorded release gates await acceptance.

```ts
import { createDitherette, DitheretteError } from 'ditherette';

const processor = await createDitherette();
try {
	const image = processor.resize({
		version: 1,
		source: { width: 1, height: 1, data: new Uint8Array([80, 120, 160, 255]) },
		output: {
			width: 32,
			height: 32,
			resize: { algorithm: 'nearest', anchor: 'center' }
		}
	});
	// image.data is a durable, JS-owned Uint8Array.
} catch (error) {
	if (error instanceof DitheretteError) console.error(error.code, error.path, error.message);
	else throw error;
} finally {
	processor.dispose();
}
```

Imports perform no initialization, network requests, or worker creation. Each `createDitherette()` loads a fresh isolated instance.
Calls are synchronous. Hosts can run them in their own worker to keep the main thread responsive.

Inputs are already-cropped, packed RGBA8 `Uint8Array` views. Offset views work; detached or incorrectly sized views fail.
For `ImageData`, create a `Uint8Array` view over its `Uint8ClampedArray` buffer without copying its bytes.
Calls preserve inputs. Returned arrays remain valid after later calls, memory growth, and disposal.

Every filter except area supports all nine anchors from `top-left` through `bottom-right`, including `center`.
Area uses `{ algorithm: 'area' }` without an anchor. Bilinear uses `{ algorithm: 'bilinear', anchor: 'center' }`.
Neither area nor bilinear accepts a support setting. Bilinear widens its triangle filter during minification.
Area and bilinear preserve the landed f32 accumulation paths and their bounded differences from the f64 reference.
Bicubic, Lanczos2, and Lanczos3 require explicit support, for example `{ algorithm: 'lanczos3', anchor: 'center', support: 'scale-aware' }`.
`fixed` keeps the kernel radius constant. `scale-aware` widens support during minification.
These filters retain the landed f64 kernels, including separable large-image downscales and their bounded reference differences.
Trilinear uses `{ algorithm: 'trilinear', anchor: 'center' }` without a support setting.
It builds area mip levels, samples them with bilinear filtering, and blends the adjacent levels selected by minification.
Mip dimensions round upward when halved. Every intermediate retains RGBA8 rounding, including hidden RGB and alpha.
The shared mip chain and temporary outputs reserve capacity before execution.
Plans, scratch, and retained stages count toward the memory limit.
Requests require version `1`, positive integer dimensions, and canonical object/string tags. Unknown fields are rejected.
Source sides are at most 32,768 pixels; resize output sides are at most 16,384. Both images allow at most 67,108,864 pixels.

## Complete processing

```ts
const indexed = processor.process({
	source,
	palette,
	recipe: {
		version: 1,
		output: { width: 320, height: 240, resize: { algorithm: 'trilinear', anchor: 'center' } },
		alpha: { mode: 'preserve', threshold: 128 },
		match: 'oklab-euclidean',
		dither: { family: 'none' }
	}
});
```

The recipe uses `match`; staged quantization methods use `matching`.
Every resize and dither family works in this composition. Process equals actual
`resize` followed by `ditherAndQuantize`, including palette metadata and warnings.
Resized and perturbed RGBA8 intermediates stay in Wasm with their rounding intact.
Only the final indexed result crosses back to JS. The complete call snapshots
input and preflights plans, prepared palettes, scratch, intermediates, and indices before processing.
Successful calls may retain prepared data and deterministic stages for reuse.

Recipe settings errors use paths such as `recipe.match` and `recipe.dither.size`.
Input, palette, memory, and result-copy errors keep their existing paths.
The landed resize kernels retain their documented frozen-reference differences;
process does not claim to remove them.

## Direct quantization

```ts
const indexed = processor.quantize({
	version: 1,
	source: { width: 1, height: 1, data: new Uint8Array([255, 0, 0, 128]) },
	palette: [{ kind: 'color', rgb: [255, 0, 0] }, { kind: 'transparent' }],
	alpha: { mode: 'preserve', threshold: 127.9999999 },
	matching: 'srgb-euclidean'
});
// indexed.indices[0] is 0. indexed.palette.rgba stores ordered RGBA bytes.
// indexed.palette.transparentIndex is 1, or null when no transparent entry exists.
```

Matching supports all fifteen valid color/metric pairs:

| Space      | Matching tags                                                     |
| ---------- | ----------------------------------------------------------------- |
| sRGB       | `srgb-euclidean`, `srgb-compuphase`, `srgb-rec601`, `srgb-rec709` |
| Linear RGB | `linear-rgb-euclidean`                                            |
| Oklab      | `oklab-euclidean`                                                 |
| OKLCH      | `oklch-euclidean`, `oklch-circular-hue`, `oklch-hue-arc`          |
| CIELAB     | `cielab-euclidean`, `cielab-ciede2000`                            |
| CIELCH     | `cielch-euclidean`, `cielch-circular-hue`, `cielch-hue-arc`       |
| YCbCr      | `ycbcr-euclidean`                                                 |

Cylindrical Euclidean compares the three coordinates directly, including hue radians.
`circular-hue` uses the geometric-mean chroma chord; `hue-arc` uses the shorter arc scaled by the smaller chroma.
These are distinct recipes. Exact byte grays have zero cylindrical chroma and hue.
CompuPhase and Rec.601/709 weight gamma-encoded sRGB. CIEDE2000 uses D65 CIELAB with unit weighting factors.
Palette order and duplicates remain intact; first-index ties win. More than 256 entries produces a truncation warning.
Every supplied entry is validated, including entries beyond that retained prefix.
`preserve` thresholds byte alpha using the exact supplied JavaScript number.
Thresholded pixels use the first transparent entry, or the darkest visible entry with a fallback warning.
`{ mode: 'premultiplied' }` rounds alpha-scaled RGB bytes; `{ mode: 'matte', rgb: [r, g, b] }` composites onto that RGB matte.
Transparent-only palettes produce transparent indices with the approved warning.
The result contains durable `indices`, `palette.rgba`, `palette.transparentIndex`, and `{ code, message }` warnings.
Quantize does not resize, dither, or retain the original source buffer. Prepared palettes may be reused.

## Palette-free fields

```ts
const perturb = {
	field: { algorithm: 'bayer', size: '4' },
	space: 'oklch',
	strength: 0.7,
	placement: { mode: 'adaptive', radius: 1, threshold: 5, softness: 10 }
} as const;
const rgba = processor.perturb({ version: 1, source, perturb });
const indexed = processor.ditherAndQuantize({
	version: 1,
	source,
	palette,
	alpha,
	matching,
	dither: { family: 'separable', perturb }
});
```

Bayer sizes are string tags `2`, `4`, `8`, and `16`.
Random uses `{ algorithm: 'random', seed: 0 }`, with an unsigned 32-bit integer seed.
Blue noise uses `{ algorithm: 'blue-noise' }`, with a fixed 32×32 tile and no size or seed controls.
Random values depend on the global pixel index, so row scheduling does not change the sequence.
Working spaces are `srgb`, `linear-rgb`, `oklab`, `oklch`, `cielab`, `cielch`, and `ycbcr`.
They are independent of palette matching settings.

`{ mode: 'everywhere' }` has no adaptive controls.
Adaptive placement uses the original image's eight-neighbor contrast and fixed color-space ranges, without reading the palette.
Cylindrical placement uses the minimum-chroma hue arc, not matching's circular chord.
Radius is an integer from 1 through 32,768. Strength, threshold, and softness accept finite nonnegative f32-range numbers.
The implementation rounds these controls to f32, but reconstructs perturbed coordinates with wide arithmetic before final RGB clipping and byte rounding.
Zero strength preserves every source byte. Both placement modes preserve alpha and hidden RGB processing.

`ditherAndQuantize` quantizes that completed RGBA8 result, with the same indices, palette, and warnings as `quantize(perturb(...))`.
`{ family: 'none' }` performs direct quantization and accepts no perturb settings.
Input, output, and the separable RGBA8 intermediate count toward the capacity limit and are preflighted before processing.
Results remain durable after later calls and disposal. Successful calls may cache deterministic field outputs and prepared palettes.
## Error diffusion

```ts
const indexed = processor.ditherAndQuantize({
	version: 1, source, palette, alpha, matching,
	dither: {
		family: 'diffusion', kernel: 'floyd-steinberg', feedback: 'srgb-bytes',
		strength: 1, serpentine: true, placement: { mode: 'everywhere' }
	}
});
```

Kernels are `floyd-steinberg`, `sierra`, `sierra-lite`, and `atkinson`.
`srgb-bytes` feedback rounds and clips before matching. `matching` feedback keeps unrounded coordinates in the selected matching space.
Serpentine scanning reverses alternate rows. Adaptive placement uses the unchanged source and the matching space.
Preserved transparent pixels discard incoming error and emit none. Palette order, duplicates, and warnings follow direct quantization.
Diffusion stays scalar and reserves three work rows. Scratch capacity scales with width; owned source and index buffers also count toward the limit.
An arithmetic overflow returns `runtime` at `dither.arithmetic`, publishes no result, and leaves the instance usable.

## Yliluoma

Yliluoma uses the same palette, matching, and alpha controls:

```ts
const indexed = processor.ditherAndQuantize({
	version: 1,
	source,
	palette,
	matching: 'srgb-euclidean',
	alpha: { mode: 'preserve', threshold: 128 },
	dither: { family: 'yliluoma', size: '4', placement: { mode: 'everywhere' } }
});
```

Matrix sizes are `'2'`, `'4'`, `'8'`, and `'16'`. Adaptive placement uses the controls shown above.
Yliluoma searches every ordered palette pair and matrix ratio. A zero adaptive mask still searches mixtures of the nearest color.
It allocates source and index storage without an RGBA8 intermediate or mixture table.
Exact outputs follow the frozen Wasm reference; native floating-point math can select different mixtures near ties.

## Initialization and ownership

`createDitherette({ memoryLimitBytes, threads, wasm })` accepts optional initialization settings.

- `memoryLimitBytes` defaults to 1.5 GiB and accepts integers from 1 byte through 2 GiB. Insufficient capacity fails with `memory-limit`.
- `threads` defaults to `disabled`, which loads only scalar assets. `preferred` tries threads when capable and falls back after failed initialization cleanup.
  `required` reports `capability` at `threads` when shared memory, workers, or blocking waits are unavailable, or `initialization` when startup fails.
- `wasm` accepts bytes, an offset byte view, a URL/string, a Request, a Response, or a compiled `WebAssembly.Module`.
  Caller Response/Request bodies are cloned before initialization. Default assets resolve relative to the package.
  Custom inputs must match the selected variant. Preferred fallback retries the supplied input with scalar bindings; incompatible bytes remain an initialization error.

Threads require cross-origin isolation, shared Wasm memory, module workers, and a caller context that permits blocking waits.
Synchronous threaded methods run in a processing worker. Browser main JS supports scalar execution;
`preferred` selects scalar there and `required` returns a capability error before loading threaded assets.
Hosts must allow the package's worker script and `blob:` bootstrap.
Each processor owns independent module memory and its pool. Workers within that pool share only that processor's memory.
The existing policy uses `clamp(logical CPUs / 2, 1, 8)` pool workers. Pool size is not a public option.
Initialization creates the pool. Measured resize, quantize, separable-field, and Yliluoma workload classes may use row bands.
Small or unmeasured classes and error diffusion remain scalar. Scheduling never changes recipe identities or exact results.

The memory limit counts private Wasm capacity and boundary copies. Caller-owned and returned JS buffers and fixed module overhead are excluded.
The shared per-instance cache retains at most `min(256 MiB, memoryLimitBytes / 4)` and 128 entries.
Pressure drops idle scratch before least-recently-used entries. Only successful calls publish new entries.
Cache keys include input content and normalized settings, so changing an existing input view does not reuse stale pixels.
Unexpected allocation/copy failures report `wasm-memory-unavailable`. Expected errors leave the processor usable.
An uncaught Wasm trap retires that processor without affecting other instances.

`dispose()` releases instance ownership and terminates its pool once. Processing afterward fails with `disposed` at `instance`.
Wasm pages can remain at their high-water mark until the discarded module is collected.
Recursive processing or disposal fails with `reentrant-call`, including calls from request property getters.

Threaded release remains blocked on the retained WebKit 26.4 engine. Its parked Wasm workers survive termination in lifecycle tests.
The [upstream WebKit fix](https://github.com/WebKit/WebKit/commit/03e836de2f7bd5627a95f60357633d59fb6bb18d) addresses this engine failure.
The failing disposal and host-termination checks remain active. Scalar processing does not require a pool.

## Public contract

All five synchronous processing methods, private caches, progress delivery, and optional pool initialization are available.
Progress completion follows durable output construction. A thrown callback fails the call without publishing new cache entries.
Progress uses typed stages and measurable counts, with within-stage callbacks throttled to 50 ms.
The package exports no raw bindings, backend selection, cache controls, or processor counters.
The `0.x` public target is browser ESM and browser bundlers. Node-based tests are development fixtures, not public Node support.

## Workspace builds

Run `pnpm package:build` at the repository root to compile both Wasm variants and the wrapper. Generated artifacts stay under this package's ignored `dist/` directory. The crate owns compilation; this package stages the scalar, threaded, and worker files for distribution.

After building, `pnpm --filter ditherette test:interface` checks the public types and boundary behavior.
`pnpm --filter ditherette test:browser` packs and installs a temporary tarball, then tests Chromium, Firefox, and WebKit.
Browser fixtures require Playwright's pinned browser binaries and their platform libraries. They do not launch the website preview.

The website remains at the repository root as private workspace `ditherette-web`. Its existing Wasm URLs remain available through the root `wasm:build` commands until package adoption.
