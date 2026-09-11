import { DitheretteError } from './errors.js';
import type { ErrorCode } from './errors.js';
import type {
	Ditherette,
	InitInput,
	ResizeRequest,
	Rgba8Image,
	QuantizeRequest,
	IndexedImage,
	PerturbRequest,
	DitherAndQuantizeRequest,
	ProcessRequest,
	Progress
} from './types.js';
import { validateResize, validateQuantize } from './validation.js';
import { validatePerturb, validateDitherAndQuantize } from './validation-fields.js';
import { processErrorPath, validateProcess } from './validation-process.js';

export type Bindings = Pick<ReturnType<
	typeof import('./wasm/scalar/ditherette_wasm.factory.js').createScalarBindings
>, 'privateInitialize' | 'privateDispose' | 'privateErrorPath' | 'privateProcess' |
	'privateResize' | 'privateQuantize' | 'privatePerturb' | 'privateDitherAndQuantize'>;

type ResultSink<T> = { value?: T; onProgress?: (progress: Progress) => void };

// Private numeric ABI. Keep these aligned with the crate's allocation-free error table.
const errorCodes: readonly ErrorCode[] = [
	'invalid-request',
	'invalid-image',
	'invalid-palette',
	'invalid-settings',
	'unsupported-operation',
	'capability',
	'initialization',
	'memory-limit',
	'wasm-memory-unavailable',
	'disposed',
	'reentrant-call',
	'callback',
	'runtime'
];
const errorPaths = [
	'instance',
	'memoryLimitBytes',
	'source.width',
	'source.height',
	'source.data',
	'source',
	'output.width',
	'output.height',
	'output',
	'output.resize.anchor',
	'wasm',
	'control',
	'output.resize',
	'palette',
	'alpha',
	'alpha.threshold',
	'matching',
	'perturb',
	'perturb.field',
	'perturb.space',
	'perturb.strength',
	'perturb.placement',
	'perturb.placement.radius',
	'perturb.placement.threshold',
	'perturb.placement.softness',
	'dither',
	'dither.strength',
	'dither.placement',
	'dither.placement.radius',
	'dither.placement.threshold',
	'dither.placement.softness',
	'dither.kernel',
	'dither.feedback',
	'dither.serpentine',
	'dither.arithmetic',
	'dither.arithmetic',
	'dither.size',
	'recipe.version',
	'onProgress'
];
const errorMessages: Record<ErrorCode, string> = {
	'invalid-request': 'Invalid processing request.',
	'invalid-image': 'Invalid RGBA8 image.',
	'invalid-palette': 'Invalid palette.',
	'invalid-settings': 'Invalid processing settings.',
	'unsupported-operation': 'This operation is not implemented in this package checkpoint.',
	capability: 'The required capability is unavailable.',
	initialization: 'Scalar initialization failed.',
	'memory-limit': 'Planned allocation capacity exceeds the instance memory limit.',
	'wasm-memory-unavailable': 'The browser could not allocate or copy processing memory.',
	disposed: 'Processor is disposed.',
	'reentrant-call': 'Processing and disposal cannot reenter an active call.',
	callback: 'Progress callback threw.',
	runtime: 'Processing failed unexpectedly.'
};

function failure(
	bindings: Bindings,
	status: number,
	fused: boolean | 'process' = false
): DitheretteError {
	const code = errorCodes[status - 1] ?? 'runtime';
	const pathTag = bindings.privateErrorPath();
	const path = errorPaths[pathTag] ?? 'wasm';
	const message =
		pathTag === 34
			? 'Diffusion work exceeded finite f32 range.'
			: pathTag === 35
				? 'Diffusion matching produced a non-finite distance.'
				: errorMessages[code];
	return new DitheretteError(
		code,
		fused === 'process'
			? processErrorPath(path, code)
			: fused && path.startsWith('perturb')
				? `dither.${path}`
				: path,
		message
	);
}

/** Load only the scalar factory. Caller-supplied response bodies remain reusable. */
export async function createScalar(options: {
	wasm?: InitInput;
	memoryLimitBytes: number;
}): Promise<Ditherette> {
	try {
		const { createScalarBindings } = await import('./wasm/scalar/ditherette_wasm.factory.js');
		const bindings = createScalarBindings();
		await bindings.default({ module_or_path: normalizeWasmInput(options.wasm) });
		return initializeProcessor(bindings, options.memoryLimitBytes);
	} catch (error) {
		if (error instanceof DitheretteError) throw error;
		const code = error instanceof RangeError ? 'wasm-memory-unavailable' : 'initialization';
		throw new DitheretteError(code, 'wasm', errorMessages[code]);
	}
}

/** Clone consumable bodies and preserve views without copying caller bytes. */
export function normalizeWasmInput(wasm: InitInput | undefined): InitInput | undefined {
	if (typeof Response !== 'undefined' && wasm instanceof Response) return wasm.clone();
	if (typeof Request !== 'undefined' && wasm instanceof Request) return wasm.clone();
	// Chromium rejects DataView at its Wasm boundary.
	if (ArrayBuffer.isView(wasm)) return new Uint8Array(wasm.buffer, wasm.byteOffset, wasm.byteLength);
	return wasm;
}

/** Share the existing processing boundary while keeping artifact resources instance-owned. */
export function initializeProcessor(bindings: Bindings, memoryLimitBytes: number, release?: () => void): Ditherette {
	let status: number;
	try {
		status = bindings.privateInitialize(memoryLimitBytes);
	} catch {
		throw new DitheretteError('wasm-memory-unavailable', 'wasm', errorMessages['wasm-memory-unavailable']);
	}
	if (status !== 0) throw failure(bindings, status);
	return new Processor(bindings, release);
}

class Processor implements Ditherette {
	#bindings: Bindings | undefined;
	#active = false;
	#release: (() => void) | undefined;

	constructor(bindings: Bindings, release?: () => void) {
		this.#bindings = bindings;
		this.#release = release;
	}

	process(request: ProcessRequest): IndexedImage {
		const bindings = this.#requireIdle();
		this.#active = true;
		try {
			const input = validateProcess(request);
			const policy = input.dither;
			const result: ResultSink<IndexedImage> = { value: undefined, onProgress: input.onProgress };
			let status: number;
			try {
				status = bindings.privateProcess(
					input.data,
					input.sourceWidth,
					input.sourceHeight,
					input.palette,
					1,
					input.outputWidth,
					input.outputHeight,
					input.algorithm,
					input.anchor,
					input.support,
					input.matching,
					input.alphaMode,
					input.threshold,
					input.matte,
					policy.family,
					policy.field,
					policy.parameter,
					policy.space,
					policy.strength,
					policy.placement,
					policy.radius,
					policy.threshold,
					policy.softness,
					result
				);
			} catch (error) {
				throw this.#trap(error);
			}
			if (status !== 0) throw failure(bindings, status, 'process');
			return result.value!;
		} finally {
			this.#active = false;
		}
	}

	resize(request: ResizeRequest): Rgba8Image {
		const bindings = this.#requireIdle();
		// Guard before touching caller properties: getters can attempt recursive calls too.
		this.#active = true;
		try {
			const input = validateResize(request);
			if (input.sourceWidth === input.outputWidth && input.sourceHeight === input.outputHeight) {
				try {
					input.onProgress?.({ stage: 'complete' });
				} catch {
					throw new DitheretteError('callback', 'onProgress', errorMessages.callback);
				}
				return input.source;
			}
			const result: ResultSink<Rgba8Image> = { value: undefined, onProgress: input.onProgress };
			let status: number;
			try {
				status = bindings.privateResize(
					input.data,
					input.sourceWidth,
					input.sourceHeight,
					input.outputWidth,
					input.outputHeight,
					input.algorithm,
					input.anchor,
					input.support,
					result
				);
			} catch (error) {
				throw this.#trap(error);
			}
			if (status !== 0) throw failure(bindings, status);
			// Success guarantees that the caught void helper populated this private result sink.
			return result.value!;
		} finally {
			this.#active = false;
		}
	}

	quantize(request: QuantizeRequest): IndexedImage {
		const bindings = this.#requireIdle();
		this.#active = true;
		try {
			const input = validateQuantize(request);
			const result: ResultSink<IndexedImage> = { value: undefined, onProgress: input.onProgress };
			let status: number;
			try {
				status = bindings.privateQuantize(
					input.data,
					input.sourceWidth,
					input.sourceHeight,
					input.palette,
					input.matching,
					input.alphaMode,
					input.threshold,
					input.matte,
					result
				);
			} catch (error) {
				throw this.#trap(error);
			}
			if (status !== 0) throw failure(bindings, status);
			return result.value!;
		} finally {
			this.#active = false;
		}
	}

	perturb(request: PerturbRequest): Rgba8Image {
		const bindings = this.#requireIdle();
		this.#active = true;
		try {
			const input = validatePerturb(request);
			const result: ResultSink<Rgba8Image> = { value: undefined, onProgress: input.onProgress };
			let status: number;
			try {
				status = bindings.privatePerturb(
					input.data,
					input.sourceWidth,
					input.sourceHeight,
					input.field,
					input.parameter,
					input.space,
					input.strength,
					input.placement,
					input.radius,
					input.threshold,
					input.softness,
					result
				);
			} catch (error) {
				throw this.#trap(error);
			}
			if (status !== 0) throw failure(bindings, status);
			return result.value!;
		} finally {
			this.#active = false;
		}
	}

	ditherAndQuantize(request: DitherAndQuantizeRequest): IndexedImage {
		const bindings = this.#requireIdle();
		this.#active = true;
		try {
			const input = validateDitherAndQuantize(request);
			const policy = input.dither;
			const result: ResultSink<IndexedImage> = { value: undefined, onProgress: input.onProgress };
			let status: number;
			try {
				status = bindings.privateDitherAndQuantize(
					input.data,
					input.sourceWidth,
					input.sourceHeight,
					input.palette,
					input.matching,
					input.alphaMode,
					input.threshold,
					input.matte,
					policy.family,
					policy.field,
					policy.parameter,
					policy.space,
					policy.strength,
					policy.placement,
					policy.radius,
					policy.threshold,
					policy.softness,
					result
				);
			} catch (error) {
				throw this.#trap(error);
			}
			if (status !== 0) throw failure(bindings, status, true);
			return result.value!;
		} finally {
			this.#active = false;
		}
	}

	dispose(): void {
		if (this.#active)
			throw new DitheretteError('reentrant-call', 'instance', errorMessages['reentrant-call']);
		if (!this.#bindings) return;
		const bindings = this.#bindings;
		this.#active = true;
		try {
			let status: number;
			try {
				status = bindings.privateDispose();
			} catch (error) {
				throw this.#trap(error);
			}
			if (status !== 0) throw failure(bindings, status);
			this.#bindings = undefined;
			this.#release?.();
			this.#release = undefined;
		} finally {
			this.#active = false;
		}
	}

	#requireIdle(): Bindings {
		if (this.#active)
			throw new DitheretteError('reentrant-call', 'instance', errorMessages['reentrant-call']);
		if (!this.#bindings) throw new DitheretteError('disposed', 'instance', errorMessages.disposed);
		return this.#bindings;
	}

	#trap(error: unknown): DitheretteError {
		// An uncaught trap may leave Rust control state incomplete. Discard only this instance.
		this.#bindings = undefined;
		this.#release?.();
		this.#release = undefined;
		const code = error instanceof RangeError ? 'wasm-memory-unavailable' : 'runtime';
		return new DitheretteError(code, 'wasm', errorMessages[code]);
	}
}
