import { coordinates } from './colour';
import { samplePixels } from './scopes';

/** How many source pixels the 3D scope plots at most. */
const SAMPLES = 150_000;
/** World units per Oklab a or b unit, so the sRGB gamut fills about the height of lightness. */
export const OKLAB_SPREAD = 3.4;

/** A colour's place in the scope: Oklab a across, lightness up, b deep. */
function place(
	r: number,
	g: number,
	b: number,
	into: Float32Array,
	at: number,
	scratch: Float64Array
) {
	coordinates('oklab', r, g, b, scratch);
	into[at] = (scratch[1]! - 0.5) * 0.8 * OKLAB_SPREAD;
	into[at + 1] = scratch[0]! - 0.5;
	into[at + 2] = (scratch[2]! - 0.5) * 0.8 * OKLAB_SPREAD;
}

/**
 * Up to 150,000 source pixels before and after the effects, as scope positions, with each pixel's
 * after colour. Without a table, after equals before.
 */
export function colourCloud(
	source: ImageData,
	effects: { pixels: Uint32Array; results: Uint32Array } | undefined
) {
	const { data, width, height } = source;
	// Each sampled pixel's colour after the effects: its colour index's result, packed.
	const effected = new Uint32Array(Math.min(width * height, SAMPLES));
	const { count, rgb } = samplePixels(width, height, SAMPLES, (pixel, rgb, at) => {
		const offset = pixel * 4;
		if (!data[offset + 3]) return false;
		rgb[at] = data[offset]!;
		rgb[at + 1] = data[offset + 1]!;
		rgb[at + 2] = data[offset + 2]!;
		effected[at / 3] = effects
			? effects.results[effects.pixels[pixel]! & 0xffffff]!
			: data[offset]! | (data[offset + 1]! << 8) | (data[offset + 2]! << 16);
		return true;
	});
	const before = new Float32Array(count * 3);
	const after = new Float32Array(count * 3);
	const colours = new Float32Array(count * 4);
	const scratch = new Float64Array(3);
	for (let k = 0; k < count; k++) {
		const offset = k * 3;
		place(rgb[offset]!, rgb[offset + 1]!, rgb[offset + 2]!, before, k * 3, scratch);
		const entry = effected[k]!;
		const [ar, ag, ab] = [entry & 0xff, (entry >>> 8) & 0xff, (entry >>> 16) & 0xff];
		place(ar, ag, ab, after, k * 3, scratch);
		colours.set([ar / 255, ag / 255, ab / 255, 0.55], k * 4);
	}
	return { before, after, colours, count };
}
