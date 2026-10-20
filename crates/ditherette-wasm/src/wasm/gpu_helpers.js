// Caught by the async Rust bridge, including failures in host control callbacks.
export function checkJob(sink, completed) {
	return sink.check(completed);
}

// Timing is optional for older/private transport callers.
export function timingJob(sink, name, begin) {
	sink.timing?.(name, begin);
}

export function completeCoverage(source, sink) {
	const bytes = new Uint8Array(source.length);
	Uint8Array.prototype.set.call(bytes, source);
	sink.coverage = bytes;
}
