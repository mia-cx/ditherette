import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	activePalette,
	colorSpace,
	ditherSettings,
	outputSettings,
	processedImage,
	processingProgress,
	selectedPalette,
	setPaletteColorEnabled,
	sourceMeta
} from '$lib/stores/app';
import { activeEffectSteps, effectLayers } from '$lib/stores/effects';
import { useTestStorageEngine } from '@nanostores/persistent';
import type { SourceMeta } from '$lib/stores/app';
import type { PaletteColor, ProcessedImage } from './types';

vi.mock('./png', () => ({ downloadIndexedPng: vi.fn() }));
vi.mock('./live-effects', async (importOriginal) => ({
	...(await importOriginal<typeof import('./live-effects')>()),
	measurePaletteFits: vi.fn(async () => new Map())
}));

import { countPalettePixels, pixelCounts } from './pixel-counts';
import { processingIdentityHash } from './hash';

const SOURCE: SourceMeta = {
	name: 'source.png',
	width: 4,
	height: 1,
	type: 'image/png',
	updatedAt: 0
};

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

useTestStorageEngine();

let unsubscribe: () => void;

beforeEach(() => {
	effectLayers.set([]);
	sourceMeta.set(SOURCE);
	processingProgress.set(undefined);
	unsubscribe = pixelCounts.listen(() => {});
});

afterEach(() => {
	unsubscribe();
});

const color = (name: string, kind: PaletteColor['kind'] = 'free'): PaletteColor => ({
	name,
	key: name.toLowerCase(),
	kind
});

describe('countPalettePixels', () => {
	it('counts each colour and leaves Transparent out of the painted total', () => {
		const palette = [color('Black'), color('White'), color('Grey'), color('Transparent', 'transparent')];
		const counts = countPalettePixels({
			indices: new Uint8Array([0, 0, 1, 3, 3, 3]),
			palette,
			transparentIndex: 3
		});
		expect(counts.byKey.get('black')).toBe(2);
		expect(counts.byKey.get('white')).toBe(1);
		expect(counts.byKey.get('grey')).toBe(0);
		expect(counts.byKey.get('transparent')).toBe(3);
		expect(counts.painted).toBe(3);
	});
});

describe('pixelCounts', () => {
	it('follows the output, never rescanning while readiness toggles', () => {
		const palette = selectedPalette.get();
		const transparent = palette.findIndex((entry) => entry.kind === 'transparent');
		processedImage.set(exportableImage([0, 0, 0, transparent]));

		const counts = pixelCounts.get();
		expect(counts?.byKey.get(palette[0]!.key)).toBe(3);
		expect(counts?.painted).toBe(3);

		// In-flight processing hides counts; recovering returns the same scan.
		processingProgress.set({ stage: 'Matching', progress: 0.5 });
		expect(pixelCounts.get()).toBeUndefined();
		processingProgress.set(undefined);
		expect(pixelCounts.get()).toBe(counts);

		// New settings make the image stale until a matching output lands.
		outputSettings.set({ ...outputSettings.get(), width: outputSettings.get().width + 1 });
		expect(pixelCounts.get()).toBeUndefined();
		processedImage.set(exportableImage([0, transparent, transparent, transparent]));
		expect(pixelCounts.get()?.byKey.get(palette[0]!.key)).toBe(1);
		expect(pixelCounts.get()?.painted).toBe(1);

		// Palette membership is part of the identity too.
		setPaletteColorEnabled(palette[1]!.key, false);
		expect(pixelCounts.get()).toBeUndefined();

		// A changed source never shows the old output's counts.
		sourceMeta.set({ ...SOURCE, name: 'other.png' });
		expect(pixelCounts.get()).toBeUndefined();

		processedImage.set(undefined);
		expect(pixelCounts.get()).toBeUndefined();
	});
});
