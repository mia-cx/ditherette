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

/** A rectangular ordered tile, named width by height. */
function orderedTile(tile: '3x1' | '4x1' | '4x2' | '5x3', short: string) {
	return {
		id: `ordered-${tile}`,
		label: `Ordered ${tile.replace('x', '×')}`,
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: `threshold.ordered.tile-${tile}`,
		short
	} as const satisfies DitherOption;
}

/** An error-diffusion kernel; the ID is the package kernel tag. */
function kernel<const Id extends DitherId>(id: Id, label: string, short: string) {
	return {
		id,
		label,
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: `error-diffusion.kernel.${id}`,
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
	orderedTile(
		'3x1',
		'A three-pixel tile, one row tall, so every row repeats the same thresholds. Texture forms vertical lines with only three levels: coarse and deliberately stripy.'
	),
	orderedTile(
		'4x1',
		'A four-pixel tile, one row tall. The same vertical-line texture as 3×1, with one more step between tones.'
	),
	orderedTile(
		'4x2',
		'Eight threshold levels in a tile twice as wide as it is tall. The texture sits between the 4×1 lines and a square Bayer 4×4.'
	),
	orderedTile(
		'5x3',
		'Fifteen threshold levels in a hand-designed tile by Joel Yliluoma. Its odd size avoids the power-of-two grid look of Bayer matrices.'
	),
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
	kernel(
		'jarvis-judice-ninke',
		'Jarvis–Judice–Ninke',
		'Spreads error over twelve pixels across three rows. Gradients come out very smooth with few worms, but fine detail softens.'
	),
	kernel(
		'stucki',
		'Stucki',
		'The same twelve-pixel reach as Jarvis–Judice–Ninke, with more weight on the nearest pixels. Just as smooth, with crisper edges.'
	),
	kernel(
		'burkes',
		'Burkes',
		'A two-row cut of Stucki that sends error to seven pixels. Close to the Stucki look while touching one row fewer.'
	),
	kernel(
		'two-row-sierra',
		'Two-row Sierra',
		'The two-row variant of Sierra, also called Sierra2. Seven pixels share the error, a middle ground between Sierra and Sierra Lite.'
	),
	kernel(
		'fan',
		'Fan',
		'A four-pixel kernel by Zhigang Fan that reaches two pixels back on the next row. It breaks up the worms Floyd–Steinberg leaves in smooth areas.'
	),
	kernel(
		'shiau-fan',
		'Shiau–Fan',
		'A four-pixel kernel weighted in eighths, designed to reduce worm artefacts in highlights and shadows.'
	),
	kernel(
		'shiau-fan-2',
		'Shiau–Fan 2',
		'The five-pixel Shiau–Fan kernel, reaching three pixels back on the next row. Worms break up further for a more even texture.'
	),
	kernel(
		'simple-2d',
		'Simple 2D',
		'Sends half the error right and half straight down. The simplest kernel: fast, with a cruder texture than Floyd–Steinberg.'
	),
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
