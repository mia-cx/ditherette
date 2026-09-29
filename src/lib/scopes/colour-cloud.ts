import { coordinates } from './colour';
import { sampleSource } from './scopes';

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
export function colourCloud(source: ImageData, table: Uint32Array | undefined) {
	const { count, rgb } = sampleSource(source, SAMPLES);
	const before = new Float32Array(count * 3);
	const after = new Float32Array(count * 3);
	const colours = new Float32Array(count * 4);
	const scratch = new Float64Array(3);
	for (let k = 0; k < count; k++) {
		const offset = k * 3;
		const [r, g, b] = [rgb[offset]!, rgb[offset + 1]!, rgb[offset + 2]!];
		place(r, g, b, before, k * 3, scratch);
		// The pipeline's effects table is exact: one entry per colour, `r | g << 8 | b << 16`.
		const key = r | (g << 8) | (b << 16);
		const entry = table ? table[key]! : key;
		const [ar, ag, ab] = [entry & 0xff, (entry >>> 8) & 0xff, (entry >>> 16) & 0xff];
		place(ar, ag, ab, after, k * 3, scratch);
		colours.set([ar / 255, ag / 255, ab / 255, 0.55], k * 4);
	}
	return { before, after, colours, count };
}
