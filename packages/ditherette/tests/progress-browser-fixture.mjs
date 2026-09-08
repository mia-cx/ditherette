/** Exercise callback contracts through installed public methods, without private cache observations. */
export async function progressBrowserChecks({
	wasmUrl,
	vectors,
	moduleUrl = '/node_modules/ditherette/dist/index.js'
}) {
	const { createDitherette, DitheretteError } = await import(moduleUrl);
	const wasm = await WebAssembly.compile(await (await fetch(wasmUrl)).arrayBuffer());
	const reference = await createDitherette({ wasm });
	const instances = [reference];
	const create = async () => {
		const instance = await createDitherette({ wasm });
		instances.push(instance);
		return instance;
	};
	const assert = (condition, label) => {
		if (!condition) throw new Error(label);
	};
	const same = (actual, expected, label) =>
		assert(JSON.stringify(actual) === JSON.stringify(expected), label);
	const rejects = (run, code, path) => {
		let returned;
		try {
			returned = run();
		} catch (error) {
			assert(error instanceof DitheretteError, 'structured package error');
			same([error.code, error.path], [code, path], 'error category and path');
			if (code === 'callback')
				assert(error.message === 'Progress callback threw.', 'callback message');
			assert(returned === undefined, 'failed call exposes no partial output');
			return;
		}
		throw new Error(`Expected ${code} at ${path}`);
	};
	const source = { ...vectors.source, data: new Uint8Array(vectors.source.data) };
	const palette = [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'transparent' }
	];
	const alpha = { mode: 'preserve', threshold: 127.9999999 };
	const matching = 'srgb-euclidean';
	const output = { width: 3, height: 3, resize: { algorithm: 'nearest', anchor: 'center' } };
	const perturb = vectors.cases[0].policy;
	const dither = { family: 'separable', perturb };
	const quantize = { version: 1, source, palette, alpha, matching };
	const requests = [
		['resize', { version: 1, source, output }],
		['perturb', { version: 1, source, perturb }],
		['quantize', quantize],
		['ditherAndQuantize', { ...quantize, dither }],
		['process', { source, palette, recipe: { version: 1, output, alpha, match: matching, dither } }]
	];
	const stages = new Set([
		'prepare',
		'resize',
		'alpha',
		'color',
		'perturb',
		'quantize',
		'dither-and-quantize',
		'complete'
	]);
	// A controlled system clock avoids sleeps and machine-dependent latency assertions.
	const clockDescriptor = Object.getOwnPropertyDescriptor(performance, 'now');
	let clock = 0;
	let step = 0;
	Object.defineProperty(performance, 'now', {
		configurable: true,
		value: () => (clock += step)
	});
	const observe = (events) => (event) => {
		assert(stages.has(event.stage), 'typed progress stage');
		for (const key of ['completed', 'total']) {
			if (event[key] !== undefined)
				assert(
					Number.isSafeInteger(event[key]) && event[key] >= 0,
					'measurable nonnegative counts'
				);
		}
		if (event.completed !== undefined && event.total !== undefined)
			assert(event.completed <= event.total, 'completed work does not exceed total');
		const previous = events.at(-1);
		assert(previous?.stage !== 'complete', 'completion is the last callback');
		if (previous?.stage === event.stage) {
			assert(clock - previous.at >= 50, 'within-stage callbacks respect 50 ms');
			if (previous.completed !== undefined && event.completed !== undefined)
				assert(event.completed >= previous.completed, 'completed work is monotonic within a stage');
		}
		events.push({ ...event, at: clock });
	};
	const checkCall = (instance, method, request, expected, cold) => {
		const events = [];
		let returned = false;
		const result = instance[method]({
			...request,
			onProgress(event) {
				assert(!returned, 'callbacks run synchronously before method return');
				observe(events)(event);
			}
		});
		returned = true;
		same(result, expected, `${method}: callbacks preserve bytes and metadata`);
		assert(events.at(-1)?.stage === 'complete', `${method}: successful call reports completion`);
		if (cold) {
			assert(
				events.some((event) => !['prepare', 'complete'].includes(event.stage)),
				'cold work reports its stage'
			);
			assert(
				events.some((event) => event.total > 0 && event.completed !== undefined),
				'cold work has measured counts'
			);
		}
		return result;
	};
	let callbackFailures = 0;
	let reentrantAttempts = 0;
	try {
		for (const [method, request] of requests) {
			const instance = await create();
			const expected = reference[method](request);
			const first = checkCall(instance, method, request, expected, true);
			const saved = structuredClone(first);
			checkCall(instance, method, request, expected, false);
			same(first, saved, `${method}: later progress leaves earlier output durable`);
			instance.dispose();
			same(first, saved, `${method}: disposal leaves output durable`);

			const failing = await create();
			for (const phase of ['intermediate', 'cold-complete', 'warm-complete']) {
				const changedSource = { ...source, data: new Uint8Array(source.data) };
				if (phase !== 'intermediate') changedSource.data[0] ^= 127;
				const changed = { ...request, source: changedSource };
				const wanted = reference[method](changed);
				let caughtStage;
				rejects(
					() =>
						failing[method]({
							...changed,
							onProgress(event) {
								if (
									phase === 'intermediate'
										? !['prepare', 'complete'].includes(event.stage)
										: event.stage === 'complete'
								) {
									caughtStage = event.stage;
									throw new Error('caller failure');
								}
							}
						}),
					'callback',
					'onProgress'
				);
				assert(caughtStage !== undefined, `${method}: requested callback failure executes`);
				same(failing[method](changed), wanted, `${method}: callback failure recovers`);
				callbackFailures++;
			}
			failing.dispose();
		}

		// The outer callback catches recursion errors, so it can finish normally.
		const instance = await create();
		const outer = requests[4];
		const independent = reference.resize(requests[0][1]);
		let exercised = false;
		instance.process({
			...outer[1],
			onProgress() {
				if (exercised) return;
				exercised = true;
				for (const [method, request] of requests) {
					rejects(() => instance[method](request), 'reentrant-call', 'instance');
					reentrantAttempts++;
				}
				rejects(() => instance.dispose(), 'reentrant-call', 'instance');
				reentrantAttempts++;
				// The recursion guard belongs to one instance, not the module.
				same(
					reference.resize(requests[0][1]),
					independent,
					'another instance remains usable inside callback'
				);
			}
		});
		assert(exercised, 'reentry probe ran');

		// Reject the last durable result copy. Completion must not have fired yet.
		for (const [method, request, lastCopy] of [
			['resize', requests[0][1], 2],
			['quantize', requests[2][1], 3]
		]) {
			const copying = await create();
			const set = Uint8Array.prototype.set;
			let copies = 0;
			let completed = false;
			Uint8Array.prototype.set = function (...args) {
				if (++copies === lastCopy) throw new RangeError('durable copy failure');
				return Reflect.apply(set, this, args);
			};
			try {
				rejects(
					() =>
						copying[method]({
							...request,
							onProgress(event) {
								if (event.stage === 'complete') completed = true;
							}
						}),
					'wasm-memory-unavailable',
					'output'
				);
			} finally {
				Uint8Array.prototype.set = set;
			}
			assert(copies === lastCopy && !completed, 'completion follows durable output construction');
			checkCall(copying, method, request, reference[method](request), true);
			copying.dispose();
		}

		// Advance the clock across real work checks. Do not require a particular row-band schedule.
		step = 20;
		const rows = { width: 16, height: 64, data: new Uint8Array(16 * 64 * 4).fill(255) };
		const counted = { ...quantize, source: rows };
		const timed = await create();
		checkCall(timed, 'quantize', counted, reference.quantize(counted), true);
		same([...source.data], vectors.source.data, 'callbacks do not mutate caller input');
		return {
			methods: 5,
			callbackFailures,
			reentrantAttempts,
			finalCopyFailures: 2,
			controlledClock: true
		};
	} finally {
		if (clockDescriptor) Object.defineProperty(performance, 'now', clockDescriptor);
		else delete performance.now;
		for (const instance of instances) instance.dispose();
	}
}
