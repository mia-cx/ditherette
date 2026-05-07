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
		id: 'blue-magenta-coral-arc',
		label: 'Blue → Magenta → Coral arc',
		description:
			'Three-color curved ramp for dither previews. Geometry bows up-left from bottom-left to top-right so Bayer matrix sizes are easier to compare.',
		stops: [
			{ position: 0, color: { r: 20, g: 40, b: 120 } },
			{ position: 0.5, color: { r: 255, g: 0, b: 255 } },
			{ position: 1, color: { r: 250, g: 128, b: 114 } }
		]
	}
] satisfies PreviewGradientPreset[];

export const DEFAULT_DITHER_PREVIEW_GRADIENT = DITHER_PREVIEW_GRADIENTS[0];
