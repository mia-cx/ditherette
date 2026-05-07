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
		id: 'navy-teal-lime-gold-coral-arc',
		label: 'Navy → Teal → Lime → Gold → Coral arc',
		description:
			'Five-color curved ramp for dither previews: dark-to-light value range plus hue shifts through green/yellow/red to expose ordered matrix scale.',
		stops: [
			{ position: 0, color: { r: 8, g: 22, b: 88 } },
			{ position: 0.25, color: { r: 12, g: 129, b: 110 } },
			{ position: 0.5, color: { r: 135, g: 255, b: 94 } },
			{ position: 0.75, color: { r: 249, g: 221, b: 59 } },
			{ position: 1, color: { r: 250, g: 128, b: 114 } }
		]
	}
] satisfies PreviewGradientPreset[];

export const DEFAULT_DITHER_PREVIEW_GRADIENT = DITHER_PREVIEW_GRADIENTS[0];
