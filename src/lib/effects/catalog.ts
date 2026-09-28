import type { CurvesEffect, Effect, ModelCurvesEffect } from 'ditherette';

export type CurveChannel = 'red' | 'green' | 'blue';
export type CurvePoints = CurvesEffect['points'];

/** A curves layer keeps one curve per channel. Processing splits it into package curves steps. */
export type ChannelCurves = {
	readonly effect: 'curves';
	readonly enabled: boolean;
	readonly curves: { readonly [C in CurveChannel]: CurvePoints };
};

/**
 * What a layer stores: a package step, except that curves keep a curve per channel. Colour-model
 * curves have no editor yet, so layers don't hold them.
 */
export type LayerStep = Exclude<Effect, CurvesEffect | ModelCurvesEffect> | ChannelCurves;
export type EffectKind = LayerStep['effect'];
export type EffectOf<K extends EffectKind> = Extract<LayerStep, { effect: K }>;

export const CURVE_CHANNELS = ['red', 'green', 'blue'] as const satisfies readonly CurveChannel[];
const STRAIGHT: CurvePoints = [
	[0, 0],
	[1, 1]
];

export const samePoints = (left: CurvePoints, right: CurvePoints) =>
	left.length === right.length &&
	left.every(([x, y], index) => x === right[index]![0] && y === right[index]![1]);

/** The package steps a layer runs: one `rgb` curve when all three match, otherwise one per channel. */
export function packageSteps(step: LayerStep): Effect[] {
	if (step.effect !== 'curves') return [step];
	const { red, green, blue } = step.curves;
	if (samePoints(red, green) && samePoints(red, blue))
		return [{ effect: 'curves', enabled: step.enabled, channel: 'rgb', points: red }];
	return CURVE_CHANNELS.filter((channel) => !samePoints(step.curves[channel], STRAIGHT)).map(
		(channel) => ({
			effect: 'curves',
			enabled: step.enabled,
			channel,
			points: step.curves[channel]
		})
	);
}

type CatalogEntry<K extends EffectKind> = {
	/** Name shown in menus and given to new instances. */
	readonly label: string;
	/** A neutral step: adding it leaves the image unchanged, except palette fit. */
	readonly create: () => EffectOf<K>;
};

/** Every built-in effect the website offers, in menu order. */
export const EFFECTS: { readonly [K in EffectKind]: CatalogEntry<K> } = {
	levels: {
		label: 'Levels',
		create: () => ({
			effect: 'levels',
			enabled: true,
			channel: 'rgb',
			input: { black: 0, white: 1 },
			gamma: 1,
			output: { black: 0, white: 1 }
		})
	},
	curves: {
		label: 'Curves',
		create: () => ({
			effect: 'curves',
			enabled: true,
			curves: { red: STRAIGHT, green: STRAIGHT, blue: STRAIGHT }
		})
	},
	'brightness-contrast': {
		label: 'Brightness and contrast',
		create: () => ({ effect: 'brightness-contrast', enabled: true, brightness: 0, contrast: 0 })
	},
	exposure: {
		label: 'Exposure',
		create: () => ({ effect: 'exposure', enabled: true, stops: 0 })
	},
	'white-balance': {
		label: 'White balance',
		create: () => ({ effect: 'white-balance', enabled: true, temperature: 0, tint: 0 })
	},
	'hue-saturation': {
		label: 'Hue and saturation',
		create: () => ({
			effect: 'hue-saturation',
			enabled: true,
			hue: 0,
			saturation: 0,
			lightness: 0
		})
	},
	recolour: {
		label: 'Palette fit',
		create: () => ({ effect: 'recolour', enabled: true, strength: 1, recipe: null })
	}
};

/** The package accepts at most 64 steps. */
export const MAX_EFFECT_LAYERS = 64;

export const EFFECT_KINDS = Object.keys(EFFECTS) as EffectKind[];
