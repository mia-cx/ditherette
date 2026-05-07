/**
 * Sample data used by skeleton layouts so panels feel real before
 * processing modules exist. Not a substitute for the actual Wplace
 * palette in `src/lib/palettes/wplace.ts` (future).
 */

import type { ColorSpaceId } from '$lib/processing/types';

export type SwatchKind = 'free' | 'premium' | 'transparent' | 'custom';

export type Swatch = {
	name: string;
	hex?: string;
	kind: SwatchKind;
	enabled: boolean;
};

export const SAMPLE_PALETTE: Swatch[] = [
	{ name: 'Black', hex: '#000000', kind: 'free', enabled: true },
	{ name: 'Dark Gray', hex: '#3C3C3C', kind: 'free', enabled: true },
	{ name: 'Gray', hex: '#787878', kind: 'free', enabled: true },
	{ name: 'Light Gray', hex: '#D2D2D2', kind: 'free', enabled: true },
	{ name: 'White', hex: '#FFFFFF', kind: 'free', enabled: true },
	{ name: 'Deep Red', hex: '#600018', kind: 'free', enabled: true },
	{ name: 'Red', hex: '#ED1C24', kind: 'free', enabled: true },
	{ name: 'Orange', hex: '#FF7F27', kind: 'free', enabled: true },
	{ name: 'Gold', hex: '#F6AA09', kind: 'free', enabled: true },
	{ name: 'Yellow', hex: '#F9DD3B', kind: 'free', enabled: true },
	{ name: 'Pale Yellow', hex: '#FFFABC', kind: 'free', enabled: true },
	{ name: 'Green', hex: '#0EB968', kind: 'free', enabled: true },
	{ name: 'Light Green', hex: '#13E67B', kind: 'free', enabled: true },
	{ name: 'Mint', hex: '#87FF5E', kind: 'free', enabled: true },
	{ name: 'Teal', hex: '#0C816E', kind: 'free', enabled: true },
	{ name: 'Cyan', hex: '#13E1BE', kind: 'free', enabled: true },
	{ name: 'Sky', hex: '#60F7F2', kind: 'free', enabled: true },
	{ name: 'Blue', hex: '#28509E', kind: 'free', enabled: true },
	{ name: 'Light Blue', hex: '#4093E4', kind: 'free', enabled: true },
	{ name: 'Indigo', hex: '#6B50F6', kind: 'free', enabled: true },
	{ name: 'Lavender', hex: '#99B1FB', kind: 'free', enabled: true },
	{ name: 'Purple', hex: '#780C99', kind: 'free', enabled: true },
	{ name: 'Magenta', hex: '#AA38B9', kind: 'free', enabled: true },
	{ name: 'Pink', hex: '#E09FF9', kind: 'free', enabled: true },
	{ name: 'Hot Pink', hex: '#CB007A', kind: 'free', enabled: true },
	{ name: 'Rose', hex: '#EC1F80', kind: 'free', enabled: true },
	{ name: 'Salmon', hex: '#F38DA9', kind: 'free', enabled: true },
	{ name: 'Brown', hex: '#684634', kind: 'free', enabled: true },
	{ name: 'Tan', hex: '#95682A', kind: 'free', enabled: true },
	{ name: 'Peach', hex: '#F8B277', kind: 'free', enabled: true },
	{ name: 'Silver', hex: '#AAAAAA', kind: 'premium', enabled: false },
	{ name: 'Crimson', hex: '#A50E1E', kind: 'premium', enabled: false },
	{ name: 'Coral', hex: '#FA8072', kind: 'premium', enabled: false },
	// Two custom entries so the grid swatches demonstrate the editable
	// state — edit/delete enabled, full-opacity icons on hover/focus.
	{ name: 'Studio Pink', hex: '#FF5C8A', kind: 'custom', enabled: true },
	{ name: 'Studio Teal', hex: '#1FB6A3', kind: 'custom', enabled: true },
	{ name: 'Transparent', kind: 'transparent', enabled: true }
];

export type ColorSpaceOption = {
	id: ColorSpaceId;
	label: string;
	short: string;
	math: string;
	latex: string;
};

export const COLOR_SPACES: ColorSpaceOption[] = [
	{
		id: 'oklab',
		label: 'OKLab',
		short:
			'Modern perceptual space tuned so equal numeric steps are closer to equal visible color changes. Usually the safest default for nearest-palette matching.',
		math: 'd² = (L₁−L₂)² + (a₁−a₂)² + (b₁−b₂)²',
		latex: String.raw`d^2 = \Delta L^2 + \Delta a^2 + \Delta b^2`
	},
	{
		id: 'srgb',
		label: 'sRGB',
		short:
			'Raw browser RGB channel distance. It is simple and predictable, but dark colors and saturated colors can be weighted unlike human vision.',
		math: 'd² = (R₁−R₂)² + (G₁−G₂)² + (B₁−B₂)²',
		latex: String.raw`d^2 = \Delta R^2 + \Delta G^2 + \Delta B^2`
	},
	{
		id: 'linear-rgb',
		label: 'Linear RGB',
		short:
			'Converts RGB into linear-light values before measuring distance. Better matches physical light mixing, but can pick surprising palette colors for pixel-art-style matching.',
		math: 'd² over linearized channels',
		latex: String.raw`d^2 = \Delta R_{lin}^2 + \Delta G_{lin}^2 + \Delta B_{lin}^2`
	},
	{
		id: 'weighted-rgb',
		label: 'Weighted RGB',
		short:
			'A fast RGB heuristic that changes red/blue weighting based on average red. Useful when OKLab feels too perceptual but plain RGB feels too naive.',
		math: '(2+r̄/256)·ΔR² + 4·ΔG² + (2+(255−r̄)/256)·ΔB²',
		latex: String.raw`d^2 = (2 + \bar r / 256)\Delta R^2 + 4\Delta G^2 + (2 + (255 - \bar r)/256)\Delta B^2`
	},
	{
		id: 'weighted-rgb-601',
		label: 'Weighted RGB · Rec.601',
		short:
			'Classic television luma weighting. Strongly favors green-channel accuracy, which can preserve brightness better than raw RGB for older image assumptions.',
		math: '0.299·ΔR² + 0.587·ΔG² + 0.114·ΔB²',
		latex: String.raw`d^2 = 0.299\Delta R^2 + 0.587\Delta G^2 + 0.114\Delta B^2`
	},
	{
		id: 'weighted-rgb-709',
		label: 'Weighted RGB · Rec.709',
		short:
			'Modern HDTV luma weighting. Even more green-heavy than Rec.601, often useful when perceived brightness should dominate hue fidelity.',
		math: '0.2126·ΔR² + 0.7152·ΔG² + 0.0722·ΔB²',
		latex: String.raw`d^2 = 0.2126\Delta R^2 + 0.7152\Delta G^2 + 0.0722\Delta B^2`
	},
	{
		id: 'cielab',
		label: 'CIELAB ΔE76',
		short:
			'Older perceptual color space using the ΔE76 distance formula. More human-oriented than RGB, though less uniform than OKLab in saturated regions.',
		math: 'ΔE*ab = √((ΔL)² + (Δa)² + (Δb)²)',
		latex: String.raw`\Delta E_{ab}^{*} = \sqrt{\Delta L^{*2} + \Delta a^{*2} + \Delta b^{*2}}`
	},
	{
		id: 'oklch',
		label: 'OKLCH',
		short:
			'OKLab expressed as lightness, chroma, and hue. Useful for reasoning about hue/chroma directly, with circular hue distance instead of flat a/b axes.',
		math: 'd uses ΔL, ΔC, and circular Δh',
		latex: String.raw`d^2 = \Delta L^2 + \Delta C^2 + w_h\,\Delta h_{circ}^2`
	}
];

export type DitherMethod = 'none' | 'threshold' | 'error-diffusion';
export type DitherField = 'none' | 'ordered' | 'noise' | 'kernel';

export type DitherOption = {
	id: string;
	label: string;
	family: 'none' | 'ordered' | 'error-diffusion' | 'noise';
	method: DitherMethod;
	field: DitherField;
	sku: string;
	short: string;
	math: string;
	latex: string;
};

export const DITHER_ALGORITHMS: DitherOption[] = [
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
];

export type ResizeOption = { id: string; label: string };

export const RESIZE_MODES: ResizeOption[] = [
	{ id: 'lanczos3', label: 'Lanczos3' },
	{ id: 'bilinear', label: 'Bilinear' },
	{ id: 'nearest', label: 'Nearest' },
	{ id: 'area', label: 'Area / Box' }
];

export const ALPHA_MODES: ResizeOption[] = [
	{ id: 'preserve', label: 'Preserve transparency' },
	{ id: 'premultiplied', label: 'Premultiplied' },
	{ id: 'matte', label: 'Matte' }
];

export const PLACEMENT_MODES: ResizeOption[] = [
	{ id: 'everywhere', label: 'Everywhere' },
	{ id: 'adaptive', label: 'Adaptive' }
];
