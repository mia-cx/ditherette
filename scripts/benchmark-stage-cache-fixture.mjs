export const events = [];
let nextId = 0;
let callCount = 0;
let mismatchAt = 0;

export function reset(transientMismatchAt = 0) {
	events.length = 0;
	nextId = callCount = 0;
	mismatchAt = transientMismatchAt;
}

export async function createDitherette() {
	const id = ++nextId;
	events.push({ type: 'create', id });
	const call = (method, request) => {
		events.push({ type: method, id, source: [...request.source.data] });
		const mismatch = ++callCount === mismatchAt;
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
