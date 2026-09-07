/** Executed by Playwright in an ordinary non-isolated browser page using the installed tarball. */
export async function browserChecks(wasmUrl) {
	const { createDitherette, DitheretteError } = await import('ditherette');
	const equal = (actual, expected, label) => {
		if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(label);
	};
	const error = async (run, code, path) => {
		try {
			await run();
		} catch (failure) {
			if (failure instanceof DitheretteError && failure.code === code && failure.path === path)
				return;
			throw failure;
		}
		throw new Error(`Expected ${code} at ${path}`);
	};
	const request = (anchor = 'center') => ({
		version: 1,
		source: {
			width: 4,
			height: 4,
			data: new Uint8Array(Array.from({ length: 16 }, (_, i) => [i, 100, 255 - i, i * 17]).flat())
		},
		output: { width: 3, height: 3, resize: { algorithm: 'nearest', anchor } }
	});
	if (crossOriginIsolated) throw new Error('Scalar fixture must not require isolation headers.');
	const processor = await createDitherette();
	const anchors = [
		'top-left',
		'top',
		'top-right',
		'left',
		'center',
		'right',
		'bottom-left',
		'bottom',
		'bottom-right'
	];
	// Independently known 4→3 sample positions for start, center, and end alignment.
	const positions = [
		[0, 1, 2],
		[0, 2, 3],
		[1, 2, 3]
	];
	for (const [index, anchor] of anchors.entries()) {
		const value = request(anchor);
		const original = Array.from(value.source.data);
		const output = processor.resize(value);
		const expected = positions[Math.floor(index / 3)].flatMap((y) =>
			positions[index % 3].flatMap((x) => {
				const pixel = y * 4 + x;
				return [pixel, 100, 255 - pixel, pixel * 17];
			})
		);
		equal(Array.from(output.data), expected, `nearest ${anchor}`);
		equal(Array.from(value.source.data), original, 'source mutation');
	}
	for (const algorithm of ['area', 'bilinear']) {
		const backing = new Uint8Array([9, 200, 0, 100, 0, 0, 100, 200, 255, 9]);
		const value = {
			version: 1,
			source: { width: 2, height: 1, data: backing.subarray(1, 9) },
			output: {
				width: 1,
				height: 1,
				resize: algorithm === 'area' ? { algorithm } : { algorithm, anchor: 'center' }
			}
		};
		const average = processor.resize(value);
		equal(Array.from(average.data), [100, 50, 150, 128], `${algorithm} hidden-RGB average`);
		equal(
			Array.from(backing),
			[9, 200, 0, 100, 0, 0, 100, 200, 255, 9],
			`${algorithm} source ownership`
		);
		value.output.width = 2;
		const identity = processor.resize(value);
		equal(Array.from(identity.data), Array.from(value.source.data), `${algorithm} identity`);
		equal(Array.from(average.data), [100, 50, 150, 128], `${algorithm} result durability`);
		value.output.resize.support = 'fixed';
		await error(() => processor.resize(value), 'invalid-settings', 'output.resize.support');
	}
	const saved = processor.resize(request());
	const savedBytes = Array.from(saved.data);
	const larger = request();
	larger.output.width = 1024;
	larger.output.height = 1024;
	processor.resize(larger);
	processor.dispose();
	processor.dispose();
	equal(Array.from(saved.data), savedBytes, 'durability after reuse/disposal');
	await error(() => processor.resize(request()), 'disposed', 'instance');
	await error(() => createDitherette({ threads: 'required' }), 'capability', 'threads');
	await error(() => createDitherette({ memoryLimitBytes: 1 }), 'memory-limit', 'memoryLimitBytes');

	const response = await fetch(wasmUrl);
	const bytes = await response.clone().arrayBuffer();
	const compiled = await WebAssembly.compile(bytes);
	const padding = new Uint8Array(bytes.byteLength + 8);
	padding.set(new Uint8Array(bytes), 4);
	const inputs = [
		new URL(wasmUrl),
		wasmUrl,
		new Request(wasmUrl),
		response,
		response,
		compiled,
		padding.subarray(4, 4 + bytes.byteLength),
		new DataView(padding.buffer, 4, bytes.byteLength)
	];
	const instances = await Promise.all(
		inputs.map((wasm) => createDitherette({ wasm, threads: 'preferred' }))
	);
	if (response.bodyUsed) throw new Error('Custom Response was consumed.');
	instances[0].dispose();
	for (const other of instances.slice(1)) {
		equal(Array.from(other.resize(request()).data), savedBytes, 'custom input output');
		other.dispose();
	}
	const [failed, healthy] = await Promise.allSettled([
		createDitherette({ wasm: new Uint8Array([1, 2, 3]) }),
		createDitherette({ wasm: compiled })
	]);
	if (
		failed.status !== 'rejected' ||
		failed.reason.code !== 'initialization' ||
		healthy.status !== 'fulfilled'
	)
		throw new Error('Concurrent failure isolation');
	const active = healthy.value;
	const malformed = request();
	malformed.output.resize.anchor = { center: null };
	await error(() => active.resize(malformed), 'invalid-settings', 'output.resize.anchor');
	const detached = request();
	structuredClone(detached.source.data.buffer, { transfer: [detached.source.data.buffer] });
	await error(() => active.resize(detached), 'invalid-image', 'source.data');
	const progress = request();
	progress.onProgress = () => {
		throw new Error('Must not silently invoke unsupported progress');
	};
	await error(() => active.resize(progress), 'unsupported-operation', 'onProgress');
	equal(Array.from(active.resize(request()).data), savedBytes, 'recovery after errors');
	active.dispose();
	return { anchors: anchors.length, customInputs: inputs.length, scalarWithoutIsolation: true };
}
