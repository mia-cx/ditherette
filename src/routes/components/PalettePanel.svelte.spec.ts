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
import PalettePanel from './PalettePanel.svelte';

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

const pixelCell = (colourName: string) =>
	page.getByRole('row').filter({ hasText: colourName }).getByRole('cell').nth(5);

it('shows per-colour pixel counts and a dash while the output is out of date', async () => {
	render(PalettePanel);
	await expect
		.element(page.getByText('Pixels', { exact: true }), { timeout: 10_000 })
		.toBeInTheDocument();

	// The first Wplace colour covers 3 of the 4 output pixels; Transparent covers the last.
	const first = selectedPalette.get()[0]!;
	await expect.element(pixelCell(first.name)).toHaveTextContent('3');
	await expect.element(pixelCell('Transparent')).toHaveTextContent('1');
	const other = selectedPalette
		.get()
		.find((color) => color.name !== first.name && color.kind !== 'transparent')!;
	await expect.element(pixelCell(other.name)).toHaveTextContent('0');

	processingProgress.set({ stage: 'Matching', progress: 0.5 });
	await expect.element(pixelCell(first.name)).toHaveTextContent('—');
	await expect.element(pixelCell('Transparent')).toHaveTextContent('—');
	await expect.element(pixelCell(other.name)).toHaveTextContent('—');
});
