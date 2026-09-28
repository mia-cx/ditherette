import type { ColorSpaceId } from '$lib/processing/types';

export type ColorSpaceOption = {
	id: ColorSpaceId;
	label: string;
	/** What the mode does to the image. Measured against OKLab on a hue and chroma sweep. */
	description: string;
};

export const COLOR_SPACES = [
	{
		id: 'oklab',
		label: 'OKLab',
		description:
			'Picks the palette colour that looks closest, balancing brightness, colourfulness, and hue. The best place to start.'
	},
	{
		id: 'srgb',
		label: 'sRGB',
		description:
			'Compares raw screen values. Results tend to be duller than with OKLab, and shadows drift in brightness.'
	},
	{
		id: 'linear-rgb',
		label: 'Linear RGB',
		description:
			'Compares amounts of light. Dark colours measure as nearly the same, so shadows lose the most detail of any mode.'
	},
	{
		id: 'weighted-rgb',
		label: 'Weighted RGB',
		description:
			'A quick fix for sRGB that weighs green most, and red or blue more depending on how red the colour is. A little closer to what you see than sRGB.'
	},
	{
		id: 'weighted-rgb-601',
		label: 'Weighted RGB · Rec.601',
		description:
			'Weighs green most and blue least, as TV brightness does. Shading holds up better than in sRGB, but blues can fade to grey.'
	},
	{
		id: 'weighted-rgb-709',
		label: 'Weighted RGB · Rec.709',
		description:
			'Like Rec.601, with even more weight on green. The best shading of the RGB modes, and the most blues fading to grey.'
	},
	{
		id: 'oklch',
		label: 'OKLCH',
		description:
			'OKLab described by lightness, chroma, and hue angle instead of two colour axes. Hue counts only as much as the duller of two colours is colourful, so vivid areas keep their hue and stay more colourful than with OKLab.'
	},
	{
		id: 'oklch-circular-hue',
		label: 'OKLCH · circular hue',
		description:
			'Measures the same distance as OKLab, so it picks exactly the same colours. It only differs with Dither in selected space: error then spreads as lightness, chroma, and hue, which leaves dithered areas duller.'
	},
	{
		id: 'oklch-euclidean',
		label: 'OKLCH · Euclidean',
		description:
			'Compares hue angle as a plain number, which outweighs lightness and chroma. Hue stays closest of any mode, but greys pick up colour, brightness drifts, and reds either side of 0° count as opposites.'
	},
	{
		id: 'cielab',
		label: 'CIELAB ΔE76',
		description:
			'The classic perceptual space, older than OKLab. It holds hue more closely than OKLab, but brightness drifts more.'
	},
	{
		id: 'cielab-ciede2000',
		label: 'CIELAB ΔE2000',
		description:
			"CIE's 2000 correction to CIELAB for blues, greys, and vivid colours. It keeps more colour than OKLab at some cost in brightness, and it's the slowest mode."
	},
	{
		id: 'cielch',
		label: 'CIELCh',
		description:
			'CIELAB described by lightness, chroma, and hue angle. As in OKLCH, hue counts only as much as the duller of two colours is colourful.'
	},
	{
		id: 'cielch-circular-hue',
		label: 'CIELCh · circular hue',
		description:
			'Measures the same distance as CIELAB ΔE76, so it picks exactly the same colours. It only differs with Dither in selected space, where error spreads as lightness, chroma, and hue.'
	},
	{
		id: 'cielch-euclidean',
		label: 'CIELCh · Euclidean',
		description:
			"Compares hue angle as a plain number, but CIELCh's angle is tiny next to its lightness and chroma, so hue barely counts. It matches brightness and colourfulness and lets hue change freely."
	},
	{
		id: 'ycbcr',
		label: 'YCbCr',
		description:
			'Splits brightness from colour, as video does, and weighs them equally. Results land close to the weighted RGB modes.'
	}
] as const satisfies readonly ColorSpaceOption[];
