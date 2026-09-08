import { observeWorkers } from './thread-worker-observer.mjs';

Object.defineProperty(navigator, 'hardwareConcurrency', { value: 4 });
let processor;
let observed;
self.onmessage = async ({ data }) => {
	try {
		if (data.kind === 'initialize') {
			observed = observeWorkers('host-pool', { failAt: data.starting ? 1 : undefined });
			const { createDitherette } = await import(data.moduleUrl);
			processor = await createDitherette({ threads: 'required' });
			self.postMessage({ kind: 'ready', workers: observed.workers.length });
		} else if (data.kind === 'process') {
			processor.resize({
				version: 1,
				source: { width: 1, height: 1, data: new Uint8Array([19, 83, 127, 255]) },
				output: { width: 3, height: 3, resize: { algorithm: 'nearest', anchor: 'center' } },
				onProgress() {
					self.postMessage({ kind: 'processing' });
					// Hold a real synchronous call open until its host is destroyed.
					Atomics.wait(new Int32Array(data.gate), 0, 0);
				}
			});
			self.postMessage({ kind: 'unexpected-result' });
		}
	} catch (error) {
		self.postMessage({ kind: 'error', message: String(error), code: error.code, path: error.path });
	}
};
