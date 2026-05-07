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
		id: 'deep-blue-cyan-white-rose-amber-arc',
		label: 'Deep blue → Cyan → White → Rose → Amber arc',
		description:
			'Five-color curved ramp for dither previews: strong dark-to-light-to-warm luminance changes without adjacent green/yellow or purple/red mush.',
		stops: [
			{ position: 0, color: { r: 7, g: 18, b: 84 } },
			{ position: 0.25, color: { r: 6, g: 182, b: 212 } },
			{ position: 0.5, color: { r: 245, g: 245, b: 245 } },
			{ position: 0.75, color: { r: 244, g: 63, b: 94 } },
			{ position: 1, color: { r: 251, g: 146, b: 60 } }
		]
	}
] satisfies PreviewGradientPreset[];

export const DEFAULT_DITHER_PREVIEW_GRADIENT = DITHER_PREVIEW_GRADIENTS[0];
