import { computed } from 'nanostores';
import {
	activePalette,
	colorSpace,
	ditherSettings,
	outputSettings,
	processedImage,
	processingProgress,
	selectedPalette,
	sourceMeta
} from '$lib/stores/app';
import { activeEffectSteps } from '$lib/stores/effects';
import { processingIdentityHash } from './hash';
import { downloadIndexedPng } from './png';
import type { ProcessedImage } from './types';

/** Identity of the settings on screen; the processed image matches it once processing catches up. */
const currentHash = computed(
	[
		outputSettings,
		ditherSettings,
		colorSpace,
		activeEffectSteps,
		activePalette,
		selectedPalette,
		sourceMeta
	],
	(output, dither, space, effects, palette, colors, source) =>
		processingIdentityHash({
			output,
			dither,
			colorSpace: space,
			effects,
			paletteName: palette.name,
			paletteSource: palette.source,
			palette: colors,
			source
		})
);

/** True when the processed image is finished and reflects the current settings. */
export const exportable = computed(
	[processedImage, processingProgress, currentHash],
	(image, progress, hash) => Boolean(image && !progress && image.settingsHash === hash)
);

/** Download the processed image as an indexed PNG named after the source. */
export function exportPng() {
	const image = processedImage.get();
	if (image && exportable.get()) downloadIndexedPng(image, exportFilename(image));
}

function exportFilename(image: ProcessedImage) {
	const original = sourceMeta.get()?.name.replace(/\.[^.]+$/, '') || 'image';
	const safeName =
		original
			.trim()
			.replace(/[^a-z0-9._-]+/gi, '-')
			.replace(/^-+|-+$/g, '') || 'image';
	return `${safeName}-ditherette-${image.width}x${image.height}.png`;
}
