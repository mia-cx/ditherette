import type { ColorSpaceId } from '$lib/processing/types';

export type ColorSpaceOption = {
	id: ColorSpaceId;
	label: string;
	short: string;
	math: string;
	latex: string;
};

export const COLOR_SPACES = [
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
] as const satisfies readonly ColorSpaceOption[];
