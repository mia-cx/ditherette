import { LUT_SIZE } from '$lib/processing/source-effects';
import { coordinates } from './colour';

/** How many source pixels the 3D scope plots at most. */
const SAMPLES = 150_000;
/** World units per Oklab a or b unit, so the sRGB gamut fills about the height of lightness. */
export const OKLAB_SPREAD = 3.4;

/** The effects table's colour for an sRGB colour, trilinear like the Source preview's GPU draw. */
function throughLut(lut: Uint8Array, r: number, g: number, b: number, out: number[]) {
	const scale = (LUT_SIZE - 1) / 255;
	const [x, y, z] = [r * scale, g * scale, b * scale];
	const [x0, y0, z0] = [
		Math.min(LUT_SIZE - 2, Math.floor(x)),
		Math.min(LUT_SIZE - 2, Math.floor(y)),
		Math.min(LUT_SIZE - 2, Math.floor(z))
	];
	const [fx, fy, fz] = [x - x0, y - y0, z - z0];
	for (let channel = 0; channel < 3; channel++) {
		let sum = 0;
		for (let corner = 0; corner < 8; corner++) {
			const [dx, dy, dz] = [corner & 1, (corner >> 1) & 1, (corner >> 2) & 1];
			const weight = (dx ? fx : 1 - fx) * (dy ? fy : 1 - fy) * (dz ? fz : 1 - fz);
			const index =
				((z0 + dz) * LUT_SIZE * LUT_SIZE + (y0 + dy) * LUT_SIZE + (x0 + dx)) * 4 + channel;
			sum += weight * lut[index]!;
		}
		out[channel] = sum;
	}
}

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
export function colourCloud(source: ImageData, lut: Uint8Array | undefined) {
	const pixels = source.width * source.height;
	const stride = Math.max(1, Math.floor(pixels / SAMPLES));
	const count = Math.floor(pixels / stride);
	const before = new Float32Array(count * 3);
	const after = new Float32Array(count * 3);
	const colours = new Float32Array(count * 4);
	const scratch = new Float64Array(3);
	const mapped = [0, 0, 0];
	for (let k = 0; k < count; k++) {
		const offset = k * stride * 4;
		const [r, g, b] = [source.data[offset]!, source.data[offset + 1]!, source.data[offset + 2]!];
		place(r, g, b, before, k * 3, scratch);
		if (lut) throughLut(lut, r, g, b, mapped);
		else [mapped[0], mapped[1], mapped[2]] = [r, g, b];
		const [ar, ag, ab] = mapped.map((value) => Math.round(value)) as [number, number, number];
		place(ar, ag, ab, after, k * 3, scratch);
		colours.set([ar / 255, ag / 255, ab / 255, 0.55], k * 4);
	}
	return { before, after, colours, count };
}
