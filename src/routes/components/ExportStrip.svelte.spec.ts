import { beforeEach, expect, it } from 'vitest';
import { page } from 'vitest/browser';
import { render } from 'vitest-browser-svelte';
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
import { activeEffectSteps, effectLayers } from '$lib/stores/effects';
import { processingIdentityHash } from '$lib/processing/hash';
import type { ProcessedImage } from '$lib/processing/types';
import ExportStrip from './ExportStrip.svelte';

function exportableImage(indices: number[]): ProcessedImage {
	const settingsHash = processingIdentityHash({
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		effects: activeEffectSteps.get(),
		paletteName: activePalette.get().name,
		paletteSource: activePalette.get().source,
		palette: selectedPalette.get(),
		source: sourceMeta.get()
	});
	const palette = selectedPalette.get();
	return {
		width: indices.length,
		height: 1,
		indices: new Uint8Array(indices),
		palette,
		transparentIndex: palette.findIndex((color) => color.kind === 'transparent'),
		warnings: [],
		settingsHash,
		updatedAt: 0
	};
}

beforeEach(() => {
	effectLayers.set([]);
	sourceMeta.set({ name: 'test.png', width: 4, height: 1, type: 'image/png', updatedAt: 1 });
	processingProgress.set(undefined);
	const transparent = selectedPalette.get().findIndex((color) => color.kind === 'transparent');
	processedImage.set(exportableImage([0, 0, 0, transparent]));
});

it('shows the pixels to paint, excluding Transparent, and a dash without an output', async () => {
	render(ExportStrip, { hasImage: true });
	await expect.element(page.getByText('3 px to paint')).toBeInTheDocument();
	processedImage.set(undefined);
	await expect.element(page.getByText('— px to paint')).toBeInTheDocument();
});
