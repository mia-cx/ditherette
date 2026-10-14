import { computed } from 'nanostores';
import { processedImage } from '$lib/stores/app';
import { exportable } from './export';
import type { ProcessedImage } from './types';

/** Pixels per palette colour in one processed image. */
export type PixelCounts = {
	/** Pixels per palette colour key; enabled colours the output never uses count 0. */
	byKey: ReadonlyMap<string, number>;
	/** Every pixel except Transparent: what painting the output takes. */
	painted: number;
};

/** Count each palette entry's pixels in one pass over the indices. */
export function countPalettePixels({
	indices,
	palette,
	transparentIndex
}: Pick<ProcessedImage, 'indices' | 'palette' | 'transparentIndex'>): PixelCounts {
	const tally = new Uint32Array(palette.length);
	for (let i = 0; i < indices.length; i++) tally[indices[i]!]!++;
	const byKey = new Map<string, number>();
	let transparent = 0;
	palette.forEach((color, index) => {
		byKey.set(color.key, tally[index]!);
		if (index === transparentIndex || color.kind === 'transparent') transparent += tally[index]!;
	});
	return { byKey, painted: indices.length - transparent };
}

// One scan per processed image; readiness changes reuse it.
const imageCounts = computed(processedImage, (image) => image && countPalettePixels(image));

/** Counts for the finished output; undefined while it is missing or out of date with the settings. */
export const pixelCounts = computed([imageCounts, exportable], (counts, ready) =>
	ready ? counts : undefined
);
