import type { Ditherette, Effect, EffectContext, Rgba8Image } from 'ditherette';

/**
 * Effects are pointwise and round to RGBA8 once, before resize, so the whole chain is a function
 * from an input colour to an output colour. The effects table holds that function as RGBA8
 * entries, one per colour: entry `r + 256 g + 65536 b`, which is also the texel of a 256³ 3D
 * texture with red fastest. Only colours the source contains are filled, so building it runs the
 * chain once per distinct colour instead of once per pixel, and the table is exact for that source.
 */
export const TABLE_SIDE = 256;
export const TABLE_ENTRIES = TABLE_SIDE ** 3;

/** A colour's table entry: its RGB bytes as one little-endian RGBA word, without alpha. */
const RGB_MASK = 0x00ffffff;
const ALPHA_MASK = 0xff000000;
/** Rows of the image the distinct colours are laid out in for the package. */
const COLOUR_ROW = 4096;

/**
 * 32-bit views of RGBA8 bytes, which read a pixel as one word. Every browser engine is
 * little-endian, so a word is `r | g << 8 | b << 16 | a << 24`.
 */
function words(bytes: Uint8Array | Uint8ClampedArray) {
	const aligned = bytes.byteOffset % 4 ? bytes.slice() : bytes;
	return new Uint32Array(aligned.buffer, aligned.byteOffset, aligned.length / 4);
}

/** Every distinct RGB in `source`, as table entries, in first-seen order. Alpha doesn't matter. */
export function distinctColours(source: Pick<ImageData, 'data'>): Uint32Array {
	const pixels = words(source.data);
	const seen = new Uint8Array(TABLE_ENTRIES / 8);
	const colours = new Uint32Array(Math.min(pixels.length, TABLE_ENTRIES));
	let count = 0;
	for (const pixel of pixels) {
		const entry = pixel & RGB_MASK;
		const bit = 1 << (entry & 7);
		if (seen[entry >>> 3]! & bit) continue;
		seen[entry >>> 3]! |= bit;
		colours[count++] = entry;
	}
	return colours.slice(0, count);
}

/**
 * Give each recipe-less palette fit the recipe it derives from `source` with the steps before it,
 * as `process` does, so the chain becomes pointwise and the same for every colour.
 */
export function resolveRecolour(
	ditherette: Ditherette,
	source: Rgba8Image,
	effects: readonly Effect[],
	context: Required<EffectContext>
): Effect[] {
	return effects.map((step, index) =>
		step.effect === 'recolour' && step.recipe === null
			? {
					...step,
					recipe: ditherette.analyzeRecolour({
						version: 1,
						source,
						effects: effects.slice(0, index),
						context
					})
				}
			: step
	);
}

/**
 * The effects table for `colours` under a pointwise `effects` chain: every listed entry holds its
 * output colour with full alpha; the rest stay zero. Resolve palette fit first.
 */
export function compileEffectsTable(
	ditherette: Ditherette,
	colours: Uint32Array,
	effects: readonly Effect[],
	context: Required<EffectContext>
): Uint32Array {
	const table = new Uint32Array(TABLE_ENTRIES);
	if (!colours.length) return table;
	const width = Math.min(colours.length, COLOUR_ROW);
	const height = Math.ceil(colours.length / width);
	// Pad the last row with the last colour; padding outputs are ignored.
	const input = new Uint32Array(width * height).fill(colours.at(-1)! | ALPHA_MASK);
	for (let index = 0; index < colours.length; index++) input[index] = colours[index]! | ALPHA_MASK;
	const { data } = ditherette.applyEffects({
		version: 1,
		source: { width, height, data: new Uint8Array(input.buffer) },
		effects,
		context
	});
	const output = words(data);
	for (let index = 0; index < colours.length; index++)
		table[colours[index]!] = output[index]! | ALPHA_MASK;
	return table;
}

/** `source` with every pixel's RGB looked up in `table`, keeping its alpha. */
export function applyEffectsTable(
	source: Pick<ImageData, 'width' | 'height' | 'data'>,
	table: Uint32Array
): Pick<ImageData, 'width' | 'height' | 'data'> {
	const data = new Uint8ClampedArray(source.data.length);
	const pixels = words(source.data);
	const mapped = new Uint32Array(data.buffer);
	for (let index = 0; index < pixels.length; index++) {
		const pixel = pixels[index]!;
		mapped[index] = (table[pixel & RGB_MASK]! & RGB_MASK) | (pixel & ALPHA_MASK);
	}
	return { width: source.width, height: source.height, data };
}
