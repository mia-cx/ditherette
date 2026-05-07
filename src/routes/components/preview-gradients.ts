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
		id: 'teal-lime-coral-arc',
		label: 'Teal → Lime → Coral arc',
		description:
			'Three-color curved ramp for dither previews. Geometry bows up-left from bottom-left to top-right so Bayer matrix sizes are easier to compare.',
		stops: [
			{ position: 0, color: { r: 12, g: 129, b: 110 } },
			{ position: 0.5, color: { r: 135, g: 255, b: 94 } },
			{ position: 1, color: { r: 250, g: 128, b: 114 } }
		]
	}
] satisfies PreviewGradientPreset[];

export const DEFAULT_DITHER_PREVIEW_GRADIENT = DITHER_PREVIEW_GRADIENTS[0];
