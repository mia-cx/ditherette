import { collectCalls, collectInitializations } from './benchmark-public-timing.mjs';

/** Observe the browser clock quantum without changing, retrying, or censoring operation samples. */
export function timerResolution(now = () => performance.now()) {
	let previous = now();
	let quantum = Infinity;
	for (let index = 0; index < 100_000; index += 1) {
		const current = now();
		if (current > previous) quantum = Math.min(quantum, current - previous);
		previous = current;
	}
	if (!Number.isFinite(quantum)) throw new Error('Could not observe browser timer resolution.');
	return quantum * 1e6;
}

/** Prepare only existing public operations. No private glue, synthetic hashing, or application caches. */
export function resizeRecipe(operation) {
	switch (operation.operation) {
		case 'resize-nearest':
			return { algorithm: 'nearest', anchor: operation.anchor };
		case 'resize-area':
			return { algorithm: 'area' };
		case 'resize-bilinear':
			return { algorithm: 'bilinear', anchor: operation.anchor };
		case 'resize-bicubic':
		case 'resize-lanczos2':
		case 'resize-lanczos3':
			return {
				algorithm: operation.operation.slice('resize-'.length),
				anchor: operation.anchor,
				support: operation.support
			};
		default:
			throw new Error('Unsupported browser operation.');
	}
}

/** Prepare the actual package or website call outside measurement timers. */
export async function prepareOperation(trial) {
	const config = trial.case.browser;
	const backend = config[trial.role];
	const measurement = trial.case.measurement;
	const quantize = config.operation.operation === 'quantize';
	const resize = quantize ? undefined : resizeRecipe(config.operation);
	if (config.cache !== 'none' || measurement.application_cache !== 'not-applicable')
		throw new Error('This package has no application cache.');
	if (measurement.mode === 'throughput' && config.preparation !== 'primed-instance')
		throw new Error('Throughput requires a primed instance.');
	const request = {
		version: 1,
		source: { ...trial.case.source, data: new Uint8Array(trial.case.rgba) },
		...(quantize
			? config.operation.settings
			: {
					output: {
						...trial.case.identity.output,
						resize
					}
				})
	};
	const url = (entry) => new URL(`/${entry}`, location.href).href;
	if (backend === 'typescript') {
		if (quantize) throw new Error('No faithful TypeScript indexed quantize adapter is registered.');
		if (resize.algorithm === 'bicubic')
			throw new Error('The website has no bicubic implementation.');
		if ('anchor' in resize && resize.anchor !== 'center')
			throw new Error('TypeScript non-center resize is unavailable.');
		if (
			measurement.scope !== 'complete-call' ||
			!['primed-instance', 'fresh-instance'].includes(config.preparation)
		)
			throw new Error('TypeScript has no Wasm initialization or processor-instance equivalent.');
		// TypeScript is stateless: both labels execute its ordinary per-call preparation.
		const { resize: resizeTypeScript } = await import(url(trial.browser.assets.entries.typescript));
		return {
			request,
			call: () => resizeTypeScript(request),
			prepare: async () => ({ call: () => resizeTypeScript(request), close() {} }),
			close() {}
		};
	}
	if (backend !== 'package') throw new Error('Unknown browser backend.');
	const { createDitherette } = await import(url(trial.browser.assets.entries.package));
	const response = await fetch(url(trial.browser.assets.entries.wasm));
	if (!response.ok) throw new Error(`Wasm fetch failed: ${response.status}`);
	const bytes = await response.arrayBuffer();
	// Load the real lazy factory module before timing. Browser compilation caches are not reset.
	const preload = await createDitherette({ wasm: bytes });
	preload.dispose();
	const compiled =
		config.preparation === 'initialization-bytes' ? undefined : await WebAssembly.compile(bytes);
	const create = () => createDitherette({ wasm: compiled ?? bytes });
	const call = (instance) => (quantize ? instance.quantize(request) : instance.resize(request));
	if (measurement.scope === 'initialization') {
		if (!['initialization-bytes', 'initialization-compiled'].includes(config.preparation))
			throw new Error('Initialization requires an explicit compilation scope.');
		return { request, create, probe: call, close() {} };
	}
	if (measurement.scope !== 'complete-call')
		throw new Error('Browser processing requires complete-call scope.');
	if (config.preparation === 'fresh-instance') {
		return {
			request,
			prepare: async () => {
				const instance = await create();
				return { call: () => call(instance), close: () => instance.dispose() };
			},
			close() {}
		};
	}
	if (config.preparation !== 'primed-instance') throw new Error('Unknown processing preparation.');
	const instance = await create();
	return {
		request,
		call: () => call(instance),
		prepare: async () => ({ call: () => call(instance), close() {} }),
		close: () => instance.dispose()
	};
}

export function verificationOutput(output) {
	if ('indices' in output) {
		return {
			dimensions: { width: output.width, height: output.height },
			pixels: {
				format: 'indexed8',
				indices: Array.from(output.indices),
				palette_rgba: Array.from(output.palette.rgba),
				transparent_index: output.palette.transparentIndex
			},
			warnings: output.warnings.map(({ code, message }) => ({ code, message }))
		};
	}
	return {
		dimensions: { width: output.width, height: output.height },
		pixels: { format: 'rgba8', data: Array.from(output.data) },
		warnings: []
	};
}

/** Compare one untimed actual call with worker-supplied frozen bytes; return concrete mismatch evidence. */
export async function preflightOperation(operation, reference) {
	if (!reference || !['rgba8', 'indexed8'].includes(reference.pixels.format))
		throw new Error('Browser trial requires frozen RGBA8 or indexed reference_output.');
	let output;
	if (operation.create) {
		const instance = await operation.create();
		try {
			output = operation.probe(instance);
		} finally {
			instance.dispose();
		}
	} else {
		const prepared = await operation.prepare();
		try {
			output = prepared.call();
		} finally {
			prepared.close();
		}
	}
	const actual = verificationOutput(output);
	if (equalOutput(actual, reference)) return undefined;
	return actual;
}

/** Exact bytes and metadata, independent of JSON object key order. */
export function equalOutput(actual, expected) {
	const equalBytes = (a, b) => a.length === b.length && a.every((byte, index) => byte === b[index]);
	if (
		actual.dimensions.width !== expected.dimensions.width ||
		actual.dimensions.height !== expected.dimensions.height ||
		actual.pixels.format !== expected.pixels.format ||
		actual.warnings.length !== expected.warnings.length ||
		actual.warnings.some(
			(warning, index) =>
				warning.code !== expected.warnings[index].code ||
				warning.message !== expected.warnings[index].message
		)
	)
		return false;
	if (actual.pixels.format === 'rgba8') return equalBytes(actual.pixels.data, expected.pixels.data);
	if (actual.pixels.format !== 'indexed8') return false;
	return (
		equalBytes(actual.pixels.indices, expected.pixels.indices) &&
		equalBytes(actual.pixels.palette_rgba, expected.pixels.palette_rgba) &&
		actual.pixels.transparent_index === expected.pixels.transparent_index
	);
}

/** Invoked only by the leased transport. All serialization and observations are outside call timers. */
export async function runTrial(trial) {
	const resolution = timerResolution();
	const operation = await prepareOperation(trial);
	try {
		const identity = {
			role: trial.role,
			pair: trial.pair,
			case_name: trial.case.name,
			input: trial.case.identity.input,
			settings: trial.case.identity.settings
		};
		const observation = {
			user_agent: navigator.userAgent,
			cross_origin_isolated: crossOriginIsolated,
			timer_resolution_ns: resolution
		};
		const mismatch = await preflightOperation(operation, trial.reference_output);
		if (!operation.request.source.data.every((byte, index) => byte === trial.case.rgba[index]))
			throw new Error('Operation mutated source bytes during preflight.');
		if (mismatch && trial.case.browser.measure_nonexact !== true)
			return {
				...identity,
				sample_ns: [],
				iterations_per_sample: 0,
				warmup_iterations: 0,
				warmup_elapsed_ns: 0,
				output: mismatch,
				observation,
				timing_skipped: 'reference-mismatch'
			};
		const measurement = trial.case.measurement;
		const measured =
			measurement.scope === 'initialization'
				? await collectInitializations({
						measurement,
						create: operation.create,
						probe: operation.probe
					})
				: await collectCalls({ measurement, prepare: operation.prepare });
		if (!operation.request.source.data.every((byte, index) => byte === trial.case.rgba[index]))
			throw new Error('Operation mutated source bytes.');
		const { output, ...timings } = measured;
		const verified = verificationOutput(output);
		return {
			...identity,
			...timings,
			output: verified,
			...(mismatch && !equalOutput(verified, mismatch) ? { unstable_output: mismatch } : {}),
			observation
		};
	} finally {
		operation.close();
	}
}
