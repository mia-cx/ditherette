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
		id: 'oklch',
		label: 'OKLCH',
		short:
			'OKLab as lightness, chroma, and hue. Hue differences count in proportion to the smaller chroma, so near-grays match on lightness while saturated colors keep their hue.',
		math: 'd² = ΔL² + ΔC² + (min(C₁,C₂)·|Δh|)²',
		latex: String.raw`d^2 = \Delta L^2 + \Delta C^2 + (\min(C_1, C_2)\,\lvert\Delta h\rvert)^2`
	},
	{
		id: 'oklch-circular-hue',
		label: 'OKLCH · circular hue',
		short:
			'OKLCH with hue measured as a chord across the hue circle, scaled by both chromas. Close to OKLab distance, but hue and chroma stay separate terms.',
		math: 'd² = ΔL² + ΔC² + (2·√(C₁C₂)·sin(Δh/2))²',
		latex: String.raw`d^2 = \Delta L^2 + \Delta C^2 + \left(2\sqrt{C_1 C_2}\,\sin\tfrac{\Delta h}{2}\right)^2`
	},
	{
		id: 'oklch-euclidean',
		label: 'OKLCH · Euclidean',
		short:
			'Compares lightness, chroma, and hue angle as plain numbers. Hue does not wrap, so reds just either side of 0° count as far apart.',
		math: 'd² = ΔL² + ΔC² + Δh² (h in radians, unwrapped)',
		latex: String.raw`d^2 = \Delta L^2 + \Delta C^2 + \Delta h^2`
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
		id: 'cielab-ciede2000',
		label: 'CIELAB ΔE2000',
		short:
			'The CIE 2000 color difference. It corrects CIELAB in blues, grays, and saturated colors, which makes it the most careful perceptual match here, and the slowest.',
		math: 'ΔE₀₀ = √((ΔL′/S_L)² + (ΔC′/S_C)² + (ΔH′/S_H)² + R_T·(ΔC′/S_C)·(ΔH′/S_H))',
		latex: String.raw`\Delta E_{00} = \sqrt{\left(\tfrac{\Delta L'}{S_L}\right)^2 + \left(\tfrac{\Delta C'}{S_C}\right)^2 + \left(\tfrac{\Delta H'}{S_H}\right)^2 + R_T \tfrac{\Delta C'}{S_C}\tfrac{\Delta H'}{S_H}}`
	},
	{
		id: 'cielch',
		label: 'CIELCh',
		short:
			'CIELAB as lightness, chroma, and hue. Like OKLCH, hue differences count in proportion to the smaller chroma.',
		math: 'd² = ΔL*² + ΔC*² + (min(C₁,C₂)·|Δh|)²',
		latex: String.raw`d^2 = \Delta L^{*2} + \Delta C^{*2} + (\min(C_1, C_2)\,\lvert\Delta h\rvert)^2`
	},
	{
		id: 'cielch-circular-hue',
		label: 'CIELCh · circular hue',
		short: 'CIELCh with hue measured as a chord across the hue circle, scaled by both chromas.',
		math: 'd² = ΔL*² + ΔC*² + (2·√(C₁C₂)·sin(Δh/2))²',
		latex: String.raw`d^2 = \Delta L^{*2} + \Delta C^{*2} + \left(2\sqrt{C_1 C_2}\,\sin\tfrac{\Delta h}{2}\right)^2`
	},
	{
		id: 'cielch-euclidean',
		label: 'CIELCh · Euclidean',
		short:
			'Compares CIELCh lightness, chroma, and hue angle as plain numbers. Hue does not wrap across 0°.',
		math: 'd² = ΔL*² + ΔC*² + Δh² (h in radians, unwrapped)',
		latex: String.raw`d^2 = \Delta L^{*2} + \Delta C^{*2} + \Delta h^2`
	},
	{
		id: 'ycbcr',
		label: 'YCbCr',
		short:
			'Full-range BT.601 luma and chroma, as video codecs use. Brightness and color are separate axes, measured with equal weight.',
		math: 'd² = ΔY² + ΔCb² + ΔCr²',
		latex: String.raw`d^2 = \Delta Y^2 + \Delta C_b^2 + \Delta C_r^2`
	}
] as const satisfies readonly ColorSpaceOption[];
