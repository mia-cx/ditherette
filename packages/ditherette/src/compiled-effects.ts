import { DitheretteError } from './errors.js';
import type { Ditherette, Effect, EffectContext, MaskCurve, Rgba8Image } from './types.js';

/**
 * An image's distinct RGB colours and, for each pixel, the index of its colour. Colours are packed
 * `r | g << 8 | b << 16`, in first-seen order; alpha plays no part.
 */
export interface IndexedColours {
	readonly colours: Uint32Array;
	readonly indices: Uint32Array;
}

/** A compile request: every pointwise effect maps each listed colour on its own. */
export interface CompileEffectsRequest {
	readonly version: 1;
	/** Packed colours to map, usually `indexColours(image).colours`. */
	readonly colours: Uint32Array;
	/** Every palette fit needs its recipe; derive a missing one with `analyzeRecolour`. */
	readonly effects: readonly Effect[];
	readonly context?: EffectContext;
}

const COLOUR_ENTRIES = 1 << 24;
/** Width of the image the colours are laid out in. */
const COLOUR_ROW = 4096;

/**
 * Index an image's colours. Build it once per image; `compileEffects` then maps each distinct
 * colour, and `applyCompiledEffects` or a GPU lookup maps the pixels.
 */
export function indexColours(image: Rgba8Image): IndexedColours {
	const { data } = image;
	const pixels = data.length / 4;
	// Each seen colour's index plus one, so a fresh zeroed array means unseen.
	const slot = new Uint32Array(COLOUR_ENTRIES);
	const colours = new Uint32Array(Math.min(pixels, COLOUR_ENTRIES));
	const indices = new Uint32Array(pixels);
	let count = 0;
	for (let pixel = 0, offset = 0; pixel < pixels; pixel++, offset += 4) {
		const colour = data[offset]! | (data[offset + 1]! << 8) | (data[offset + 2]! << 16);
		let index = slot[colour]!;
		if (!index) {
			colours[count] = colour;
			index = slot[colour] = ++count;
		}
		indices[pixel] = index - 1;
	}
	return { colours: colours.slice(0, count), indices };
}

/** A mask compile request: the mask of a step after `effects`, for each listed colour. */
export interface CompileEffectMaskRequest extends CompileEffectsRequest {
	/** The masked step's curves. Missing or empty is full strength everywhere. */
	readonly mask?: readonly MaskCurve[];
}

/**
 * Run a pointwise effect chain once per colour. Returns each colour's result, packed like the input.
 * Effects round to RGBA8 once, so mapping a pixel through its colour's result is exact.
 */
export function compileEffects(
	ditherette: Ditherette,
	{ colours, effects, context }: CompileEffectsRequest
): Uint32Array {
	return perColour(colours, effects, (source) =>
		ditherette.applyEffects({ version: 1, source, effects, ...(context ? { context } : {}) })
	);
}

/**
 * Each colour's strength for the mask of a step after `effects`, as a packed grey
 * `round(strength * 255)`. Map pixels through it like `compileEffects` results.
 */
export function compileEffectMask(
	ditherette: Ditherette,
	{ colours, effects, mask, context }: CompileEffectMaskRequest
): Uint32Array {
	return perColour(colours, effects, (source) =>
		ditherette.effectMask({
			version: 1,
			source,
			effects,
			...(mask ? { mask } : {}),
			...(context ? { context } : {})
		})
	);
}

/** Lays the colours out as an image, runs `apply` on it, and packs each colour's result. */
function perColour(
	colours: Uint32Array,
	effects: readonly Effect[],
	apply: (source: Rgba8Image) => Rgba8Image
): Uint32Array {
	const unresolved = effects.findIndex(
		(step) =>
			step.enabled &&
			((step.effect === 'recolour' && step.recipe === null) ||
				(step.effect === 'palette-fit' && step.curves === null))
	);
	if (unresolved >= 0)
		throw new DitheretteError(
			'invalid-request',
			`effects.${unresolved}.${effects[unresolved]!.effect === 'palette-fit' ? 'curves' : 'recipe'}`,
			effects[unresolved]!.effect === 'palette-fit'
				? 'Palette fit needs its curves; derive them with analyzePaletteFit.'
				: 'Palette fit needs a recipe; derive one with analyzeRecolour.'
		);
	const results = new Uint32Array(colours.length);
	if (!colours.length) return results;
	const width = Math.min(colours.length, COLOUR_ROW);
	const height = Math.ceil(colours.length / width);
	const data = new Uint8Array(width * height * 4);
	for (let index = 0; index < width * height; index++) {
		// Pad the last row with the last colour; padding results are ignored.
		const colour = colours[Math.min(index, colours.length - 1)]!;
		data[index * 4] = colour & 0xff;
		data[index * 4 + 1] = (colour >>> 8) & 0xff;
		data[index * 4 + 2] = (colour >>> 16) & 0xff;
		data[index * 4 + 3] = 0xff;
	}
	const output = apply({ width, height, data }).data;
	for (let index = 0, offset = 0; index < colours.length; index++, offset += 4)
		results[index] = output[offset]! | (output[offset + 1]! << 8) | (output[offset + 2]! << 16);
	return results;
}

/**
 * `image` with each pixel's RGB replaced by its colour's result, keeping alpha. Pass the previous
 * output as `into` to reuse its memory; allocating a large image costs more than the lookups.
 */
export function applyCompiledEffects(
	image: Rgba8Image,
	indices: Uint32Array,
	results: Uint32Array,
	into?: Uint8Array
): Rgba8Image {
	const { data } = image;
	if (indices.length * 4 !== data.length)
		throw new DitheretteError(
			'invalid-request',
			'indices',
			'Indices must list one entry per pixel.'
		);
	const output = into?.length === data.length ? into : new Uint8Array(data.length);
	for (let pixel = 0, offset = 0; pixel < indices.length; pixel++, offset += 4) {
		const result = results[indices[pixel]!]!;
		output[offset] = result & 0xff;
		output[offset + 1] = (result >>> 8) & 0xff;
		output[offset + 2] = result >>> 16;
		output[offset + 3] = data[offset + 3]!;
	}
	return { width: image.width, height: image.height, data: output };
}
