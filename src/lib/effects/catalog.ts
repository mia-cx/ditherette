import type { Effect } from 'ditherette';
import { isValidCurve } from './spline';

export type EffectKind = Effect['effect'];
export type EffectOf<K extends EffectKind> = Extract<Effect, { effect: K }>;

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
			channel: 'rgb',
			points: [
				[0, 0],
				[1, 1]
			]
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

/** True when `value` has the same keys and value types as `template`, recursively. */
function sameShape(value: unknown, template: unknown): boolean {
	if (template === null) return value === null;
	if (typeof template === 'number') return typeof value === 'number' && Number.isFinite(value);
	if (typeof template !== 'object') return typeof value === typeof template;
	if (Array.isArray(template))
		return Array.isArray(value) && value.every((item) => sameShape(item, template[0]));
	if (!value || typeof value !== 'object' || Array.isArray(value)) return false;
	const keys = Object.keys(template);
	const record = value as Record<string, unknown>;
	return (
		Object.keys(record).length === keys.length &&
		keys.every(
			(key) =>
				Object.hasOwn(record, key) &&
				sameShape(record[key], (template as Record<string, unknown>)[key])
		)
	);
}

/**
 * A built-in effect step shaped like its neutral default, so editors can render it. Saved steps
 * from storage pass through here; the package still checks argument ranges when it runs.
 */
export function isEffectStep(value: unknown): value is Effect {
	if (!value || typeof value !== 'object') return false;
	const { effect } = value as Record<string, unknown>;
	if (typeof effect !== 'string' || !Object.hasOwn(EFFECTS, effect)) return false;
	const step = value as Effect;
	if (!sameShape(step, EFFECTS[step.effect].create())) return false;
	return step.effect !== 'curves' || isValidCurve(step.points);
}
