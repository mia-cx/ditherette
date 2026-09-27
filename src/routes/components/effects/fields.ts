import type { EffectKind, EffectOf } from '$lib/effects/catalog';

type NumericKey<S> = { [K in keyof S]: S[K] extends number ? K : never }[keyof S];

/** One slider. Bounds and step are in display units; the step stores `display / scale`. */
export type NumberField<S> = {
	readonly key: NumericKey<S>;
	readonly label: string;
	readonly min: number;
	readonly max: number;
	readonly step: number;
	readonly scale?: number;
	readonly unit?: string;
};

const PERCENT = { min: -100, max: 100, step: 1, scale: 100, unit: '%' } as const;

/** Effects whose editor is a list of sliders. Levels and curves have their own editors. */
export const NUMBER_FIELDS: {
	readonly [K in Exclude<EffectKind, 'levels' | 'curves'>]: readonly NumberField<EffectOf<K>>[];
} = {
	'brightness-contrast': [
		{ key: 'brightness', label: 'Brightness', ...PERCENT },
		{ key: 'contrast', label: 'Contrast', ...PERCENT }
	],
	exposure: [{ key: 'stops', label: 'Exposure', min: -4, max: 4, step: 0.05, unit: 'EV' }],
	'white-balance': [
		{ key: 'temperature', label: 'Temperature', ...PERCENT },
		{ key: 'tint', label: 'Tint', ...PERCENT }
	],
	'hue-saturation': [
		{ key: 'hue', label: 'Hue', min: -180, max: 180, step: 1, unit: '°' },
		{ key: 'saturation', label: 'Saturation', ...PERCENT },
		{ key: 'lightness', label: 'Lightness', ...PERCENT }
	],
	recolour: [{ key: 'strength', label: 'Strength', ...PERCENT, min: 0 }]
};
