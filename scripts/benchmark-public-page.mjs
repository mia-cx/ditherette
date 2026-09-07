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
export async function prepareOperation(trial) {
	const config = trial.case.browser;
	const backend = config[trial.role];
	const measurement = trial.case.measurement;
	if (config.operation.operation !== 'resize-nearest')
		throw new Error('Unsupported browser operation.');
	if (config.cache !== 'none' || measurement.application_cache !== 'not-applicable')
		throw new Error('This package has no application cache.');
	if (measurement.mode === 'throughput' && config.preparation !== 'primed-instance')
		throw new Error('Throughput requires a primed instance.');
	const request = {
		version: 1,
		source: { ...trial.case.source, data: new Uint8Array(trial.case.rgba) },
		output: {
			...trial.case.identity.output,
			resize: { algorithm: 'nearest', anchor: config.operation.anchor }
		}
	};
	const url = (entry) => new URL(`/${entry}`, location.href).href;
	if (backend === 'typescript') {
		if (config.operation.anchor !== 'center')
			throw new Error('TypeScript non-center nearest is unavailable.');
		if (
			measurement.scope !== 'complete-call' ||
			!['primed-instance', 'fresh-instance'].includes(config.preparation)
		)
			throw new Error('TypeScript has no Wasm initialization or processor-instance equivalent.');
		// TypeScript is stateless: both labels execute its ordinary per-call preparation.
		const { resize } = await import(url(trial.browser.assets.entries.typescript));
		return {
			request,
			call: () => resize(request),
			prepare: async () => ({ call: () => resize(request), close() {} }),
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
	if (measurement.scope === 'initialization') {
		if (!['initialization-bytes', 'initialization-compiled'].includes(config.preparation))
			throw new Error('Initialization requires an explicit compilation scope.');
		return { request, create, probe: (instance) => instance.resize(request), close() {} };
	}
	if (measurement.scope !== 'complete-call')
		throw new Error('Browser processing requires complete-call scope.');
	if (config.preparation === 'fresh-instance') {
		return {
			request,
			prepare: async () => {
				const instance = await create();
				return { call: () => instance.resize(request), close: () => instance.dispose() };
			},
			close() {}
		};
	}
	if (config.preparation !== 'primed-instance') throw new Error('Unknown processing preparation.');
	const instance = await create();
	return {
		request,
		call: () => instance.resize(request),
		prepare: async () => ({ call: () => instance.resize(request), close() {} }),
		close: () => instance.dispose()
	};
}

/** Invoked only by the leased transport. All serialization and observations are outside call timers. */
export async function runTrial(trial) {
	const resolution = timerResolution();
	const operation = await prepareOperation(trial);
	try {
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
		return {
			role: trial.role,
			pair: trial.pair,
			case_name: trial.case.name,
			input: trial.case.identity.input,
			settings: trial.case.identity.settings,
			...timings,
			output: {
				dimensions: { width: output.width, height: output.height },
				pixels: { format: 'rgba8', data: Array.from(output.data) },
				warnings: []
			},
			observation: {
				user_agent: navigator.userAgent,
				cross_origin_isolated: crossOriginIsolated,
				timer_resolution_ns: resolution
			}
		};
	} finally {
		operation.close();
	}
}
