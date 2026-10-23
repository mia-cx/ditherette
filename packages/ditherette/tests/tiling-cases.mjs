import { matchingModes } from '../dist/validation.js';
import { kernels, spaces, tiles } from '../dist/validation-fields.js';

/** Public-operation inputs shared by correctness checks and the prepared tuning sweep. */
export function tilingCases(source, palette) {
	const cases = [];
	const add = (id, method, request, schedule = 'independent', cpuOnly = false) =>
		cases.push({ id, method, request, schedule, cpuOnly });
	const base = {
		version: 1,
		source,
		palette,
		alpha: { mode: 'preserve', threshold: 128 },
		matching: 'oklab-euclidean'
	};
	const placements = [
		{ mode: 'everywhere' },
		{ mode: 'smooth-area-v1', sampleSize: 20, contrastThreshold: 0.12 }
	];
	const filters = ['nearest', 'area', 'bilinear', 'trilinear', 'bicubic', 'lanczos2', 'lanczos3'];
	const output = (algorithm) => ({
		width: Math.max(1, Math.floor(source.width * 0.61)),
		height: Math.max(1, Math.floor(source.height * 0.73)),
		resize: {
			algorithm,
			...(algorithm === 'area' ? {} : { anchor: 'bottom-right' }),
			...(['bicubic', 'lanczos2', 'lanczos3'].includes(algorithm) ? { support: 'scale-aware' } : {})
		}
	});
	for (const algorithm of filters)
		add(`resize/${algorithm}`, 'resize', { version: 1, source, output: output(algorithm) });
	for (const matching of matchingModes) add(`match/${matching}`, 'quantize', { ...base, matching });
	const fields = [
		...['2', '4', '8', '16'].map((size) => ({ algorithm: 'bayer', size })),
		...tiles.map((tile) => ({ algorithm: 'ordered', tile })),
		{ algorithm: 'random', seed: 41234 },
		{ algorithm: 'blue-noise' }
	];
	for (const placement of placements) {
		for (const [index, field] of fields.entries()) {
			const perturb = { field, space: 'oklab', strength: 0.3, placement };
			add(`field/${index}/${placement.mode}`, 'perturb', { version: 1, source, perturb });
			add(`indexed/${index}/${placement.mode}`, 'ditherAndQuantize', {
				...base,
				dither: { family: 'separable', perturb }
			});
		}
		for (const space of spaces)
			add(`space/${space}/${placement.mode}`, 'perturb', {
				version: 1,
				source,
				perturb: { field: { algorithm: 'bayer', size: '16' }, space, strength: 0.3, placement }
			});
		for (const kernel of kernels)
			for (const feedback of ['srgb-bytes', 'matching'])
				for (const serpentine of [false, true])
					add(
						`diffusion/${kernel}/${feedback}/${serpentine}/${placement.mode}`,
						'ditherAndQuantize',
						{
							...base,
							dither: { family: 'diffusion', kernel, feedback, serpentine, strength: 1, placement }
						},
						'dependent'
					);
		for (const size of ['2', '4', '8', '16'])
			add(`mixture/${size}/${placement.mode}`, 'ditherAndQuantize', {
				...base,
				dither: { family: 'yliluoma', size, placement }
			});
		add(`process/${placement.mode}`, 'process', {
			source,
			palette,
			recipe: {
				version: 2,
				effects: [{ effect: 'exposure', enabled: true, stops: 0.33 }],
				output: output('lanczos3'),
				alpha: base.alpha,
				match: base.matching,
				dither: {
					family: 'separable',
					perturb: {
						space: 'oklab',
						field: { algorithm: 'bayer', size: '16' },
						strength: 0.3,
						placement
					}
				}
			}
		});
	}
	const mask = [
		{
			x: { model: 'srgb', channel: 'red' },
			points: [
				[0, 0],
				[1, 1]
			]
		}
	];
	const profiles = [
		[{ effect: 'exposure', enabled: true, stops: 0.33 }],
		[{ effect: 'hue-saturation', enabled: true, hue: 30, saturation: 0.2, lightness: -0.1 }],
		[
			{
				effect: 'curves',
				enabled: true,
				mask,
				curves: [
					{
						kind: 'adjust',
						x: { model: 'srgb', channel: 'green' },
						y: { model: 'oklab', channel: 'a' },
						points: [
							[0, 0.4],
							[1, 0.6]
						]
					}
				]
			}
		],
		[
			{
				effect: 'palette-fit',
				enabled: true,
				strength: 0.6,
				space: 'oklab',
				look: 'natural',
				curves: null
			}
		]
	];
	for (const [index, effects] of profiles.entries()) {
		const request = { version: 1, source, effects, context: { palette, space: 'oklab' } };
		add(`effects/${index}`, 'applyEffects', request, 'independent', index === 3);
		add(`mask/${index}`, 'effectMask', { ...request, mask }, 'independent', index === 3);
	}
	const analysis = {
		version: 1,
		source,
		effects: profiles[1],
		context: { palette, space: 'oklab' }
	};
	add('analysis/recolour', 'analyzeRecolour', analysis, 'global', true);
	for (const method of ['analyzePaletteFit', 'measurePaletteFit'])
		add(
			`analysis/${method}`,
			method,
			{ ...analysis, look: 'natural', space: 'oklab' },
			'global',
			true
		);
	return cases;
}
