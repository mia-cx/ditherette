import { initializeProcessor, normalizeWasmInput } from './scalar.js';
import type { Ditherette, InitInput } from './types.js';
import { WorkerPool } from './worker-pool.js';

/** One fresh shared memory and global Rayon pool per public processor. */
export async function createThreaded(options: { wasm?: InitInput; memoryLimitBytes: number }): Promise<Ditherette> {
	const pool = new WorkerPool();
	try {
		const { createThreadedBindings } = await import('./wasm/threads/ditherette_wasm.factory.js');
		const bindings = createThreadedBindings((module, memory, builder) =>
			pool.start(module, memory, builder, (value) => bindings.privateAbandonThreadPool(value)));
		await bindings.default({ module_or_path: normalizeWasmInput(options.wasm) });
		const processor = initializeProcessor(bindings, options.memoryLimitBytes, () => pool.dispose());
		const available = typeof navigator === 'undefined' ? 1 : navigator.hardwareConcurrency;
		try {
			await bindings.initThreadPool(bindings.privateThreadCount(available));
			return processor;
		} catch (error) {
			processor.dispose();
			throw error;
		}
	} catch (error) {
		pool.dispose();
		throw error;
	}
}
