/** Caller-visible controls shared by Node scalar and the real isolated threaded host. */
export function smoothGradientChecks(processor) {
	const check = (condition, message) => {
		if (!condition) throw new Error(message);
	};
	const placement = { mode: 'smooth-gradient-v1', minGradientSize: 24 };
	const perturb = {
		field: { algorithm: 'bayer', size: '16' },
		space: 'srgb',
		strength: 0.3,
		placement
	};
	const base = {
		version: 1,
		palette: [64, 96, 160, 192].map((v) => ({ kind: 'color', rgb: [v, v, v] })),
		alpha: { mode: 'premultiplied' },
		matching: 'oklab-euclidean'
	};
	const outputs = [];
	for (const kind of ['vertical', 'horizontal', 'diagonal', 'ramp', 'short', 'flat', 'mixed']) {
		const data = new Uint8Array(96 * 64 * 4);
		for (let y = 0; y < 64; y++)
			for (let x = 0; x < 96; x++) {
				const v =
					kind === 'vertical'
						? x < 48
							? 80
							: 176
						: kind === 'horizontal'
							? y < 32
								? 80
								: 176
							: kind === 'diagonal'
								? x + y < 80
									? 80
									: 176
								: kind === 'ramp'
									? 80 + x
									: kind === 'short'
										? 80 + Math.max(0, Math.min(8, x - 40)) * 12
										: kind === 'mixed' && x < 64
											? 80 + Math.floor(x * 1.5)
											: 80;
				data.set([v, v, v, 255], (y * 96 + x) * 4);
			}
		const source = { width: 96, height: 64, data };
		const plain = processor.quantize({ ...base, source });
		for (const dither of [
			{ family: 'separable', perturb },
			{
				family: 'diffusion',
				kernel: 'floyd-steinberg',
				feedback: 'srgb-bytes',
				strength: 0.3,
				serpentine: true,
				placement
			}
		]) {
			const result = processor.ditherAndQuantize({ ...base, source, dither });
			const changed = result.indices.reduce((n, v, i) => n + Number(v !== plain.indices[i]), 0);
			check(
				kind === 'ramp' || kind === 'mixed' ? changed > 0 : changed === 0,
				`${dither.family} ${kind}: ${changed}`
			);
			if (kind === 'mixed')
				for (let y = 0; y < 64; y++)
					for (let x = 63; x < 96; x++)
						check(
							result.indices[y * 96 + x] === plain.indices[y * 96 + x],
							'Diffusion crossed an excluded boundary.'
						);
			const large = { mode: 'smooth-gradient-v1', minGradientSize: 32768 };
			const excluded = processor.ditherAndQuantize({
				...base,
				source,
				dither:
					dither.family === 'separable'
						? { ...dither, perturb: { ...perturb, placement: large } }
						: { ...dither, placement: large }
			});
			check(
				String(excluded.indices) === String(plain.indices),
				'Minimum size was absent from processing identity.'
			);
			const processed = processor.process({
				source,
				palette: base.palette,
				recipe: {
					version: 1,
					output: { width: 96, height: 64, resize: { algorithm: 'nearest', anchor: 'center' } },
					alpha: base.alpha,
					match: base.matching,
					dither
				}
			});
			check(
				String(processed.indices) === String(result.indices),
				'Process changed direct placement semantics.'
			);
			outputs.push([...result.indices]);
		}
	}
	const wide = { width: 2048, height: 8, data: new Uint8Array(2048 * 8 * 4) };
	for (let y = 0; y < 8; y++)
		for (let x = 0; x < 2048; x++) {
			const v = 64 + Math.round((128 * x) / 2047);
			wide.data.set([v, v, v, 255], (y * 2048 + x) * 4);
		}
	const perturbed = processor.perturb({ version: 1, source: wide, perturb });
	let repeated = 0,
		changedRepeated = 0;
	for (let y = 0; y < 8; y++)
		for (let x = 1; x < 2047; x++) {
			const i = (y * 2048 + x) * 4;
			if (wide.data[i] === wide.data[i - 4] && wide.data[i] === wide.data[i + 4]) {
				repeated++;
				changedRepeated += Number(wide.data[i] !== perturbed.data[i]);
			}
		}
	check(
		changedRepeated / repeated > 0.9,
		`Repeated-byte plateau coverage ${changedRepeated}/${repeated}`
	);
	for (const value of [undefined, null, '24', 3, 24.5, 32769, Infinity, NaN]) {
		let error;
		try {
			processor.perturb({
				version: 1,
				source: wide,
				perturb: { ...perturb, placement: { ...placement, minGradientSize: value } }
			});
		} catch (caught) {
			error = caught;
		}
		check(
			error?.code === 'invalid-settings' && error.path === 'perturb.placement.minGradientSize',
			`Invalid spatial size ${value} accepted.`
		);
	}
	for (const mode of ['everywhere', 'adaptive', 'smooth-gradient-v1']) {
		const invalid =
			mode === 'smooth-gradient-v1'
				? { ...placement, radius: 6 }
				: mode === 'adaptive'
					? { mode, radius: 6, threshold: 12, softness: 12, minGradientSize: 24 }
					: { mode, minGradientSize: 24 };
		let rejected = false;
		try {
			processor.perturb({ version: 1, source: wide, perturb: { ...perturb, placement: invalid } });
		} catch (error) {
			rejected = error.code === 'invalid-settings';
		}
		check(rejected, `Cross-mode controls accepted for ${mode}.`);
	}
	return { outputs, repeated, changedRepeated };
}

/** Proves wrapper copies, analysis and retained output fit together at the actual public budget. */
export async function smoothGradientBudgetChecks(createAtLimit) {
	const placement = { mode: 'smooth-gradient-v1', minGradientSize: 24 };
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

/** Proves byte-quantized coloured ramps retain each selected algorithm's ordinary texture. */
export function coloredGradientChecks(processor) {
	const source = { width: 512, height: 128, data: new Uint8Array(512 * 128 * 4) };
	for (let y = 0; y < source.height; y++)
		for (let x = 0; x < source.width; x++) {
			const chatter = x % 40 === 0 ? 1 : 0;
			const rgb =
				x < 480
					? [
							2 * Math.round((50 - 0.004 * x - 0.003 * y) / 2),
							2 * Math.round((10 - 0.001 * x - 0.004 * y) / 2),
							2 * Math.round((72 - 0.03 * x - 0.04 * y) / 2) + chatter
						]
					: [180, 180, 180];
			source.data.set([...rgb, 255], (y * source.width + x) * 4);
		}
	const placement = { mode: 'smooth-gradient-v1', minGradientSize: 4 };
	const field = { algorithm: 'bayer', size: '16' };
	// A nonzero field exposes selection through the public RGBA boundary without a private hook.
	const marked = processor.perturb({
		version: 1,
		source,
		perturb: { field, space: 'srgb', strength: 8, placement }
	});
	const selected = Array.from({ length: source.width * source.height }, (_, i) =>
		[0, 1, 2].some((c) => marked.data[i * 4 + c] !== source.data[i * 4 + c])
	);
	const coverage = selected.filter(Boolean).length / (480 * source.height);
	if (coverage < 0.95) throw new Error(`Quantized coloured ramp has sparse bands: ${coverage}`);
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
	for (const matching of ['oklab-euclidean', 'srgb-euclidean', 'cielch-hue-arc']) {
		const base = { version: 1, source, palette, alpha: { mode: 'premultiplied' }, matching };
		const plain = processor.quantize(base);
		const policies = [
			'floyd-steinberg',
			'sierra',
			'sierra-lite',
			'atkinson',
			'jarvis-judice-ninke',
			'stucki',
			'burkes',
			'two-row-sierra',
			'fan',
			'shiau-fan',
			'shiau-fan-2',
			'simple-2d'
		].flatMap((kernel) =>
			['srgb-bytes', 'matching'].map((feedback) => ({
				family: 'diffusion',
				kernel,
				feedback,
				strength: 1,
				serpentine: true,
				placement: { mode: 'everywhere' }
			}))
		);
		policies.push({
			family: 'separable',
			perturb: {
				field,
				space: 'oklab',
				strength: 1,
				placement: { mode: 'everywhere' }
			}
		});
		for (const dither of policies) {
			const normal = processor.ditherAndQuantize({ ...base, dither });
			const masked = processor.ditherAndQuantize({
				...base,
				dither:
					dither.family === 'separable'
						? { ...dither, perturb: { ...dither.perturb, placement } }
						: { ...dither, placement }
			});
			for (let i = 0; i < selected.length; i++) {
				if (i % source.width >= 480 && selected[i]) throw new Error('Flat fill was selected.');
				const expected = selected[i] ? normal.indices[i] : plain.indices[i];
				if (masked.indices[i] !== expected)
					throw new Error(
						`${matching}/${dither.family}/${dither.kernel}/${dither.feedback}: texture changed at ${i}`
					);
			}
			outputs.push([...masked.indices]);
		}
	}
	return { coverage, outputs };
}
