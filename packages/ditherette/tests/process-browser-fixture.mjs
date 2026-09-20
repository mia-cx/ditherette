/** Exercise actual process/staged composition through an installed package, without timing. */
export async function processBrowserChecks({
	wasmUrl,
	wasm,
	moduleUrl = '/node_modules/ditherette/dist/index.js'
}) {
	const { createDitherette, DitheretteError } = await import(moduleUrl);
	const binary = wasm ?? (await (await fetch(wasmUrl)).arrayBuffer());
	const processor = await createDitherette({ wasm: binary });
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
	const backing = new Uint8Array([
		99, 255, 0, 0, 0, 17, 33, 71, 127, 0, 255, 0, 128, 0, 0, 255, 255, 98
	]);
	const original = [...backing];
	const source = { width: 2, height: 2, data: backing.subarray(1, 17) };
	const palette = [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'color', rgb: [255, 0, 0] },
		{ kind: 'color', rgb: [255, 0, 0] },
		{ kind: 'transparent' }
	];
	const matching = [
		'srgb-euclidean',
		'linear-rgb-euclidean',
		'oklab-euclidean',
		'cielab-euclidean',
		'ycbcr-euclidean',
		'srgb-compuphase',
		'srgb-rec601',
		'srgb-rec709',
		'oklch-euclidean',
		'oklch-circular-hue',
		'oklch-hue-arc',
		'cielab-ciede2000',
		'cielch-euclidean',
		'cielch-circular-hue',
		'cielch-hue-arc'
	];
	const filters = ['nearest', 'area', 'bilinear', 'bicubic', 'lanczos2', 'lanczos3', 'trilinear'];
	const spaces = ['srgb', 'linear-rgb', 'oklab', 'oklch', 'cielab', 'cielch', 'ycbcr'];
	const alphas = [
		{ mode: 'preserve', threshold: 127.9999999 },
		{ mode: 'premultiplied' },
		{ mode: 'matte', rgb: [3, 19, 47] }
	];
	const value = {
		source,
		palette,
		recipe: {
			version: 1,
			output: { width: 3, height: 4, resize: { algorithm: 'nearest', anchor: 'center' } },
			alpha: alphas[0],
			match: matching[0],
			dither: { family: 'none' }
		}
	};
	let compositions = 0;
	const compose = (request) => {
		const { recipe } = request;
		const resized = processor.resize({ version: 1, source: request.source, output: recipe.output });
		const expected = processor.ditherAndQuantize({
			version: 1,
			source: resized,
			palette: request.palette,
			alpha: recipe.alpha,
			matching: recipe.match,
			dither: recipe.dither
		});
		const actual = processor.process(request);
		same(actual, expected, `process composition ${JSON.stringify(recipe)}`);
		if (
			actual.indices.buffer === source.data.buffer ||
			actual.palette.rgba.buffer === actual.indices.buffer
		)
			throw new Error('Process output ownership');
		compositions++;
		return actual;
	};
	let retained;
	try {
		for (const [filterIndex, algorithm] of filters.entries()) {
			for (const [index, match] of matching.entries()) {
				const placement =
					index % 2
						? { mode: 'adaptive', radius: 1, threshold: 4, softness: 7 }
						: { mode: 'everywhere' };
				const field =
					index % 3 === 0
						? { algorithm: 'blue-noise' }
						: index % 3 === 1
							? { algorithm: 'random', seed: 0xffffffff }
							: { algorithm: 'bayer', size: ['2', '4', '8', '16'][index % 4] };
				const resize = {
					algorithm,
					...(algorithm === 'area'
						? {}
						: { anchor: ['top-left', 'center', 'bottom-right'][index % 3] }),
					...(['bicubic', 'lanczos2', 'lanczos3'].includes(algorithm)
						? { support: index % 2 ? 'fixed' : 'scale-aware' }
						: {})
				};
				for (const dither of [
					{ family: 'none' },
					{
						family: 'separable',
						perturb: {
							field,
							space: spaces[(index + filterIndex) % spaces.length],
							strength: 0.7,
							placement
						}
					},
					{
						family: 'diffusion',
						kernel: ['floyd-steinberg', 'sierra', 'sierra-lite', 'atkinson'][index % 4],
						feedback: index % 2 ? 'matching' : 'srgb-bytes',
						strength: 0.7,
						serpentine: index % 3 === 0,
						placement
					},
					{ family: 'yliluoma', size: ['2', '4', '8', '16'][index % 4], placement }
				])
					compose({
						...value,
						recipe: {
							...value.recipe,
							output: { width: 3, height: 4, resize },
							alpha: alphas[index % 3],
							match,
							dither
						}
					});
			}
		}
		const warningPalettes = [
			Array(257).fill({ kind: 'transparent' }),
			Array(257).fill({ kind: 'color', rgb: [20, 20, 20] })
		];
		for (const palette of warningPalettes) compose({ ...value, palette });
		retained = compose(value);
		const saved = JSON.stringify(retained);
		for (const [change, code, path] of [
			[{ ...value, version: 1 }, 'invalid-request', 'request.version'],
			[
				{ ...value, recipe: { ...value.recipe, version: 1.5 } },
				'invalid-request',
				'recipe.version'
			],
			[
				{ ...value, recipe: { ...value.recipe, matching: matching[0] } },
				'invalid-settings',
				'recipe.matching'
			],
			[
				{ ...value, recipe: { ...value.recipe, match: 'oklch-ciede2000' } },
				'invalid-settings',
				'recipe.match'
			],
			[
				{ ...value, recipe: { ...value.recipe, alpha: { mode: 'preserve', threshold: NaN } } },
				'invalid-settings',
				'recipe.alpha.threshold'
			],
			[
				{ ...value, recipe: { ...value.recipe, output: { ...value.recipe.output, width: 0 } } },
				'invalid-settings',
				'recipe.output.width'
			],
			[
				{
					...value,
					recipe: {
						...value.recipe,
						dither: { family: 'yliluoma', size: 4, placement: { mode: 'everywhere' } }
					}
				},
				'invalid-settings',
				'recipe.dither.size'
			],
			[{ ...value, palette: [] }, 'invalid-palette', 'palette'],
			[{ ...value, onProgress: 1 }, 'invalid-settings', 'onProgress']
		])
			rejects(() => processor.process(change), code, path);
		const reads = new Map();
		const once = (object, prefix) => {
			const clone = {};
			for (const [key, entry] of Object.entries(object))
				Object.defineProperty(clone, key, {
					enumerable: true,
					get() {
						const path = `${prefix}.${key}`;
						reads.set(path, (reads.get(path) ?? 0) + 1);
						if (reads.get(path) !== 1) throw new Error(`Read twice ${path}`);
						return entry;
					}
				});
			return clone;
		};
		const recipe = once(
			{
				...value.recipe,
				output: once(
					{ ...value.recipe.output, resize: once(value.recipe.output.resize, 'resize') },
					'output'
				),
				alpha: once(value.recipe.alpha, 'alpha'),
				dither: once(value.recipe.dither, 'dither')
			},
			'recipe'
		);
		const caller = once({ ...value, source: once(source, 'source'), recipe }, 'request');
		processor.process(caller);
		const recursive = {
			...value,
			get recipe() {
				rejects(() => processor.process(value), 'reentrant-call', 'instance');
				rejects(
					() => processor.resize({ version: 1, source, output: value.recipe.output }),
					'reentrant-call',
					'instance'
				);
				rejects(() => processor.dispose(), 'reentrant-call', 'instance');
				return value.recipe;
			}
		};
		processor.process(recursive);
		same([...backing], original, 'caller bytes remain unchanged');
		processor.process({
			...value,
			recipe: { ...value.recipe, output: { ...value.recipe.output, width: 64, height: 64 } }
		});
		if (JSON.stringify(retained) !== saved) throw new Error('Later calls changed prior output');
	} finally {
		processor.dispose();
	}
	processor.dispose();
	rejects(() => processor.process(value), 'disposed', 'instance');
	if (!retained.indices.byteLength) throw new Error('Disposed result lost storage');
	const succeeds = async (memoryLimitBytes) => {
		let instance;
		try {
			instance = await createDitherette({ wasm: binary, memoryLimitBytes });
			instance.process(value);
			return true;
		} catch (error) {
			if (!(error instanceof DitheretteError) || error.code !== 'memory-limit') throw error;
			return false;
		} finally {
			instance?.dispose();
		}
	};
	let low = 1,
		high = 100_000;
	while (low < high) {
		const middle = Math.floor((low + high) / 2);
		if (await succeeds(middle)) high = middle;
		else low = middle + 1;
	}
	const under = await createDitherette({ wasm: binary, memoryLimitBytes: low - 1 });
	const exact = await createDitherette({ wasm: binary, memoryLimitBytes: low });
	const set = Uint8Array.prototype.set;
	let copies = 0;
	try {
		Uint8Array.prototype.set = function (...args) {
			copies++;
			return Reflect.apply(set, this, args);
		};
		rejects(() => under.process(value), 'memory-limit', 'memoryLimitBytes');
		if (copies > 1) throw new Error('Under-budget process copied beyond its input snapshot');
	} finally {
		Uint8Array.prototype.set = set;
	}
	try {
		const expected = exact.process(value);
		for (const failAt of [1, 2, 3]) {
			copies = 0;
			Uint8Array.prototype.set = function (...args) {
				if (++copies === failAt) {
					rejects(() => exact.process(value), 'reentrant-call', 'instance');
					rejects(() => exact.dispose(), 'reentrant-call', 'instance');
					throw new RangeError('process copy failure');
				}
				return Reflect.apply(set, this, args);
			};
			try {
				rejects(
					() => exact.process(value),
					'wasm-memory-unavailable',
					failAt === 1 ? 'source.data' : 'output'
				);
			} finally {
				Uint8Array.prototype.set = set;
			}
			same(exact.process(value), expected, 'copy failure recovery');
		}
		under.process({
			...value,
			recipe: { ...value.recipe, output: { ...value.recipe.output, width: 1, height: 1 } }
		});
	} finally {
		under.dispose();
		exact.dispose();
	}
	return {
		compositions,
		strictRequests: 9,
		caughtCopies: 3,
		exactBudget: true,
		scalarWithoutIsolation: true
	};
}
