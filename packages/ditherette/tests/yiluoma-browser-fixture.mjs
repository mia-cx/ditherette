/** Runs unchanged in Node and real browsers against the package export. No timing collector runs. */
export async function yiluomaBrowserChecks({ vectors, wasm, wasmUrl }) {
	const { createDitherette, DitheretteError } = await import('ditherette');
	const module = wasm ?? (await WebAssembly.compile(await (await fetch(wasmUrl)).arrayBuffer()));
	const encode = (image) => ({
		...image,
		indices: [...image.indices],
		palette: { ...image.palette, rgba: [...image.palette.rgba] }
	});
	const equal = (actual, expected, label) => {
		const stable = (value) =>
			JSON.stringify(value, (_key, item) =>
				item && typeof item === 'object' && !Array.isArray(item)
					? Object.fromEntries(Object.entries(item).sort(([a], [b]) => a.localeCompare(b)))
					: item
			);
		if (stable(actual) !== stable(expected))
			throw new Error(`${label}: ${JSON.stringify(actual)} != ${JSON.stringify(expected)}`);
	};
	const request = () => ({
		...vectors.cases[0].request,
		dither: structuredClone(vectors.cases[0].request.dither),
		source: {
			...vectors.cases[0].request.source,
			data: new Uint8Array(vectors.cases[0].request.source.data)
		}
	});
	const expectError = (action, code, path) => {
		try {
			action();
		} catch (error) {
			if (error instanceof DitheretteError && error.code === code && error.path === path) return;
			throw error;
		}
		throw new Error(`Expected ${code} at ${path}`);
	};
	const processor = await createDitherette({ wasm: module });
	let previous;
	try {
		for (const vector of vectors.cases) {
			const bytes = vector.request.source.data;
			const backing = new Uint8Array([99, ...bytes, 99]);
			const source = { ...vector.request.source, data: backing.subarray(1, backing.length - 1) };
			const image = processor.ditherAndQuantize({ ...vector.request, source });
			equal(encode(image), vector.output, JSON.stringify(vector.request.dither));
			if (
				image.indices.buffer === source.data.buffer ||
				image.indices.buffer === previous?.indices.buffer ||
				image.palette.rgba.buffer === previous?.palette.rgba.buffer
			)
				throw new Error('borrowed output');
			equal([...backing], [99, ...bytes, 99], 'source ownership');
			previous = image;
		}
		for (const [change, path] of [
			[{ size: 2 }, 'size'],
			[{ size: '3' }, 'size'],
			[{ strength: 1 }, 'strength'],
			[{ perturb: {} }, 'perturb'],
			[{ placement: { mode: 'everywhere', radius: 1 } }, 'placement.radius'],
			[
				{ placement: { mode: 'adaptive', radius: 0, threshold: 0, softness: 0 } },
				'placement.radius'
			],
			[
				{ placement: { mode: 'adaptive', radius: 1, threshold: Infinity, softness: 0 } },
				'placement.threshold'
			],
			[
				{ placement: { mode: 'adaptive', radius: 1, threshold: 0, softness: 3.5e38 } },
				'placement.softness'
			]
		]) {
			expectError(
				() =>
					processor.ditherAndQuantize({ ...request(), dither: { ...request().dither, ...change } }),
				'invalid-settings',
				`dither.${path}`
			);
		}
		const recursive = request();
		let reads = 0;
		Object.defineProperty(recursive.dither, 'size', {
			get() {
				reads++;
				expectError(() => processor.ditherAndQuantize(request()), 'reentrant-call', 'instance');
				expectError(() => processor.dispose(), 'reentrant-call', 'instance');
				return '2';
			}
		});
		processor.ditherAndQuantize(recursive);
		equal(reads, 1, 'single control read');
		for (let failAt = 1; failAt <= 3; failAt++) {
			const changed = request();
			changed.source.data[0] ^= 1;
			processor.ditherAndQuantize(changed);
			const set = Uint8Array.prototype.set;
			let copies = 0;
			Uint8Array.prototype.set = function (...args) {
				if (++copies === failAt) throw new RangeError('copy');
				return Reflect.apply(set, this, args);
			};
			try {
				expectError(
					() => processor.ditherAndQuantize(request()),
					'wasm-memory-unavailable',
					failAt === 1 ? 'source.data' : 'output'
				);
			} finally {
				Uint8Array.prototype.set = set;
			}
			equal(
				encode(processor.ditherAndQuantize(request())),
				vectors.cases[0].output,
				'copy recovery'
			);
		}
		const retained = encode(previous);
		processor.dispose();
		equal(encode(previous), retained, 'durable disposal');
		expectError(() => processor.ditherAndQuantize(request()), 'disposed', 'instance');
	} finally {
		processor.dispose();
	}
	let low = 1,
		high = 100_000;
	const succeeds = async (limit) => {
		let instance;
		try {
			instance = await createDitherette({ wasm: module, memoryLimitBytes: limit });
			instance.ditherAndQuantize(request());
			return true;
		} catch (error) {
			if (!(error instanceof DitheretteError) || error.code !== 'memory-limit') throw error;
			return false;
		} finally {
			instance?.dispose();
		}
	};
	while (low < high) {
		const middle = Math.floor((low + high) / 2);
		if (await succeeds(middle)) high = middle;
		else low = middle + 1;
	}
	if (!(await succeeds(low))) throw new Error('exact budget failed');
	const under = await createDitherette({ wasm: module, memoryLimitBytes: low - 1 });
	const set = Uint8Array.prototype.set;
	let copies = 0;
	Uint8Array.prototype.set = function (...args) {
		copies++;
		return Reflect.apply(set, this, args);
	};
	try {
		expectError(() => under.ditherAndQuantize(request()), 'memory-limit', 'memoryLimitBytes');
	} finally {
		Uint8Array.prototype.set = set;
		under.dispose();
	}
	if (copies > 1) throw new Error('budget failure copied beyond its input snapshot');
	return { vectors: vectors.cases.length, caughtFailures: 3, strictControls: 8, exactBudget: true };
}
