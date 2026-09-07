/** One synchronous public call per latency sample. Preparation and cleanup remain outside its timer. */
export function timeCalls(call, iterations, now, outputs) {
	let output;
	const start = now();
	for (let index = 0; index < iterations; index += 1) {
		output = call();
		if (outputs) outputs[index] = output;
	}
	const elapsed = now() - start;
	return { output, elapsed, perCallNs: (elapsed * 1e6) / iterations };
}

// Protocol change: durable results live through their entire batch. Reference writes stay
// inside the existing timer; validation and slot allocation stay outside. Fresh artifacts are required.
export const RETAINED_OUTPUT_LIMIT = 64 * 1024 * 1024;
export const RESULT_BOOKKEEPING_BYTES = 1024; // Output records, typed-array wrappers, slots, and validation sets.
export function retainedOutputSlots(iterations, outputBytes) {
	// Reserve two additional results for the first/distinct stability evidence.
	const required = (iterations + 2) * (outputBytes + RESULT_BOOKKEEPING_BYTES);
	if (
		!Number.isSafeInteger(iterations) ||
		iterations < 1 ||
		!Number.isSafeInteger(outputBytes) ||
		outputBytes < 0 ||
		!Number.isSafeInteger(required) ||
		required > RETAINED_OUTPUT_LIMIT
	)
		throw new Error('Retained output batch exceeds the 64 MiB stability budget.');
	return Array(iterations).fill(undefined);
}

/** Initialization has its own asynchronous timer; processing never pays an artificial await. */
export async function timeInitialization(create, now) {
	const start = now();
	const instance = await create();
	return { instance, elapsed: now() - start };
}

/** Initialization samples exclude the correctness probe and disposal. */
export async function collectInitializations({
	measurement,
	create,
	probe,
	observe = () => {},
	outputBytes = 0,
	now = () => performance.now(),
	warmupAttemptLimit = 1_000_000
}) {
	const outputs = retainedOutputSlots(1, outputBytes);
	const warmupStart = now();
	let warmupIterations = 0;
	let warmupElapsed;
	async function one() {
		const timed = await timeInitialization(create, now);
		try {
			const output = probe(timed.instance);
			outputs[0] = output;
			observe(outputs);
			return { elapsed: timed.elapsed, output };
		} finally {
			outputs.fill(undefined);
			timed.instance.dispose();
		}
	}
	do {
		await one();
		warmupIterations += 1;
		warmupElapsed = now() - warmupStart;
		if (warmupIterations >= warmupAttemptLimit && warmupElapsed < measurement.warmup_ms)
			throw new Error('Insufficient timer resolution to bound warmup.');
	} while (warmupElapsed < measurement.warmup_ms);
	const samples = [];
	let output;
	let measuredElapsed = 0;
	for (let index = 0; index < measurement.samples; index += 1) {
		output = undefined;
		const timed = await one();
		output = timed.output;
		samples.push(timed.elapsed * 1e6);
		measuredElapsed += timed.elapsed;
		if (samples.length >= 5 && measuredElapsed >= measurement.measurement_ms) break;
	}
	return {
		output,
		sample_ns: samples,
		iterations_per_sample: 1,
		warmup_iterations: warmupIterations,
		warmup_elapsed_ns: Math.round(warmupElapsed * 1e6)
	};
}

/** Run warmup first, then prepare/reset each sample. Hooks never add hashing to the operation. */
export async function collectCalls({
	measurement,
	prepare,
	observe = () => {},
	outputBytes = 0,
	now = () => performance.now(),
	warmupAttemptLimit = 1_000_000
}) {
	const warmupOutputs = retainedOutputSlots(1, outputBytes);
	let warmupIterations = 0;
	let warmupElapsed = 0;
	let warmupCallElapsed = 0;
	const warmupStart = now();
	do {
		const prepared = await prepare();
		try {
			warmupCallElapsed += timeCalls(prepared.call, 1, now, warmupOutputs).elapsed;
			observe(warmupOutputs);
			warmupIterations += 1;
		} finally {
			warmupOutputs.fill(undefined);
			prepared.close();
		}
		// This observation includes untimed preparation, validation, and disposal.
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
	// Reject an oversized calibrated batch before preparing or timing a sample. Never lower its count.
	const outputs = retainedOutputSlots(iterations, outputBytes);
	const samples = [];
	let output;
	let measuredElapsed = 0;
	for (let index = 0; index < measurement.samples; index += 1) {
		const prepared = await prepare();
		try {
			output = undefined; // Release the previous final result before entering a new batch.
			const timed = timeCalls(prepared.call, iterations, now, outputs);
			observe(outputs);
			output = timed.output;
			samples.push(timed.perCallNs);
			measuredElapsed += timed.elapsed;
		} finally {
			outputs.fill(undefined);
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
