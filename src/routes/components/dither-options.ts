import type { DitherId } from '$lib/processing/types';

export type DitherMethod = 'none' | 'threshold' | 'error-diffusion' | 'mixing';
export type DitherField = 'none' | 'ordered' | 'noise' | 'kernel';

export type DitherOption = {
	id: DitherId;
	label: string;
	family: 'none' | 'ordered' | 'error-diffusion' | 'noise';
	method: DitherMethod;
	field: DitherField;
	sku: string;
	short: string;
};

/**
 * Yliluoma ordered mixing: each pixel picks the two-color palette mixture that best matches it,
 * and the Bayer threshold decides which of the two colors it shows.
 */
function yliluoma(size: 2 | 4 | 8 | 16, short: string) {
	return {
		id: `yliluoma-${size}`,
		label: `Yliluoma ${size}×${size}`,
		family: 'ordered',
		method: 'mixing',
		field: 'ordered',
		sku: `mixing.ordered.yliluoma-${size}`,
		short
	} as const satisfies DitherOption;
}

export const DITHER_ALGORITHMS = [
	{
		id: 'none',
		label: 'None',
		family: 'none',
		method: 'none',
		field: 'none',
		sku: 'direct.none',
		short:
			'Maps every pixel directly to its nearest palette colour. No texture is added, so edges stay clean, but smooth gradients can collapse into harsh flat bands.'
	},
	{
		id: 'bayer-2',
		label: 'Bayer 2×2',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-2',
		short:
			'Tiny ordered matrix with a loud checker texture. Good for chunky retro structure and previewing threshold strength; repetition is very obvious.'
	},
	{
		id: 'bayer-4',
		label: 'Bayer 4×4',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-4',
		short:
			'Balanced ordered matrix with visible but manageable texture. A practical default when you want crisp, deterministic dithering without diffusion trails.'
	},
	{
		id: 'bayer-8',
		label: 'Bayer 8×8',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-8',
		short:
			'Larger ordered matrix that spreads thresholds across more pixels. Gradients look smoother than 4×4, but the repeating tile is still part of the look.'
	},
	{
		id: 'bayer-16',
		label: 'Bayer 16×16',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-16',
		short:
			'Fine ordered matrix with the least chunky Bayer texture. Best when you want deterministic dithering that reads smoother at normal viewing distance.'
	},
	{
		id: 'floyd-steinberg',
		label: 'Floyd–Steinberg',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.floyd-steinberg',
		short:
			'Classic error diffusion that pushes quantisation error into four nearby future pixels. Gradients look organic, but texture can form worms and directional streaks.'
	},
	{
		id: 'sierra',
		label: 'Sierra',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.sierra',
		short:
			'Spreads error across a wider three-row neighbourhood. Softer and less speckled than Floyd–Steinberg, at the cost of a slightly blurrier texture.'
	},
	{
		id: 'sierra-lite',
		label: 'Sierra Lite',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.sierra-lite',
		short:
			'Small diffusion kernel with strong directionality. Fast and punchy, useful when Floyd–Steinberg feels too busy but direct quantisation is too banded.'
	},
	{
		id: 'atkinson',
		label: 'Atkinson',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.atkinson',
		short:
			'The classic Macintosh kernel. It passes on only three quarters of the error, so highlights and shadows settle into flat colour while midtones keep a crisp, high-contrast texture.'
	},
	{
		id: 'random',
		label: 'Random',
		family: 'noise',
		method: 'threshold',
		field: 'noise',
		sku: 'threshold.noise.white',
		short:
			'Adds deterministic white-noise thresholding before palette matching. It avoids visible tiles, but the result is grainier and less structured than ordered matrices.'
	},
	{
		id: 'blue-noise',
		label: 'Blue noise',
		family: 'noise',
		method: 'threshold',
		field: 'noise',
		sku: 'threshold.noise.blue',
		short:
			'Thresholds against a 32×32 void-and-cluster tile. The grain is fine and evenly spread, so gradients read smoother than white noise without the crosshatch of a Bayer matrix.'
	},
	yliluoma(
		2,
		'Coarse two-colour mixing with a visible 2×2 pattern. Flat areas become exact palette blends at a few mixing ratios.'
	),
	yliluoma(
		4,
		'Two-colour mixing over a 4×4 pattern. A good balance between smooth blends and a readable, retro texture.'
	),
	yliluoma(
		8,
		'Two-colour mixing over an 8×8 pattern. More mixing ratios make gradients smoother, at a slower search.'
	),
	yliluoma(
		16,
		'Two-colour mixing over a 16×16 pattern. The finest blends and the slowest search; best for small outputs.'
	)
] as const satisfies readonly DitherOption[];
