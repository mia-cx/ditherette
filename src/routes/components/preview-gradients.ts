import type { Rgb } from '$lib/processing/types';

export type PreviewGradientStop = {
	position: number;
	color: Rgb;
};

export type PreviewGradientPreset = {
	id: string;
	label: string;
	description: string;
	stops: PreviewGradientStop[];
};

export const DITHER_PREVIEW_GRADIENTS = [
	{
		id: 'five-stop-grayscale-arc',
		label: 'Five-stop grayscale arc',
		description:
			'Monochrome curved luminance ramp with five tonal stops; removes hue so matrix scale and texture are easiest to compare.',
		stops: [
			{ position: 0, color: { r: 0, g: 0, b: 0 } },
			{ position: 0.2, color: { r: 32, g: 32, b: 32 } },
			{ position: 0.5, color: { r: 119, g: 119, b: 119 } },
			{ position: 0.8, color: { r: 221, g: 221, b: 221 } },
			{ position: 1, color: { r: 255, g: 255, b: 255 } }
		]
	}
] satisfies PreviewGradientPreset[];

export const DEFAULT_DITHER_PREVIEW_GRADIENT = DITHER_PREVIEW_GRADIENTS[0];
