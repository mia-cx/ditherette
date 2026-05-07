import type { Rgb } from '$lib/processing/types';

export type PreviewGradientPoint = {
	x: number;
	y: number;
	color: Rgb;
};

export type PreviewGradientPreset = {
	id: string;
	label: string;
	description: string;
	points: PreviewGradientPoint[];
};

export const DITHER_PREVIEW_GRADIENTS = [
	{
		id: 'navy-cyan-coral-arc',
		label: 'Navy → Cyan → Coral arc',
		description:
			'High-contrast curved ramp for dither previews: dark navy in the lower-left, bright cyan off-axis, warm coral in the upper-right.',
		points: [
			{ x: 0, y: 1, color: { r: 12, g: 24, b: 80 } },
			{ x: 0.32, y: 0.18, color: { r: 34, g: 211, b: 238 } },
			{ x: 1, y: 0, color: { r: 251, g: 113, b: 133 } }
		]
	}
] satisfies PreviewGradientPreset[];

export const DEFAULT_DITHER_PREVIEW_GRADIENT = DITHER_PREVIEW_GRADIENTS[0];
