/** Serialized into each real browser against the installed package, never local Rust bindings. */
export async function fieldBrowserChecks({ vectors, wasmUrl }) {
	const { createDitherette, DitheretteError } = await import('ditherette');
	const equal = (actual, expected, label) => {
		if (JSON.stringify(actual) !== JSON.stringify(expected))
			throw new Error(`${label}: ${JSON.stringify(actual)} != ${JSON.stringify(expected)}`);
	};
	const encoded = (image) => ({
		...image,
		indices: [...image.indices],
		palette: { ...image.palette, rgba: [...image.palette.rgba] }
	});
	const modes = [
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
	const bytes = await (await fetch(wasmUrl)).arrayBuffer();
	const module = await WebAssembly.compile(bytes);
	const processor = await createDitherette({ wasm: module });
	const other = await createDitherette({ wasm: bytes });
	const backing = new Uint8Array([99, ...vectors.source.data, 99]);
	const source = { ...vectors.source, data: backing.subarray(1, 17) };
	const palette = [
		{ kind: 'color', rgb: [0, 0, 0] },
		{ kind: 'color', rgb: [255, 0, 0] },
		{ kind: 'color', rgb: [255, 255, 255] },
		{ kind: 'transparent' }
	];
	let previous;
	let compositions = 0;
	try {
		for (const vector of vectors.cases) {
			const vectorSource = vector.source
				? { ...vector.source, data: new Uint8Array(vector.source.data) }
				: source;
			const rgba = processor.perturb({ version: 1, source: vectorSource, perturb: vector.policy });
			equal([...rgba.data], vector.rgba, JSON.stringify(vector.policy));
			if (rgba.data.buffer === source.data.buffer || rgba.data.buffer === previous?.data.buffer)
				throw new Error('borrowed result');
			for (const matching of modes) {
				const value = {
					version: 1,
					source: vectorSource,
					palette,
					matching,
					alpha: { mode: 'preserve', threshold: 127.9999999 }
				};
				const fused = processor.ditherAndQuantize({
					...value,
					dither: { family: 'separable', perturb: vector.policy }
				});
				equal(
					encoded(fused),
					encoded(processor.quantize({ ...value, source: rgba })),
					'RGBA8 composition'
				);
				compositions++;
			}
			previous = rgba;
		}
		equal([...backing], [99, ...vectors.source.data, 99], 'caller preservation');
		const value = { version: 1, source, perturb: vectors.cases[0].policy };
		for (const [method, request, copies] of [
			['perturb', value, 2],
			[
				'ditherAndQuantize',
				{
					version: 1,
					source,
					palette,
					matching: 'oklch-hue-arc',
					alpha: { mode: 'preserve', threshold: 127 },
					dither: { family: 'separable', perturb: value.perturb }
				},
				3
			]
		]) {
			const expected = processor[method](request);
			for (let failAt = 1; failAt <= copies; failAt++) {
				const set = Uint8Array.prototype.set;
				let copy = 0,
					failure;
				Uint8Array.prototype.set = function (...args) {
					if (++copy === failAt) throw new RangeError('field copy');
					return Reflect.apply(set, this, args);
				};
				try {
					processor[method](request);
				} catch (error) {
					failure = error;
				} finally {
					Uint8Array.prototype.set = set;
				}
				if (!(failure instanceof DitheretteError) || failure.code !== 'wasm-memory-unavailable')
					throw new Error('uncaught field failure');
				const actual = processor[method](request);
				equal(
					method === 'perturb' ? [...actual.data] : encoded(actual),
					method === 'perturb' ? [...expected.data] : encoded(expected),
					'recovery'
				);
			}
		}
		const retained = [...previous.data];
		processor.dispose();
		processor.dispose();
		equal([...previous.data], retained, 'durable result');
		equal([...other.perturb(value).data], vectors.cases[0].rgba, 'isolated bytes initialization');
		return { fields: vectors.cases.length, compositions, caughtFailures: 5 };
	} finally {
		processor.dispose();
		other.dispose();
	}
}
