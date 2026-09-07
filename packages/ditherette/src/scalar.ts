import { DitheretteError } from './errors.js';
import type { ErrorCode } from './errors.js';
import type { Ditherette, InitInput, ResizeRequest, Rgba8Image } from './types.js';
import { validateResize } from './validation.js';

type Bindings = ReturnType<
	typeof import('./wasm/scalar/ditherette_wasm.factory.js').createScalarBindings
>;

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
	'output.resize'
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

function failure(bindings: Bindings, status: number): DitheretteError {
	const code = errorCodes[status - 1] ?? 'runtime';
	return new DitheretteError(
		code,
		errorPaths[bindings.privateErrorPath()] ?? 'wasm',
		errorMessages[code]
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
		let wasm = options.wasm;
		if (typeof Response !== 'undefined' && wasm instanceof Response) wasm = wasm.clone();
		if (typeof Request !== 'undefined' && wasm instanceof Request) wasm = wasm.clone();
		// Chromium rejects DataView at its Wasm boundary. Normalize every view without copying bytes.
		if (ArrayBuffer.isView(wasm))
			wasm = new Uint8Array(wasm.buffer, wasm.byteOffset, wasm.byteLength);
		await bindings.default({ module_or_path: wasm });
		let status: number;
		try {
			status = bindings.privateInitialize(options.memoryLimitBytes);
		} catch {
			// Pinned private initialization only adds fallible externref bookkeeping after preflight.
			throw new DitheretteError(
				'wasm-memory-unavailable',
				'wasm',
				errorMessages['wasm-memory-unavailable']
			);
		}
		if (status !== 0) throw failure(bindings, status);
		return new ScalarProcessor(bindings);
	} catch (error) {
		if (error instanceof DitheretteError) throw error;
		const code = error instanceof RangeError ? 'wasm-memory-unavailable' : 'initialization';
		throw new DitheretteError(code, 'wasm', errorMessages[code]);
	}
}

class ScalarProcessor implements Ditherette {
	#bindings: Bindings | undefined;
	#active = false;

	constructor(bindings: Bindings) {
		this.#bindings = bindings;
	}

	resize(request: ResizeRequest): Rgba8Image {
		const bindings = this.#requireIdle();
		// Guard before touching caller properties: getters can attempt recursive calls too.
		this.#active = true;
		try {
			const input = validateResize(request);
			const result: { value?: Rgba8Image } = { value: undefined };
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
		const code = error instanceof RangeError ? 'wasm-memory-unavailable' : 'runtime';
		return new DitheretteError(code, 'wasm', errorMessages[code]);
	}
}
