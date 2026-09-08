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
	let convolutionCases = 0;
	for (const algorithm of ['bicubic', 'lanczos2', 'lanczos3']) {
		for (const support of ['fixed', 'scale-aware']) {
			const value = {
				version: 1,
				source: {
					width: 3,
					height: 2,
					data: new Uint8Array(Array.from({ length: 6 }, () => [207, 31, 9, 0]).flat())
				},
				output: { width: 5, height: 3, resize: { algorithm, anchor: 'center', support } }
			};
			for (const anchor of anchors) {
				value.output.resize.anchor = anchor;
				const output = processor.resize(value);
				equal(
					Array.from(output.data),
					Array.from({ length: 15 }, () => [207, 31, 9, 0]).flat(),
					`${algorithm} ${support} ${anchor}`
				);
				convolutionCases++;
			}
			value.output.resize.support = { fixed: null };
			await error(() => processor.resize(value), 'invalid-settings', 'output.resize.support');
			value.output.resize.support = support;
			const bounded = await createDitherette({ memoryLimitBytes: 4000 });
			const oversized = {
				...value,
				source: { width: 101, height: 100, data: new Uint8Array(101 * 100 * 4) }
			};
			await error(() => bounded.resize(oversized), 'memory-limit', 'memoryLimitBytes');
			value.source = {
				width: 2,
				height: 1,
				data: new Uint8Array([200, 0, 100, 0, 0, 100, 200, 255])
			};
			value.output = { width: 1, height: 1, resize: { algorithm, anchor: 'center', support } };
			const average = bounded.resize(value);
			// Fixed fixture from the unchanged landed Wasm path, including its half-byte tie arithmetic.
			const expected = [
				100,
				50,
				150,
				algorithm === 'lanczos3' && support === 'scale-aware' ? 127 : 128
			];
			equal(Array.from(average.data), expected, `${algorithm} ${support} alpha`);
			bounded.dispose();
			equal(Array.from(average.data), expected, 'convolution result durability');
		}
	}
	const quantizeRequest = {
		version: 1,
		source: { width: 2, height: 1, data: new Uint8Array([255, 0, 0, 128, 17, 31, 53, 0]) },
		palette: [
			{ kind: 'color', rgb: [255, 0, 0] },
			{ kind: 'color', rgb: [0, 0, 0] },
			{ kind: 'transparent' }
		],
		alpha: { mode: 'preserve', threshold: 127.9999999 },
		matching: 'srgb-euclidean'
	};
	let quantizeCases = 0;
	let savedIndexed;
	for (const matching of [
		'srgb-euclidean',
		'linear-rgb-euclidean',
		'oklab-euclidean',
		'cielab-euclidean',
		'ycbcr-euclidean'
	]) {
		const indexed = processor.quantize({ ...quantizeRequest, matching });
		equal([...indexed.indices], [0, 2], `${matching} indices`);
		equal(
			[...indexed.palette.rgba],
			[255, 0, 0, 255, 0, 0, 0, 255, 0, 0, 0, 0],
			`${matching} palette`
		);
		equal(indexed.palette.transparentIndex, 2, 'transparent index');
		equal(indexed.warnings, [], 'quantize warnings');
		savedIndexed = indexed;
		quantizeCases++;
	}
	const transparent = processor.quantize({
		...quantizeRequest,
		palette: Array.from({ length: 257 }, () => ({ kind: 'transparent' }))
	});
	equal(
		transparent.warnings,
		[
			{
				code: 'palette-truncated',
				message: 'Palette was truncated to 256 entries for indexed PNG export.'
			},
			{
				code: 'transparent-only',
				message: 'Only Transparent is enabled; every output pixel is transparent.'
			}
		],
		'authoritative indexed warnings'
	);
	await error(
		() => processor.quantize({ ...quantizeRequest, matching: 'oklch-hue-arc' }),
		'unsupported-operation',
		'matching'
	);
	await error(
		() =>
			processor.quantize({ ...quantizeRequest, alpha: { mode: 'premultiplied', threshold: 0 } }),
		'invalid-settings',
		'alpha.threshold'
	);
	const set = Uint8Array.prototype.set;
	let copies = 0;
	try {
		Uint8Array.prototype.set = function (...args) {
			if (++copies === 3) throw new RangeError('indexed palette copy failure');
			return Reflect.apply(set, this, args);
		};
		await error(() => processor.quantize(quantizeRequest), 'wasm-memory-unavailable', 'output');
	} finally {
		Uint8Array.prototype.set = set;
	}
	equal([...processor.quantize(quantizeRequest).indices], [0, 2], 'quantize recovery');
	const saved = processor.resize(request());
	const savedBytes = Array.from(saved.data);
	const larger = request();
	larger.output.width = 1024;
	larger.output.height = 1024;
	processor.resize(larger);
	processor.dispose();
	processor.dispose();
	equal(Array.from(saved.data), savedBytes, 'durability after reuse/disposal');
	equal([...savedIndexed.indices], [0, 2], 'indexed durability after resize growth/disposal');
	equal(savedIndexed.palette.rgba[0], 255, 'indexed palette durability');
	await error(() => processor.quantize(quantizeRequest), 'disposed', 'instance');
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
	return {
		anchors: anchors.length,
		convolutionCases,
		quantizeCases,
		customInputs: inputs.length,
		scalarWithoutIsolation: true
	};
}
