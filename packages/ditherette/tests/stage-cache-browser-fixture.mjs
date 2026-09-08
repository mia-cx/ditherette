/** Check public ownership and composition, not private cache hits, through an installed package. */
export async function stageCacheBrowserChecks({
	wasmUrl,
	vectors,
	moduleUrl = '/node_modules/ditherette/dist/index.js'
}) {
	const { createDitherette, DitheretteError } = await import(moduleUrl);
	const wasm = await (await fetch(wasmUrl)).arrayBuffer();
	const processor = await createDitherette({ wasm });
	const other = await createDitherette({ wasm });
	const same = (actual, expected, label) => {
		if (JSON.stringify(actual) !== JSON.stringify(expected)) throw new Error(label);
	};
	const rejects = (run, code, path) => {
		try {
			run();
		} catch (error) {
			if (error instanceof DitheretteError && error.code === code && error.path === path) return;
			throw error;
		}
		throw new Error(`Expected ${code} at ${path}`);
	};
	const color = (...rgb) => ({ kind: 'color', rgb });
	const palette = [color(0, 0, 0), color(255, 255, 255), color(255, 0, 0), { kind: 'transparent' }];
	const backing = new Uint8Array([99, ...vectors.source.data, 98]);
	const source = { ...vectors.source, data: backing.subarray(1, backing.length - 1) };
	const alpha = { mode: 'preserve', threshold: 127.9999999 };
	const matching = 'srgb-euclidean';
	const output = { width: 3, height: 3, resize: { algorithm: 'nearest', anchor: 'center' } };
	const perturb = vectors.cases[0].policy;
	const dither = { family: 'separable', perturb };
	const quantize = { version: 1, source, palette, alpha, matching };
	const recipe = { version: 1, output, alpha, match: matching, dither };
	const requests = [
		['resize', { version: 1, source, output }],
		['perturb', { version: 1, source, perturb }],
		['quantize', quantize],
		['ditherAndQuantize', { ...quantize, dither }],
		['process', { source, palette, recipe }]
	];
	let compositions = 0;
	try {
		for (const [method, request] of requests) {
			const first = processor[method](request);
			const saved = structuredClone(first);
			// Keep the same caller view and backing allocation throughout all three identities.
			source.data.fill(255);
			const changed = processor[method](request);
			same(
				changed,
				other[method](request),
				`${method}: changed input matches independent instance`
			);
			if (JSON.stringify(changed) === JSON.stringify(saved))
				throw new Error(`${method}: stale input`);
			same(first, saved, `${method}: caller mutation leaves previous output intact`);
			source.data.set(vectors.source.data);
			same(processor[method](request), saved, `${method}: restored input`);

			if (first.data) first.data.fill(0);
			else {
				first.indices.fill(255);
				first.palette.rgba.fill(17);
				first.palette.transparentIndex = null;
				first.warnings.push({ code: 'caller-only', message: 'mutated by caller' });
			}
			first.width = 99;
			const repeated = processor[method](request);
			same(repeated, saved, `${method}: output mutation cannot poison retained results`);
			const firstBytes = first.data ?? first.indices;
			const nextBytes = repeated.data ?? repeated.indices;
			if (firstBytes.buffer === nextBytes.buffer || nextBytes.buffer === source.data.buffer)
				throw new Error(`${method}: borrowed output bytes`);
			if (first.palette && first.palette.rgba.buffer === repeated.palette.rgba.buffer)
				throw new Error(`${method}: borrowed palette bytes`);
		}
		same(
			[...backing],
			[99, ...vectors.source.data, 98],
			'source view and guard bytes stay caller-owned'
		);
		same(
			[...processor.perturb(requests[1][1]).data],
			vectors.cases[0].rgba,
			'frozen field boundary'
		);

		const families = [
			{ family: 'none' },
			dither,
			{
				family: 'diffusion',
				kernel: 'floyd-steinberg',
				feedback: 'srgb-bytes',
				strength: 0.7,
				serpentine: true,
				placement: { mode: 'everywhere' }
			},
			{ family: 'yliluoma', size: '2', placement: { mode: 'everywhere' } }
		];
		for (const family of families) {
			const request = { source, palette, recipe: { ...recipe, dither: family } };
			const resized = processor.resize(requests[0][1]);
			const staged = { ...quantize, source: resized, dither: family };
			const indexed = processor.ditherAndQuantize(staged);
			same(processor.process(request), indexed, `${family.family}: resize then Process`);
			same(other.process(request), indexed, `${family.family}: independent complete call`);
			same(other.resize(requests[0][1]), resized, `${family.family}: Process then resize`);
			same(other.ditherAndQuantize(staged), indexed, `${family.family}: Process then fused call`);
			if (family.family === 'separable') {
				const rgba = processor.perturb({ version: 1, source: resized, perturb });
				same(processor.quantize({ ...quantize, source: rgba }), indexed, 'perturb then quantize');
				compositions++;
			}
			if (family.family === 'none') {
				same(processor.quantize({ ...quantize, source: resized }), indexed, 'direct and no-dither');
				compositions++;
			}
			compositions += 4;
		}

		const precise = {
			...quantize,
			source: { width: 2, height: 1, data: new Uint8Array([255, 0, 0, 128, 17, 31, 53, 0]) },
			palette: [color(255, 0, 0), color(0, 0, 0), { kind: 'transparent' }],
			alpha: { ...alpha }
		};
		same([...processor.quantize(precise).indices], [0, 2], 'threshold below 128');
		precise.alpha.threshold = 128.0000001;
		same([...processor.quantize(precise).indices], [2, 2], 'full f64 threshold above 128');
		precise.alpha.threshold = 127.9999999;
		precise.palette.reverse();
		const reordered = processor.quantize(precise);
		same([...reordered.indices], [2, 0], 'palette order participates in identity');
		same(reordered, other.quantize(precise), 'complete reordered metadata');
		precise.alpha.threshold = -0;
		const zero = processor.quantize(precise);
		precise.alpha.threshold = 0;
		same(processor.quantize(precise), zero, 'signed-zero normalization');

		const truncated = { ...quantize, palette: Array.from({ length: 257 }, () => color(0, 0, 0)) };
		const warning = processor.quantize(truncated);
		const warningSaved = structuredClone(warning);
		if (!warning.warnings.some(({ code }) => code === 'palette-truncated'))
			throw new Error('missing warning');
		warning.warnings[0].message = 'caller edit';
		same(processor.quantize(truncated), warningSaved, 'warning objects are durable copies');

		// Use an unseen source for multiple pending Process stages, then reject the final palette copy.
		const failedSource = { ...source, data: new Uint8Array(source.data) };
		failedSource.data[0] ^= 127;
		const failedRequest = { source: failedSource, palette, recipe };
		const expected = other.process(failedRequest);
		const set = Uint8Array.prototype.set;
		let copies = 0;
		Uint8Array.prototype.set = function (...args) {
			if (++copies === 3) throw new RangeError('final result copy');
			return Reflect.apply(set, this, args);
		};
		try {
			rejects(() => processor.process(failedRequest), 'wasm-memory-unavailable', 'output');
		} finally {
			Uint8Array.prototype.set = set;
		}
		same(processor.process(failedRequest), expected, 'failed final-copy recovery');
		const durable = processor.process(failedRequest);
		const durableSaved = structuredClone(durable);
		processor.dispose();
		processor.dispose();
		rejects(() => processor.process(failedRequest), 'disposed', 'instance');
		same(other.process(failedRequest), expected, 'disposing one instance leaves the other usable');
		same(durable, durableSaved, 'disposal preserves earlier returned storage');
		return { methods: 5, compositions, settingsChanges: 4, caughtCopies: 1, isolatedInstances: 2 };
	} finally {
		processor.dispose();
		other.dispose();
	}
}
