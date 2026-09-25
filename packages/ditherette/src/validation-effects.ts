import { DitheretteError } from './errors.js';
import { spaces } from './validation-fields.js';
import {
	dimensions,
	field,
	object,
	paletteCodes,
	progressCallback,
	rgbaBytes,
	TRANSPARENT_CODE
} from './validation.js';
import type { Rgba8Image } from './types.js';

const channels = ['rgb', 'red', 'green', 'blue'];
/** Mirrors the Rust `MAX_EFFECTS`. */
const maxEffects = 64;

/** Context an enabled effect reads. The processor supplies it; ordinary effects need none. */
interface Needs {
	readonly palette: boolean;
	readonly space: boolean;
}

interface Builtin {
	readonly keys: readonly string[];
	readonly needs: Needs;
	/** Returns fresh canonical arguments, reading each caller property once. */
	normalize(effect: Record<string, unknown>, path: string): Record<string, unknown>;
}

/** Round to f32 first, so bounds checks see exactly the value Rust decodes. */
function bounded(value: unknown, minimum: number, maximum: number, path: string): number {
	const rounded = typeof value === 'number' ? Math.fround(value) : Number.NaN;
	if (!Number.isFinite(rounded) || rounded < minimum || rounded > maximum)
		throw new DitheretteError(
			'invalid-settings',
			path,
			`Expected a finite number between ${minimum} and ${maximum}.`
		);
	return rounded;
}

function points(value: unknown, path: string) {
	const input = object(value, ['black', 'white'], 'invalid-settings', path);
	return {
		black: bounded(field(input, 'black'), 0, 1, `${path}.black`),
		white: bounded(field(input, 'white'), 0, 1, `${path}.white`)
	};
}

function channel(value: unknown, path: string): string {
	if (typeof value !== 'string' || !channels.includes(value))
		throw new DitheretteError('invalid-settings', path, 'Expected rgb, red, green, or blue.');
	return value;
}

/** 2 to 16 exact `[x, y]` pairs in `[0, 1]` with strictly increasing x, as Rust validates them. */
function curvePoints(value: unknown, path: string): [number, number][] {
	if (!Array.isArray(value) || value.length < 2 || value.length > 16)
		throw new DitheretteError('invalid-settings', path, 'Expected 2 to 16 points.');
	const count = value.length;
	const points: [number, number][] = [];
	for (let index = 0; index < count; index++) {
		const pointPath = `${path}.${index}`;
		const point: unknown = Object.hasOwn(value, index) ? value[index] : undefined;
		if (
			!Array.isArray(point) ||
			point.length !== 2 ||
			Reflect.ownKeys(point).some((key) => !['0', '1', 'length'].includes(String(key)))
		)
			throw new DitheretteError('invalid-settings', pointPath, 'Expected an [x, y] pair.');
		const x = bounded(point[0], 0, 1, `${pointPath}.0`);
		const y = bounded(point[1], 0, 1, `${pointPath}.1`);
		if (index > 0 && x <= points[index - 1][0])
			throw new DitheretteError(
				'invalid-settings',
				`${pointPath}.0`,
				'Point x values must strictly increase.'
			);
		points.push([x, y]);
	}
	return points;
}

/** Named arguments that are each a bounded f32, in validation order. */
function scalars(ranges: Record<string, readonly [number, number]>): Builtin['normalize'] {
	return (effect, path) =>
		Object.fromEntries(
			Object.entries(ranges).map(([key, [minimum, maximum]]) => [
				key,
				bounded(field(effect, key), minimum, maximum, `${path}.${key}`)
			])
		);
}

const none = { palette: false, space: false } as const;

/** The static registry. Keys mirror the Rust `BuiltinEffect` tags. */
const builtins: Record<string, Builtin> = {
	levels: {
		keys: ['channel', 'input', 'gamma', 'output'],
		needs: none,
		normalize(effect, path) {
			const selected = channel(field(effect, 'channel'), `${path}.channel`);
			const input = points(field(effect, 'input'), `${path}.input`);
			if (input.black >= input.white)
				throw new DitheretteError(
					'invalid-settings',
					`${path}.input`,
					'Input black must be below input white.'
				);
			return {
				channel: selected,
				input,
				gamma: bounded(field(effect, 'gamma'), 0.1, 10, `${path}.gamma`),
				output: points(field(effect, 'output'), `${path}.output`)
			};
		}
	},
	curves: {
		keys: ['channel', 'points'],
		needs: none,
		normalize: (effect, path) => ({
			channel: channel(field(effect, 'channel'), `${path}.channel`),
			points: curvePoints(field(effect, 'points'), `${path}.points`)
		})
	},
	'brightness-contrast': {
		keys: ['brightness', 'contrast'],
		needs: none,
		normalize: scalars({ brightness: [-1, 1], contrast: [-1, 1] })
	},
	exposure: {
		keys: ['stops'],
		needs: none,
		normalize: scalars({ stops: [-4, 4] })
	},
	'white-balance': {
		keys: ['temperature', 'tint'],
		needs: none,
		normalize: scalars({ temperature: [-1, 1], tint: [-1, 1] })
	},
	'hue-saturation': {
		keys: ['hue', 'saturation', 'lightness'],
		needs: none,
		normalize: scalars({ hue: [-180, 180], saturation: [-1, 1], lightness: [-1, 1] })
	}
};

/**
 * Validate an ordered effects array with indexed paths such as `effects.2.gamma`.
 * Disabled steps are validated too. Returns canonical JSON for the private binding.
 */
export function validateEffects(value: unknown, path: string) {
	if (!Array.isArray(value))
		throw new DitheretteError('invalid-settings', path, 'Expected an array of effects.');
	const count = value.length;
	if (count > maxEffects)
		throw new DitheretteError(
			'invalid-settings',
			path,
			`A chain holds at most ${maxEffects} effects.`
		);
	const steps: Record<string, unknown>[] = [];
	const needs = { palette: false, space: false };
	for (let index = 0; index < count; index++) {
		const stepPath = `${path}.${index}`;
		const raw = Object.hasOwn(value, index) ? value[index] : undefined;
		if (typeof raw !== 'object' || raw === null || Array.isArray(raw))
			throw new DitheretteError('invalid-settings', stepPath, 'Expected an effect object.');
		const name = field(raw as Record<string, unknown>, 'effect');
		const builtin =
			typeof name === 'string' && Object.hasOwn(builtins, name) ? builtins[name] : undefined;
		if (!builtin)
			throw new DitheretteError('invalid-settings', `${stepPath}.effect`, 'Unknown effect.');
		const effect = object(
			raw,
			['effect', 'enabled', ...builtin.keys],
			'invalid-settings',
			stepPath
		);
		const enabled = field(effect, 'enabled');
		if (typeof enabled !== 'boolean')
			throw new DitheretteError('invalid-settings', `${stepPath}.enabled`, 'Expected a boolean.');
		steps.push({ effect: name, enabled, ...builtin.normalize(effect, stepPath) });
		if (enabled) {
			needs.palette ||= builtin.needs.palette;
			needs.space ||= builtin.needs.space;
		}
	}
	return { json: JSON.stringify(steps), enabled: steps.some((step) => step.enabled), needs };
}

/** Private tag for "no working space supplied". */
const NO_SPACE = -1;

/** Reject missing context before any Wasm work. `palette` holds private palette codes. */
export function requireContext(
	needs: Needs,
	palette: readonly number[] | undefined,
	space: boolean,
	paths: { readonly palette: string; readonly space: string }
): void {
	if (needs.palette && !palette?.some((code) => code !== TRANSPARENT_CODE))
		throw new DitheretteError(
			'invalid-request',
			paths.palette,
			'An enabled effect requires a visible palette colour.'
		);
	if (needs.space && !space)
		throw new DitheretteError(
			'invalid-request',
			paths.space,
			'An enabled effect requires a working space.'
		);
}

/** Normalize `applyEffects` once under the instance guard. */
export function validateApplyEffects(value: unknown) {
	try {
		const request = object(
			value,
			['version', 'source', 'effects', 'context', 'onProgress'],
			'invalid-request',
			'request'
		);
		if (field(request, 'version') !== 1)
			throw new DitheretteError('invalid-request', 'version', 'Unsupported request version.');
		const onProgress = progressCallback(field(request, 'onProgress'));
		const effects = validateEffects(field(request, 'effects'), 'effects');
		const rawContext = field(request, 'context');
		const context = object(
			rawContext === undefined ? {} : rawContext,
			['palette', 'space'],
			'invalid-settings',
			'context'
		);
		const rawPalette = field(context, 'palette');
		// An empty palette supplies no colours, exactly like an omitted one.
		const palette =
			rawPalette === undefined || (Array.isArray(rawPalette) && rawPalette.length === 0)
				? undefined
				: paletteCodes(rawPalette, 'context.palette');
		const rawSpace = field(context, 'space');
		const space = rawSpace === undefined ? NO_SPACE : spaces.indexOf(rawSpace as string);
		if (rawSpace !== undefined && (typeof rawSpace !== 'string' || space < 0))
			throw new DitheretteError('invalid-settings', 'context.space', 'Unknown working space.');
		requireContext(effects.needs, palette, space !== NO_SPACE, {
			palette: 'context.palette',
			space: 'context.space'
		});
		const source = object(
			field(request, 'source'),
			['width', 'height', 'data'],
			'invalid-image',
			'source'
		);
		const size = dimensions(source, 32_768, 'invalid-image', 'source');
		return {
			// Dimensions, allowed keys and packed bytes validate this caller-owned image.
			source: source as unknown as Rgba8Image,
			data: rgbaBytes(field(source, 'data'), size.width * size.height * 4),
			sourceWidth: size.width,
			sourceHeight: size.height,
			effects,
			palette,
			space,
			onProgress
		};
	} catch (error) {
		if (error instanceof DitheretteError) throw error;
		throw new DitheretteError(
			error instanceof RangeError ? 'wasm-memory-unavailable' : 'invalid-request',
			'request',
			'Effects request could not be normalized.'
		);
	}
}
