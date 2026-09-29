import type { Ditherette, TwoInputCurve } from 'ditherette';
import { initializePackageProcessor } from '$lib/processing/worker-pipeline';
import { channelValue } from './pick';

/** Lattice steps per sRGB channel: enough colours to fill most cells of the editor's backdrop. */
const STEPS = 24;

/** Every lattice colour once, as a tiny image the package can run the curve over. */
const LATTICE = (() => {
	const step = 255 / (STEPS - 1);
	const data = new Uint8Array(STEPS ** 3 * 4);
	let offset = 0;
	for (let blue = 0; blue < STEPS; blue++)
		for (let green = 0; green < STEPS; green++)
			for (let red = 0; red < STEPS; red++, offset += 4)
				data.set(
					[Math.round(red * step), Math.round(green * step), Math.round(blue * step), 255],
					offset
				);
	return { width: STEPS, height: STEPS * STEPS, data };
})();

let processor: Promise<Ditherette> | undefined;

/**
 * What a two-input curve does, as a `width` × `height` image over its inputs: `x` across, `x2`
 * up. Each cell shows its most saturated lattice colour after the real curve runs, so the editor
 * shows exact results in every colour model.
 */
export async function gridBackdrop(curve: TwoInputCurve, width: number, height: number) {
	processor ??= initializePackageProcessor().catch((error: unknown) => {
		processor = undefined;
		throw error;
	});
	const ditherette = await processor;
	const { data: after } = ditherette.applyEffects({
		version: 1,
		source: LATTICE,
		effects: [{ effect: 'curves', enabled: true, curves: [curve] }]
	});
	const best = new Int32Array(width * height).fill(-1);
	const score = new Float32Array(width * height).fill(-1);
	const { data } = LATTICE;
	for (let offset = 0; offset < data.length; offset += 4) {
		const rgb = [data[offset]!, data[offset + 1]!, data[offset + 2]!] as const;
		const column = Math.min(width - 1, Math.floor(channelValue(curve.x, rgb) * width));
		const row = Math.min(height - 1, Math.floor((1 - channelValue(curve.x2, rgb)) * height));
		const cell = row * width + column;
		const saturation = Math.max(...rgb) - Math.min(...rgb);
		if (saturation > score[cell]!) {
			score[cell] = saturation;
			best[cell] = offset;
		}
	}
	// Cells no lattice colour reaches, like saturated near-whites, borrow the nearest filled cell
	// in their column, so the backdrop has no holes.
	const image = new ImageData(width, height);
	for (let column = 0; column < width; column++)
		for (let row = 0; row < height; row++) {
			let source = -1;
			for (let reach = 0; source < 0 && reach < height; reach++)
				for (const candidate of [row - reach, row + reach])
					if (source < 0 && candidate >= 0 && candidate < height)
						source = best[candidate * width + column]!;
			if (source < 0) continue;
			image.data.set(
				[after[source]!, after[source + 1]!, after[source + 2]!, 255],
				(row * width + column) * 4
			);
		}
	return image;
}
