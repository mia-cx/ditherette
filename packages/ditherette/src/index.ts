import { DitheretteError } from './errors.js';
import { createScalar } from './scalar.js';
import { validateOptions } from './validation.js';
import type { Ditherette, InitOptions } from './types.js';

export { DitheretteError } from './errors.js';
export type { ErrorCode } from './errors.js';
export type {
	AlphaPolicy,
	IndexedImage,
	Matching,
	PaletteEntry,
	QuantizeRequest,
	WorkingSpace,
	Field,
	Placement,
	PerturbPolicy,
	PerturbRequest,
	DitherAndQuantizeRequest,
	RecipeV1,
	ProcessRequest,
	Ditherette,
	InitInput,
	InitOptions,
	Progress,
	ResizeAnchor,
	ResizeRequest,
	Rgba8Image
} from './types.js';

/** Initialize one isolated browser processor. Importing the package itself loads no Wasm or workers. */
export async function createDitherette(options?: InitOptions): Promise<Ditherette> {
	const normalized = validateOptions(options);
	if (typeof WebAssembly === 'undefined' || typeof WebAssembly.instantiate !== 'function') {
		throw new DitheretteError('capability', 'wasm', 'WebAssembly is unavailable.');
	}
	if (normalized.threads !== 'disabled') {
		const capable = globalThis.crossOriginIsolated === true &&
			typeof Worker === 'function' && typeof SharedArrayBuffer === 'function' &&
			typeof Atomics === 'object';
		if (capable) {
			try {
				const { createThreaded } = await import('./threads.js');
				return await createThreaded(normalized);
			} catch (error) {
				if (error instanceof DitheretteError && error.code === 'memory-limit') throw error;
				if (normalized.threads === 'required') {
					throw new DitheretteError('initialization', 'threads', 'Required threaded initialization is unavailable.');
				}
			}
		} else if (normalized.threads === 'required') {
			throw new DitheretteError('capability', 'threads', 'Required threaded initialization is unavailable.');
		}
	}
	return createScalar(normalized);
}
