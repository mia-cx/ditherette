/** Proves coherent 2D areas retain Bayer/Sierra texture at large sample sizes. */
export function areaChecks(processor) {
	const width = 128,
		height = 96;
	const sourceFor = (kind) => {
		const data = new Uint8Array(width * height * 4);
		for (let y = 0; y < height; y++)
			for (let x = 0; x < width; x++) {
				let rgb;
				if (kind === 'step') rgb = Array(3).fill(x < 64 ? 40 : 208);
				else if (kind === 'wide-ramp')
					rgb = [
						Math.round((255 * x) / 127),
						255 - Math.round((255 * x) / 127),
						80 + Math.floor(x / 8)
					];
				else
					rgb = [
						50 - 2 * Math.floor(x / 64),
						10,
						72 - 2 * Math.floor((x + y) / 24) + (x % 40 === 0 ? 1 : 0)
					];
				data.set([...rgb, 255], (y * width + x) * 4);
			}
		return { width, height, data };
	};
	const field = { algorithm: 'bayer', size: '16' };
	const mark = (source, placement) => {
		const output = processor.perturb({
			version: 1,
			source,
			perturb: { field, space: 'srgb', strength: 8, placement }
		});
		return Array.from({ length: width * height }, (_, i) =>
			[0, 1, 2].some((c) => source.data[i * 4 + c] !== output.data[i * 4 + c])
		);
	};
	const palette = [
		[0, 0, 0],
		[96, 0, 24],
		[107, 80, 246],
		[120, 12, 153],
		[51, 57, 65],
		[104, 70, 52],
		[210, 210, 210],
		[255, 255, 255],
		[12, 129, 110],
		[156, 132, 49]
	].map((rgb) => ({ kind: 'color', rgb }));
	const outputs = [];
	for (const kind of ['wide-ramp', 'quantized-ramp', 'step']) {
		const source = sourceFor(kind);
		for (const sampleSize of [4, 64, 114]) {
			const placement = { mode: 'smooth-area-v1', sampleSize, contrastThreshold: 0.12 };
			const mask = mark(source, placement);
			if (kind !== 'step' && !mask.every(Boolean))
				throw Error(`Sparse bands in ${kind} at sample${sampleSize}`);
			if (kind === 'step')
				for (let y = 0; y < height; y++)
					for (const x of [63, 64])
						if (mask[y * width + x]) throw Error('Strong edge entered a qualifying footprint.');
			let previous = Array(mask.length).fill(false);
			for (const contrastThreshold of [0, 0.01, 0.12, 0.5, 1]) {
				const current = mark(source, { ...placement, contrastThreshold });
				if (current.some((value, i) => previous[i] && !value))
					throw Error('Increasing tolerance removed pixels.');
				if (contrastThreshold === 1 && !current.every(Boolean))
					throw Error('Maximum tolerance differs from Everywhere.');
				previous = current;
			}
			for (const matching of ['oklab-euclidean', 'srgb-euclidean']) {
				const base = { version: 1, source, palette, alpha: { mode: 'premultiplied' }, matching };
				const plain = processor.quantize(base);
				for (const dither of [
					{
						family: 'separable',
						perturb: {
							field,
							space: matching === 'oklab-euclidean' ? 'oklab' : 'srgb',
							strength: 1,
							placement: { mode: 'everywhere' }
						}
					},
					...['matching', 'srgb-bytes'].map((feedback) => ({
						family: 'diffusion',
						kernel: 'sierra',
						feedback,
						strength: 1,
						serpentine: true,
						placement: { mode: 'everywhere' }
					}))
				]) {
					const ordinary = processor.ditherAndQuantize({ ...base, dither });
					const selected =
						dither.family === 'separable'
							? { ...dither, perturb: { ...dither.perturb, placement } }
							: { ...dither, placement };
					const actual = processor.ditherAndQuantize({ ...base, dither: selected });
					for (let i = 0; i < mask.length; i++)
						if (actual.indices[i] !== (mask[i] ? ordinary.indices[i] : plain.indices[i]))
							throw Error(`${kind}/${sampleSize}/${matching}/${dither.family}: texture at ${i}`);
					const composed = processor.process({
						source,
						palette,
						recipe: {
							version: 1,
							output: { width, height, resize: { algorithm: 'nearest', anchor: 'center' } },
							alpha: base.alpha,
							match: matching,
							dither: selected
						}
					});
					if (String(composed.indices) !== String(actual.indices))
						throw Error('Process changed area placement.');
					outputs.push([...actual.indices]);
				}
			}
		}
	}
	const source = sourceFor('wide-ramp');
	const valid = { mode: 'smooth-area-v1', sampleSize: 64, contrastThreshold: 0.12 };
	for (const [key, values] of [
		['sampleSize', [undefined, null, '64', 3, 64.5, 32769, Infinity, NaN]],
		['contrastThreshold', [undefined, null, '0.12', -0.001, 1.001, Infinity, NaN]]
	])
		for (const value of values) {
			let error;
			try {
				mark(source, { ...valid, [key]: value });
			} catch (caught) {
				error = caught;
			}
			if (error?.code !== 'invalid-settings' || error.path !== `perturb.placement.${key}`)
				throw Error(`Invalid ${key} accepted: ${value}`);
		}
	for (const placement of [
		{ mode: 'smooth-gradient-v1', minGradientSize: 24 },
		{ ...valid, radius: 4 },
		{ mode: 'adaptive', radius: 6, threshold: 12, softness: 12, sampleSize: 64 },
		{ mode: 'everywhere', contrastThreshold: 0.12 }
	]) {
		let rejected = false;
		try {
			mark(source, placement);
		} catch (error) {
			rejected = error.code === 'invalid-settings';
		}
		if (!rejected) throw Error('Cross-mode controls were accepted.');
	}
	return { outputs, completeFootprints: true };
}

/** Proves wrapper copies, analysis and retained output fit together at the actual public budget. */
export async function areaBudgetChecks(createAtLimit) {
	const placement = { mode: 'smooth-area-v1', sampleSize: 24, contrastThreshold: 0.12 };
	const perturb = {
		field: { algorithm: 'bayer', size: '16' },
		space: 'srgb',
		strength: 0.3,
		placement
	};
	const data = new Uint8Array(70 * 12 * 4);
	for (let y = 0; y < 12; y++)
		for (let x = 0; x < 70; x++) data.set([80 + x, 80 + x, 80 + x, 255], (y * 70 + x) * 4);
	const source = { width: 70, height: 12, data };
	const palette = [64, 96, 160, 192].map((v) => ({ kind: 'color', rgb: [v, v, v] }));
	const alpha = { mode: 'premultiplied' },
		matching = 'oklab-euclidean';
	const run = (processor, kind, onProgress) =>
		kind === 0
			? processor.perturb({ version: 1, source, perturb, onProgress }).data
			: kind === 1
				? processor.ditherAndQuantize({
						version: 1,
						source,
						palette,
						alpha,
						matching,
						dither: { family: 'separable', perturb },
						onProgress
					}).indices
				: processor.process({
						source,
						palette,
						recipe: {
							version: 1,
							output: { width: 35, height: 6, resize: { algorithm: 'nearest', anchor: 'center' } },
							alpha,
							match: matching,
							dither: {
								family: 'diffusion',
								kernel: 'floyd-steinberg',
								feedback: 'srgb-bytes',
								strength: 0.3,
								serpentine: true,
								placement
							}
						},
						onProgress
					}).indices;
	const minima = [];
	for (const kind of [0, 1, 2]) {
		const succeeds = async (limit) => {
			let processor;
			try {
				processor = await createAtLimit(limit);
				run(processor, kind);
				return true;
			} catch (error) {
				if (error.code !== 'memory-limit') throw error;
				return false;
			} finally {
				processor?.dispose();
			}
		};
		let low = 1,
			high = 1_000_000;
		if (!(await succeeds(high))) throw new Error('Gradient fixture exceeds qualification budget.');
		while (low < high) {
			const middle = Math.floor((low + high) / 2);
			if (await succeeds(middle)) high = middle;
			else low = middle + 1;
		}
		if (await succeeds(low - 1)) throw new Error('Under-budget gradient call succeeded.');
		let exact, roomy;
		try {
			exact = await createAtLimit(low);
			roomy = await createAtLimit(1_000_000);
			const expected = run(roomy, kind);
			let cancelled = false;
			try {
				run(exact, kind, (event) => {
					if (['perturb', 'dither-and-quantize'].includes(event.stage)) throw new Error('cancel');
				});
			} catch (error) {
				cancelled = error.code === 'callback';
			}
			if (!cancelled) throw new Error('Gradient call did not report cancellation.');
			if (String(run(exact, kind)) !== String(expected))
				throw new Error('Budget/cancellation changed gradient output.');
			minima.push(low);
		} finally {
			exact?.dispose();
			roomy?.dispose();
		}
	}
	return minima;
}
