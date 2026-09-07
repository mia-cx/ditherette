# ditherette

An MIT-licensed browser ESM image processor. This private checkpoint supports scalar nearest, area, and bilinear resize.

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

Nearest and bilinear support all nine anchors from `top-left` through `bottom-right`, including `center`.
Area uses `{ algorithm: 'area' }` without an anchor. Bilinear uses `{ algorithm: 'bilinear', anchor: 'center' }`.
Neither area nor bilinear accepts a support setting. Bilinear widens its triangle filter during minification.
Area and bilinear preserve the landed f32 accumulation paths and their bounded differences from the f64 reference.
Requests require version `1`, positive integer dimensions, and canonical object/string tags. Unknown fields are rejected.
Source sides are at most 32,768 pixels; output sides are at most 16,384. Both images allow at most 67,108,864 pixels.

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

Cubic, Lanczos, trilinear, and the other processing methods arrive in later implementation slices.
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
