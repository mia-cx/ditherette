import { DitheretteError } from './errors.js';
import { createScalar } from './scalar.js';
import { validateOptions } from './validation.js';
import type { CpuInitOptions, Ditherette } from './types.js';

/** Shared CPU initialization for the public factory and GPU retirement paths. */
export async function createCpu(options?: Omit<CpuInitOptions, 'engine'>): Promise<Ditherette> {
	const normalized = validateOptions(options);
	if (typeof WebAssembly === 'undefined' || typeof WebAssembly.instantiate !== 'function')
		throw new DitheretteError('capability', 'wasm', 'WebAssembly is unavailable.');
	if (normalized.threads !== 'disabled') {
		let capable =
			globalThis.crossOriginIsolated === true &&
			typeof Worker === 'function' &&
			typeof SharedArrayBuffer === 'function' &&
			typeof Atomics === 'object' &&
			typeof Atomics.wait === 'function';
		if (capable) {
			try {
				Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, 0);
			} catch {
				capable = false;
			}
		}
		if (capable) {
			try {
				const { createThreaded } = await import('./threads.js');
				return await createThreaded(normalized);
			} catch (error) {
				if (error instanceof DitheretteError && error.code === 'memory-limit') throw error;
				if (normalized.threads === 'required') {
					if (error instanceof DitheretteError && error.code === 'wasm-memory-unavailable')
						throw error;
					if (error instanceof RangeError)
						throw new DitheretteError(
							'wasm-memory-unavailable',
							'wasm',
							'The browser could not allocate or copy processing memory.'
						);
					throw new DitheretteError(
						'initialization',
						'threads',
						'Required threaded initialization is unavailable.'
					);
				}
			}
		} else if (normalized.threads === 'required')
			throw new DitheretteError(
				'capability',
				'threads',
				'Required threaded initialization is unavailable.'
			);
	}
	return createScalar(normalized);
}
