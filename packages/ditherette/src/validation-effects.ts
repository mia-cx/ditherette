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
export interface Needs {
	readonly palette: boolean;
	readonly space: boolean;
}

interface Builtin {
	readonly keys: readonly string[];
	/** Context the normalized effect reads when enabled. */
	needs(effect: Record<string, unknown>): Needs;
	/** Returns fresh canonical arguments, reading each caller property once. */
	normalize(effect: Record<string, unknown>, path: string): Record<string, unknown>;
}

/** One enabled step's context requirements, checked in order like the Rust executor. */
export interface Requirement {
	readonly index: number;
	readonly needs: Needs;
	/** The working space an explicit recolour recipe was analysed in. */
	readonly recipeSpace?: string;
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

/** Mirrors the Rust `curves::MIN_GAP`, compared as f32 like Rust. */
const minGap = Math.fround(0.001);

/** 2 to 16 exact `[x, y]` pairs in `[0, 1]`, x rising by at least 0.001, as Rust validates them. */
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
		const x = bounded(Object.hasOwn(point, 0) ? point[0] : undefined, 0, 1, `${pointPath}.0`);
		const y = bounded(Object.hasOwn(point, 1) ? point[1] : undefined, 0, 1, `${pointPath}.1`);
		if (index > 0 && Math.fround(x - points[index - 1][0]) < minGap)
			throw new DitheretteError(
				'invalid-settings',
				`${pointPath}.0`,
				'Point x values must increase by at least 0.001.'
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

const none = () => ({ palette: false, space: false });

/** Exact `[u, v]` shift within `±0.5`. */
function shiftPair(value: unknown, path: string): [number, number] {
	if (
		!Array.isArray(value) ||
		value.length !== 2 ||
		Reflect.ownKeys(value).some((key) => !['0', '1', 'length'].includes(String(key)))
	)
		throw new DitheretteError('invalid-settings', path, 'Expected a [u, v] pair.');
	return [
		bounded(Object.hasOwn(value, 0) ? value[0] : undefined, -0.5, 0.5, `${path}.0`),
		bounded(Object.hasOwn(value, 1) ? value[1] : undefined, -0.5, 0.5, `${path}.1`)
	];
}

/** Mirrors the Rust `RecolourRecipe::validate`, field by field and in the same order. */
function recolourRecipe(value: unknown, path: string) {
	const recipe = object(
		value,
		['space', 'tone', 'chroma', 'shift', 'groups'],
		'invalid-settings',
		path
	);
	const space = field(recipe, 'space');
	if (typeof space !== 'string' || !spaces.includes(space))
		throw new DitheretteError('invalid-settings', `${path}.space`, 'Unknown working space.');
	const tone = curvePoints(field(recipe, 'tone'), `${path}.tone`);
	const chroma = bounded(field(recipe, 'chroma'), 0, 2, `${path}.chroma`);
	const shift = shiftPair(field(recipe, 'shift'), `${path}.shift`);
	const rawGroups = field(recipe, 'groups');
	if (!Array.isArray(rawGroups) || rawGroups.length > 12)
		throw new DitheretteError('invalid-settings', `${path}.groups`, 'Expected at most 12 groups.');
	const count = rawGroups.length;
	const groups = [];
	for (let index = 0; index < count; index++) {
		const at = `${path}.groups.${index}`;
		const group = object(
			Object.hasOwn(rawGroups, index) ? rawGroups[index] : undefined,
			['hue', 'width', 'turn', 'chroma'],
			'invalid-settings',
			at
		);
		groups.push({
			hue: bounded(field(group, 'hue'), 0, 360, `${at}.hue`),
			width: bounded(field(group, 'width'), 1, 180, `${at}.width`),
			turn: bounded(field(group, 'turn'), -180, 180, `${at}.turn`),
			chroma: bounded(field(group, 'chroma'), 0, 2, `${at}.chroma`)
		});
	}
	return { space, tone, chroma, shift, groups };
}

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
	},
	recolour: {
		keys: ['strength', 'recipe'],
		needs: (effect) => ({ palette: effect.recipe === null, space: true }),
		normalize: (effect, path) => {
			const strength = bounded(field(effect, 'strength'), 0, 1, `${path}.strength`);
			const recipe = field(effect, 'recipe');
			return {
				strength,
				recipe: recipe === null ? null : recolourRecipe(recipe, `${path}.recipe`)
			};
		}
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
	const requirements: Requirement[] = [];
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
		const normalized = builtin.normalize(effect, stepPath);
		steps.push({ effect: name, enabled, ...normalized });
		if (enabled) {
			const recipe = normalized.recipe as { space: string } | null | undefined;
			requirements.push({ index, needs: builtin.needs(normalized), recipeSpace: recipe?.space });
		}
	}
	return { json: JSON.stringify(steps), enabled: steps.some((step) => step.enabled), requirements };
}

export type ValidatedEffects = ReturnType<typeof validateEffects>;

/** Private tag for "no working space supplied". */
const NO_SPACE = -1;

/**
 * Reject missing or mismatched context before any Wasm work, step by step like Rust.
 * `palette` holds private palette codes; `space` is the context's working-space name.
 */
export function requireContext(
	effects: ValidatedEffects,
	palette: readonly number[] | undefined,
	space: string | undefined,
	paths: { readonly effects: string; readonly palette: string; readonly space: string }
): void {
	for (const { index, needs, recipeSpace } of effects.requirements) {
		if (needs.palette && !palette?.some((code) => code !== TRANSPARENT_CODE))
			throw new DitheretteError(
				'invalid-request',
				paths.palette,
				`Effect effects.${index} requires a visible palette colour.`
			);
		if (needs.space && space === undefined)
			throw new DitheretteError(
				'invalid-request',
				paths.space,
				`Effect effects.${index} requires a working space.`
			);
		if (recipeSpace !== undefined && recipeSpace !== space)
			throw new DitheretteError(
				'invalid-settings',
				`${paths.effects}.${index}.recipe.space`,
				'The recipe was analysed in a different working space than this context.'
			);
	}
}

/** Normalize `analyzeRecolour`: an `applyEffects` request whose context must hold a colour and a space. */
export function validateAnalyzeRecolour(value: unknown) {
	const input = validateApplyEffects(value);
	if (!input.palette?.some((code) => code !== TRANSPARENT_CODE))
		throw new DitheretteError(
			'invalid-request',
			'context.palette',
			'Analysis requires a visible palette colour.'
		);
	if (input.space === NO_SPACE)
		throw new DitheretteError(
			'invalid-request',
			'context.space',
			'Analysis requires a working space.'
		);
	return input;
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
		requireContext(effects, palette, space === NO_SPACE ? undefined : spaces[space], {
			effects: 'effects',
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
