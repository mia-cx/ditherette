// Rust catches these scalar/void imports without retaining returned JS handles.
const stages = ['prepare', 'resize', 'alpha', 'color', 'perturb', 'quantize', 'dither-and-quantize', 'complete'];

export function progressEnabled(sink) {
	if (sink.onProgress === undefined) return false;
	if (typeof sink.onProgress !== 'function') throw new TypeError('Expected a progress callback.');
	return true;
}

export function progressClock() {
	return performance.now();
}

export function reportProgress(sink, stage, completed, total) {
	const callback = sink.onProgress;
	try {
		callback({ stage: stages[stage], completed, total });
	} catch (error) {
		// Completion can fail after constructing a whole result; do not expose it through the sink.
		sink.value = undefined;
		throw error;
	}
}
