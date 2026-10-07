import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	activePalette,
	colorSpace,
	ditherSettings,
	outputSettings,
	processedImage,
	shareExportData,
	sourceImageData,
	sourceMeta
} from '$lib/stores/app';
import { effectLayers } from '$lib/stores/effects';
import { selectedPalette } from '$lib/stores/app';
import { useTestStorageEngine } from '@nanostores/persistent';

vi.mock('./png', () => ({ downloadIndexedPng: vi.fn() }));
vi.mock('./live-effects', async (importOriginal) => ({
	...(await importOriginal<typeof import('./live-effects')>()),
	measurePaletteFits: vi.fn(async () => new Map())
}));

import { exportPng } from './export';
import { processingIdentityHash } from './hash';
import type { ProcessedImage } from './types';

const image = () => ({ width: 2, height: 2, data: new Uint8ClampedArray(16) }) as ImageData;

function exportableImage(): ProcessedImage {
	const settingsHash = processingIdentityHash({
		output: outputSettings.get(),
		dither: ditherSettings.get(),
		colorSpace: colorSpace.get(),
		effects: [],
		paletteName: activePalette.get().name,
		paletteSource: activePalette.get().source,
		palette: selectedPalette.get(),
		source: sourceMeta.get()
	});
	return {
		width: 2,
		height: 2,
		indices: new Uint8Array(4),
		palette: selectedPalette.get(),
		transparentIndex: 0,
		warnings: [],
		settingsHash,
		updatedAt: 0
	};
}

useTestStorageEngine();

beforeEach(() => {
	effectLayers.set([]);
	sourceImageData.set(image());
	processedImage.set(exportableImage());
});

afterEach(() => {
	vi.unstubAllGlobals();
});

describe('exportPng telemetry', () => {
	it('posts the v1 event after the download when sharing is on', async () => {
		shareExportData.set(true);
		const fetch = vi.fn(async () => new Response(null, { status: 204 }));
		vi.stubGlobal('fetch', fetch);
		exportPng();
		await vi.waitFor(() => expect(fetch).toHaveBeenCalledOnce());
		const [url, init] = fetch.mock.calls[0]! as unknown as [string, RequestInit];
		expect(url).toBe('/api/events');
		expect(init.method).toBe('POST');
		const event = JSON.parse(init.body as string);
		expect(event.version).toBe(1);
		expect(event.export).toEqual({ width: 2, height: 2, format: 'png' });
		expect(event.settings.output).not.toHaveProperty('crop');
	});

	it('sends nothing when sharing is off', async () => {
		shareExportData.set(false);
		const fetch = vi.fn(async () => new Response(null, { status: 204 }));
		vi.stubGlobal('fetch', fetch);
		exportPng();
		await new Promise((done) => setTimeout(done, 20));
		expect(fetch).not.toHaveBeenCalled();
	});
});
