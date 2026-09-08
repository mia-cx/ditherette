import { createThreadedBindings } from './wasm/threads/ditherette_wasm.factory.js';

// Each dedicated worker owns its glue/TLS state and shares only its processor's memory.
const bindings = createThreadedBindings(async () => { throw new Error('Nested pool initialization is unavailable.'); });
type Command =
	| { type: 'ditherette-worker-init'; module: WebAssembly.Module; memory: WebAssembly.Memory }
	| { type: 'ditherette-worker-start'; receiver: number };
globalThis.onmessage = async ({ data }: MessageEvent<Command>) => {
	try {
		if (data.type === 'ditherette-worker-init') {
			await bindings.default({ module_or_path: data.module, memory: data.memory });
			globalThis.postMessage({ type: 'ditherette-worker-ready' });
		} else if (data.type === 'ditherette-worker-start') {
			bindings.wbg_rayon_start_worker(data.receiver);
		}
	} catch {
		globalThis.postMessage({ type: 'ditherette-worker-error' });
	}
};
