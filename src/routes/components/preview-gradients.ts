import type { Rgb } from '$lib/processing/types';

export type PreviewGradientStop = {
	position: number;
	color: Rgb;
};

export type PreviewGradientPreset = {
	id: string;
	label: string;
	description: string;
	from: 'bottom-left';
	to: 'top-right';
	stops: PreviewGradientStop[];
};

export const DITHER_PREVIEW_GRADIENTS = [
	{
		id: 'blue-magenta-red-diagonal',
		label: 'Blue → Magenta → Red',
		description:
			'Dark blue to light red through magenta, along the bottom-left to top-right diagonal.',
		from: 'bottom-left',
		to: 'top-right',
		stops: [
			{ position: 0, color: { r: 20, g: 40, b: 120 } },
			{ position: 0.5, color: { r: 255, g: 0, b: 255 } },
			{ position: 1, color: { r: 250, g: 128, b: 114 } }
		]
	}
] satisfies PreviewGradientPreset[];

export const DEFAULT_DITHER_PREVIEW_GRADIENT = DITHER_PREVIEW_GRADIENTS[0];
