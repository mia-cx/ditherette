export const events = [];
let nextId = 0;
let callCount = 0;
let mismatchAt = 0;
let progressCalls = 0;
let progressFailure;
let initializationFailure = 0;

export function failInitialization(at) {
	initializationFailure = at;
}

export function failProgress(failure) {
	progressFailure = failure;
}

export function reset(transientMismatchAt = 0) {
	events.length = 0;
	nextId = callCount = 0;
	mismatchAt = transientMismatchAt;
	progressCalls = 0;
	progressFailure = undefined;
	initializationFailure = 0;
}

export async function createDitherette(options) {
	const id = ++nextId;
	events.push({ type: 'initialize', id, threads: options.threads,
		wasm: options.wasm instanceof WebAssembly.Module ? 'compiled' : 'bytes' });
	if (id === initializationFailure) throw new Error('injected required startup failure');
	events.push({ type: 'create', id });
	const call = (method, request) => {
		events.push({ type: method, id, source: [...request.source.data] });
		const mismatch = ++callCount === mismatchAt;
		if (request.onProgress) {
			const failure = ++progressCalls === progressFailure?.at ? progressFailure.kind : undefined;
			events.push({ type: 'callback', id });
			request.onProgress(
				failure === 'throw'
					? {
							get stage() {
								throw new Error('injected callback failure');
							}
						}
					: { stage: 'prepare', completed: 0, total: 1 }
			);
			if (failure !== 'incomplete')
				request.onProgress({ stage: 'complete', completed: 1, total: 1 });
		}
		if (method === 'resize' || method === 'perturb') {
			const data = request.source.data.slice();
			if (mismatch) data[0] = 99;
			return { width: 1, height: 1, data };
		}
		return {
			width: 1,
			height: 1,
			indices: new Uint8Array([mismatch ? 1 : 0]),
			palette: { rgba: new Uint8Array([0, 0, 0, 255]), transparentIndex: null },
			warnings: []
		};
	};
	return {
		resize: (request) => call('resize', request),
		quantize: (request) => call('quantize', request),
		perturb: (request) => call('perturb', request),
		ditherAndQuantize: (request) => call('ditherAndQuantize', request),
		process: (request) => call('process', request),
		dispose: () => events.push({ type: 'dispose', id })
	};
}
