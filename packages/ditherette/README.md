# ditherette

An MIT-licensed browser ESM image processor. This private checkpoint supports scalar resize, palette quantization, and Bayer/random/blue-noise perturbation.

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

Imports perform no initialization, network requests, or worker creation. Each `createDitherette()` loads a fresh scalar instance.
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
Plans and scratch count toward the memory limit. Each call releases this transient storage; there is no package cache.
Requests require version `1`, positive integer dimensions, and canonical object/string tags. Unknown fields are rejected.
Source sides are at most 32,768 pixels; resize output sides are at most 16,384. Both images allow at most 67,108,864 pixels.

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
Quantize does not resize, dither, retain the source, or cache prepared palettes in this checkpoint.

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
Input, output, and the separable RGBA8 intermediate count toward the capacity limit and are reserved before input copy.
Results remain durable after later calls and disposal. No field buffers or prepared palettes are cached.
Diffusion and Yliluoma are not enabled in this checkpoint.

## Initialization and ownership

`createDitherette({ memoryLimitBytes, threads, wasm })` accepts optional initialization settings.

- `memoryLimitBytes` defaults to 1.5 GiB and accepts integers from 1 byte through 2 GiB. Insufficient capacity fails with `memory-limit`.
- `threads` defaults to `disabled`. At this checkpoint, `preferred` uses scalar and `required` fails with `capability`.
- `wasm` accepts bytes, an offset byte view, a URL/string, a Request, a Response, or a compiled `WebAssembly.Module`.
  Caller Response/Request bodies are cloned before initialization. Default assets resolve relative to the package.

The memory limit counts private Wasm capacity and boundary copies. Caller-owned and returned JS buffers and fixed module overhead are excluded.
Unexpected allocation/copy failures report `wasm-memory-unavailable`. Expected errors leave the processor usable.
An uncaught Wasm trap retires that processor without affecting other instances.

`dispose()` releases instance ownership and is idempotent. Processing afterward fails with `disposed` at `instance`.
Wasm pages can remain at their high-water mark until the discarded module is collected.
Recursive processing or disposal fails with `reentrant-call`, including calls from request property getters.

## Checkpoint scope

Trilinear and the remaining processing methods arrive in later implementation slices.
Supplying `onProgress` currently fails explicitly with `unsupported-operation`; S33 adds progress delivery.
S34 adds the optional threaded runtime. These are temporary slice limits, not permanent API restrictions.
The package exports no raw bindings, backend selection, cache controls, or processor counters.
The `0.x` public target is browser ESM and browser bundlers. Node-based tests are development fixtures, not public Node support.

## Workspace builds

Run `pnpm package:build` at the repository root to compile both Wasm variants and the wrapper. Generated artifacts stay under this package's ignored `dist/` directory. The crate owns compilation; this package stages the scalar, threaded, and worker files for distribution.

After building, `pnpm --filter ditherette test:interface` checks the public types and boundary behavior.
`pnpm --filter ditherette test:browser` packs and installs a temporary tarball, then tests Chromium, Firefox, and WebKit.
Browser fixtures require Playwright's pinned browser binaries and their platform libraries. They do not launch the website preview.

The website remains at the repository root as private workspace `ditherette-web`. Its existing Wasm URLs remain available through the root `wasm:build` commands until package adoption.
