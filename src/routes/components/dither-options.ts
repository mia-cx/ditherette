import type { DitherId } from '$lib/processing/types';

export type DitherMethod = 'none' | 'threshold' | 'error-diffusion';
export type DitherField = 'none' | 'ordered' | 'noise' | 'kernel';

export type DitherOption = {
	id: DitherId;
	label: string;
	family: 'none' | 'ordered' | 'error-diffusion' | 'noise';
	method: DitherMethod;
	field: DitherField;
	sku: string;
	short: string;
	math: string;
	latex: string;
};

export const DITHER_ALGORITHMS = [
	{
		id: 'none',
		label: 'None',
		family: 'none',
		method: 'none',
		field: 'none',
		sku: 'direct.none',
		short:
			'Maps every pixel directly to its nearest palette color. No texture is added, so edges stay clean, but smooth gradients can collapse into harsh flat bands.',
		math: 'index = nearestPaletteColor(pixel)',
		latex: String.raw`q(p) = \operatorname*{arg\,min}_{c \in P} d(p, c)`
	},
	{
		id: 'bayer-2',
		label: 'Bayer 2×2',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-2',
		short:
			'Tiny ordered matrix with a loud checker texture. Good for chunky retro structure and previewing threshold strength; repetition is very obvious.',
		math: 'pixel += (Bayer₂[x mod 2,y mod 2] − 0.5) · strength',
		latex: String.raw`p' = p + s\,(B_2[x \bmod 2, y \bmod 2] - 0.5)`
	},
	{
		id: 'bayer-4',
		label: 'Bayer 4×4',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-4',
		short:
			'Balanced ordered matrix with visible but manageable texture. A practical default when you want crisp, deterministic dithering without diffusion trails.',
		math: 'pixel += (Bayer₄[x mod 4,y mod 4] − 0.5) · strength',
		latex: String.raw`p' = p + s\,(B_4[x \bmod 4, y \bmod 4] - 0.5)`
	},
	{
		id: 'bayer-8',
		label: 'Bayer 8×8',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-8',
		short:
			'Larger ordered matrix that spreads thresholds across more pixels. Gradients look smoother than 4×4, but the repeating tile is still part of the look.',
		math: 'pixel += (Bayer₈[x mod 8,y mod 8] − 0.5) · strength',
		latex: String.raw`p' = p + s\,(B_8[x \bmod 8, y \bmod 8] - 0.5)`
	},
	{
		id: 'bayer-16',
		label: 'Bayer 16×16',
		family: 'ordered',
		method: 'threshold',
		field: 'ordered',
		sku: 'threshold.ordered.bayer-16',
		short:
			'Fine ordered matrix with the least chunky Bayer texture. Best when you want deterministic dithering that reads smoother at normal viewing distance.',
		math: 'pixel += (Bayer₁₆[x mod 16,y mod 16] − 0.5) · strength',
		latex: String.raw`p' = p + s\,(B_{16}[x \bmod 16, y \bmod 16] - 0.5)`
	},
	{
		id: 'floyd-steinberg',
		label: 'Floyd–Steinberg',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.floyd-steinberg',
		short:
			'Classic error diffusion that pushes quantization error into four nearby future pixels. Gradients look organic, but texture can form worms and directional streaks.',
		math: 'error = pixel − quantized; diffuse {→7, ↙3, ↓5, ↘1}/16',
		latex: String.raw`e = p - q(p),\quad p_n \leftarrow p_n + w_n e`
	},
	{
		id: 'sierra',
		label: 'Sierra',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.sierra',
		short:
			'Spreads error across a wider three-row neighborhood. Softer and less speckled than Floyd–Steinberg, at the cost of a slightly blurrier texture.',
		math: 'error = pixel − quantized; diffuse Sierra weights /32 across three rows',
		latex: String.raw`e = p - q(p),\quad W = \frac{1}{32}\begin{bmatrix}0&0&0&5&3\\2&4&5&4&2\\0&2&3&2&0\end{bmatrix}`
	},
	{
		id: 'sierra-lite',
		label: 'Sierra Lite',
		family: 'error-diffusion',
		method: 'error-diffusion',
		field: 'kernel',
		sku: 'error-diffusion.kernel.sierra-lite',
		short:
			'Small diffusion kernel with strong directionality. Fast and punchy, useful when Floyd–Steinberg feels too busy but direct quantization is too banded.',
		math: 'error = pixel − quantized; diffuse {→2, ↙1, ↓1}/4',
		latex: String.raw`e = p - q(p),\quad W = \frac{1}{4}\{\rightarrow 2,\swarrow 1,\downarrow 1\}`
	},
	{
		id: 'random',
		label: 'Random',
		family: 'noise',
		method: 'threshold',
		field: 'noise',
		sku: 'threshold.noise.white',
		short:
			'Adds deterministic white-noise thresholding before palette matching. It avoids visible tiles, but the result is grainier and less structured than ordered matrices.',
		math: 'pixel += (mulberry32(seed,x,y) − 0.5) · strength',
		latex: String.raw`p' = p + s\,(n(seed,x,y) - 0.5)`
	}
] as const satisfies readonly DitherOption[];
