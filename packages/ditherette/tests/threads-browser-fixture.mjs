/** Exercise selected artifacts only through the installed public factory and processing methods. */
export async function scalarSelectionChecks({ moduleUrl, capabilityUnavailable = true }) {
	const { createDitherette, DitheretteError } = await import(moduleUrl);
	const request = {
		version: 1,
		source: { width: 1, height: 1, data: new Uint8Array([19, 83, 127, 255]) },
		output: { width: 1, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } }
	};
	const selections = capabilityUnavailable
		? [undefined, 'disabled', 'preferred']
		: [undefined, 'disabled'];
	for (const threads of selections) {
		const instance = await createDitherette({ threads });
		try {
			if (String(instance.resize(request).data) !== '19,83,127,255')
				throw new Error('Scalar selection changed exact output.');
		} finally {
			instance.dispose();
		}
	}
	if (!capabilityUnavailable) return { scalarSelections: 2, requiredCapabilityError: false };
	try {
		await createDitherette({ threads: 'required' });
	} catch (error) {
		if (error instanceof DitheretteError && error.code === 'capability' && error.path === 'threads')
			return { scalarSelections: 3, requiredCapabilityError: true };
		throw error;
	}
	throw new Error('Required threads unexpectedly initialized without isolation.');
}

/** Reusing custom Wasm inputs must not consume them or share processor memory. */
export async function customThreadedInputs({ moduleUrl, wasmUrl }) {
	const { createDitherette } = await import(moduleUrl);
	const bytes = await (await fetch(wasmUrl)).arrayBuffer();
	const padded = new Uint8Array(bytes.byteLength + 7);
	padded.set(new Uint8Array(bytes), 3);
	const response = new Response(bytes, { headers: { 'Content-Type': 'application/wasm' } });
	const request = new Request(wasmUrl);
	const module = await WebAssembly.compile(bytes);
	const inputs = [
		['string', wasmUrl],
		['URL', new URL(wasmUrl)],
		['Request', request],
		['Response first use', response],
		['Response reuse', response],
		['ArrayBuffer', bytes],
		['offset Uint8Array', padded.subarray(3, 3 + bytes.byteLength)],
		['offset DataView', new DataView(padded.buffer, 3, bytes.byteLength)],
		['WebAssembly.Module', module]
	];
	const image = {
		version: 1,
		source: { width: 1, height: 1, data: new Uint8Array([19, 83, 127, 255]) },
		output: { width: 1, height: 1, resize: { algorithm: 'nearest', anchor: 'center' } }
	};
	for (const [form, wasm] of inputs) {
		let instance;
		try {
			instance = await createDitherette({ threads: 'required', wasm });
			const output = instance.resize(image);
			instance.dispose();
			if (String(output.data) !== '19,83,127,255')
				throw new Error('Custom input changed durable output.');
		} catch (error) {
			if (error instanceof Error) {
				error.message = `${form}: ${error.message} (code=${error.code}, path=${error.path})`;
				throw error;
			}
			throw new Error(`${form}: ${String(error)}`, { cause: error });
		} finally {
			instance?.dispose();
		}
	}
	if (response.bodyUsed || request.bodyUsed)
		throw new Error('Initialization consumed caller input.');
	if (padded[0] !== 0 || padded[padded.length - 1] !== 0)
		throw new Error('Wasm view guards changed.');
	return { customInputs: inputs.length };
}

/** Keep two real pools live while exercising per-instance memory and callback guards. */
export async function initializeThreadedPair({ moduleUrl, wasmUrl, vectors }) {
	const { createDitherette, DitheretteError } = await import(moduleUrl);
	const { observeWorkers } = await import('/__tests__/thread-worker-observer.mjs');
	Object.defineProperty(navigator, 'hardwareConcurrency', { value: 4, configurable: true });
	const observed = observeWorkers('pair');
	const memories = [];
	const Memory = WebAssembly.Memory;
	WebAssembly.Memory = new Proxy(Memory, {
		construct(target, args) {
			const memory = Reflect.construct(target, args);
			memories.push(memory);
			return memory;
		}
	});
	const wasm = await WebAssembly.compile(await (await fetch(wasmUrl)).arrayBuffer());
	const instances = [];
	let reference;
	try {
		instances.push(
			await createDitherette({ threads: 'required', wasm, memoryLimitBytes: 100_000 })
		);
		const firstWorkers = observed.workers.length;
		instances.push(
			await createDitherette({ threads: 'required', wasm, memoryLimitBytes: 2_000_000 })
		);
		if (firstWorkers === 0 || observed.workers.length <= firstWorkers)
			throw new Error('Each processor needs its own pool.');
		const shared = memories.filter((memory) => memory.buffer instanceof SharedArrayBuffer);
		if (shared.length !== 2 || shared[0] === shared[1] || shared[0].buffer === shared[1].buffer)
			throw new Error('Compiled code reuse must allocate separate shared memories.');
		reference = await createDitherette({ threads: 'disabled' });
		globalThis.threadPair = {
			instances,
			reference,
			observed,
			DitheretteError,
			vectors,
			firstWorkers
		};
		return { firstWorkers, totalWorkers: observed.workers.length };
	} catch (error) {
		reference?.dispose();
		for (const instance of instances) instance.dispose();
		for (const worker of observed.workers) worker.terminate();
		observed.restore();
		throw error;
	} finally {
		WebAssembly.Memory = Memory;
	}
}

export function exerciseThreadedPair() {
	const {
		instances: [first, second],
		DitheretteError,
		reference,
		vectors
	} = globalThis.threadPair;
	const same = (a, b, message) => {
		if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error(message);
	};
	const rejects = (run, code, path) => {
		try {
			run();
		} catch (error) {
			if (error instanceof DitheretteError && error.code === code && error.path === path) return;
			throw error;
		}
		throw new Error(`Expected ${code} at ${path}.`);
	};
	const source = { ...vectors.source, data: new Uint8Array(vectors.source.data) };
	const output = { width: 3, height: 3, resize: { algorithm: 'nearest', anchor: 'center' } };
	const palette = [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] }
	];
	const alpha = { mode: 'matte', rgb: [255, 255, 255] };
	const perturb = vectors.cases[0].policy;
	const dither = { family: 'separable', perturb };
	const quantize = { version: 1, source, palette, alpha, matching: 'srgb-euclidean' };
	const requests = [
		['resize', { version: 1, source, output }],
		['perturb', { version: 1, source, perturb }],
		['quantize', quantize],
		['ditherAndQuantize', { ...quantize, dither }],
		[
			'process',
			{ source, palette, recipe: { version: 1, output, alpha, match: 'srgb-euclidean', dither } }
		]
	];
	const saved = [];
	for (const [method, request] of requests) {
		const expected = reference[method](request);
		same(second[method](request), expected, 'Threaded output matches the scalar artifact.');
		for (let repeat = 0; repeat < 2; repeat++) {
			const stages = [];
			const result = first[method]({
				...request,
				onProgress(event) {
					stages.push(event.stage);
					rejects(() => first.dispose(), 'reentrant-call', 'instance');
					rejects(() => first[method](request), 'reentrant-call', 'instance');
					same(
						second[method](request),
						expected,
						'Other instance remains callable during progress.'
					);
				}
			});
			if (stages.length === 0 || stages.at(-1) !== 'complete')
				throw new Error('Threaded progress must complete.');
			same(result, expected, 'Threaded repeated output is exact.');
			saved.push([result, structuredClone(result)]);
		}
		for (const completion of [false, true]) {
			rejects(
				() =>
					first[method]({
						...request,
						onProgress(event) {
							if ((event.stage === 'complete') === completion)
								throw new Error('Caller callback failure.');
						}
					}),
				'callback',
				'onProgress'
			);
			same(first[method](request), expected, 'Callback failure leaves this pool usable.');
		}
	}
	same([...first.perturb(requests[1][1]).data], vectors.cases[0].rgba, 'Frozen field output.');
	const oversized = { ...requests[0][1], output: { ...output, width: 256, height: 256 } };
	rejects(() => first.resize(oversized), 'memory-limit', 'memoryLimitBytes');
	if (second.resize(oversized).data.length !== 256 * 256 * 4)
		throw new Error('Independent budget was lost.');
	first.dispose();
	first.dispose();
	rejects(() => first.resize(requests[0][1]), 'disposed', 'instance');
	same(second.resize(requests[0][1]), saved[0][1], 'Other pool survives disposal.');
	for (const [result, copy] of saved)
		same(result, copy, 'Caller output survives work and disposal.');
	return { methods: requests.length, callbackFailures: requests.length * 2 };
}
