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
	if (normalized.threads === 'required') {
		throw new DitheretteError(
			'capability',
			'threads',
			'The threaded runtime is not implemented in this package checkpoint.'
		);
	}
	if (typeof WebAssembly === 'undefined' || typeof WebAssembly.instantiate !== 'function') {
		throw new DitheretteError('capability', 'wasm', 'WebAssembly is unavailable.');
	}
	return createScalar(normalized);
}
