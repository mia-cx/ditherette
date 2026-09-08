export const events = [];
let fail = false;
let nextId = 0;

export function reset(failCall = false) {
	events.length = 0;
	nextId = 0;
	fail = failCall;
}

export async function createDitherette() {
	const id = ++nextId;
	events.push({ type: 'create', id });
	return {
		resize(request) {
			events.push({ type: 'call', id, bytes: [...request.source.data] });
			if (fail) throw new Error('fixture call failed');
			return { width: 1, height: 1, data: request.source.data.slice() };
		},
		dispose() {
			events.push({ type: 'dispose', id });
		}
	};
}
