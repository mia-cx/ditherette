/** One synchronous public call per latency sample. Preparation and cleanup remain outside its timer. */
export function timeCalls(call, iterations, now) {
	let output;
	const start = now();
	for (let index = 0; index < iterations; index += 1) output = call();
	const elapsed = now() - start;
	return { output, elapsed, perCallNs: (elapsed * 1e6) / iterations };
}

/** Initialization has its own asynchronous timer; processing never pays an artificial await. */
export async function timeInitialization(create, now) {
	const start = now();
	const instance = await create();
	return { instance, elapsed: now() - start };
}

/** Run warmup first, then prepare/reset each sample. Hooks never add hashing to the operation. */
export async function collectCalls({
	measurement,
	prepare,
	now = () => performance.now(),
	warmupAttemptLimit = 1_000_000
}) {
	let warmupIterations = 0;
	let warmupElapsed = 0;
	let warmupCallElapsed = 0;
	const warmupStart = now();
	do {
		const prepared = await prepare();
		try {
			warmupCallElapsed += timeCalls(prepared.call, 1, now).elapsed;
			warmupIterations += 1;
		} finally {
			prepared.close();
		}
		// This observation includes untimed preparation/disposal, not just operation durations.
		warmupElapsed = now() - warmupStart;
		if (warmupIterations >= warmupAttemptLimit && warmupElapsed < measurement.warmup_ms) {
			throw new Error('Insufficient timer resolution to bound warmup.');
		}
	} while (warmupElapsed < measurement.warmup_ms);
	const iterations =
		measurement.mode === 'single-call'
			? 1
			: Math.max(
					1,
					Math.ceil(measurement.target_sample_ms / (warmupCallElapsed / warmupIterations))
				);
	if (!Number.isSafeInteger(iterations))
		throw new Error('Insufficient timer resolution for throughput calibration.');
	const samples = [];
	let output;
	let measuredElapsed = 0;
	for (let index = 0; index < measurement.samples; index += 1) {
		const prepared = await prepare();
		try {
			const timed = timeCalls(prepared.call, iterations, now);
			output = timed.output;
			samples.push(timed.perCallNs);
			measuredElapsed += timed.elapsed;
		} finally {
			prepared.close();
		}
		if (samples.length >= 5 && measuredElapsed >= measurement.measurement_ms) break;
	}
	return {
		output,
		sample_ns: samples,
		iterations_per_sample: iterations,
		warmup_iterations: warmupIterations,
		warmup_elapsed_ns: Math.round(warmupElapsed * 1e6)
	};
}
