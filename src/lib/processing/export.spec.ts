import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	activePalette,
	colorSpace,
	ditherSettings,
	outputSettings,
	processedImage,
	processingProgress,
	shareExportData,
	sourceImageData,
	sourceMeta
} from '$lib/stores/app';
import { activeEffectSteps, addEffect, effectLayers, updateEffect } from '$lib/stores/effects';
import { selectedPalette } from '$lib/stores/app';
import { useTestStorageEngine } from '@nanostores/persistent';

vi.mock('./png', () => ({ downloadPngBlob: vi.fn() }));
vi.mock('./png-worker', () => ({ encodeIndexedPngOffThread: vi.fn() }));
vi.mock('./live-effects', async (importOriginal) => ({
	...(await importOriginal<typeof import('./live-effects')>()),
	measurePaletteFits: vi.fn(async () => new Map())
}));

import { cancelPngExport, exportable, exportPng } from './export';
import { downloadPngBlob } from './png';
import { encodeIndexedPngOffThread } from './png-worker';
import { processingIdentityHash } from './hash';
import { measurePaletteFits } from './live-effects';
import type { ProcessedImage } from './types';

const image = () => ({ width: 2, height: 2, data: new Uint8ClampedArray(16) }) as ImageData;

function exportableImage(): ProcessedImage {
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
	vi.clearAllMocks();
	vi.stubGlobal('window', new EventTarget());
	vi.mocked(encodeIndexedPngOffThread)
		.mockReset()
		.mockResolvedValue(new Blob(['PNG'], { type: 'image/png' }));
	shareExportData.set(false);
	processingProgress.set(undefined);
	effectLayers.set([]);
	sourceImageData.set(image());
	processedImage.set(exportableImage());
});

afterEach(async () => {
	cancelPngExport();
	await Promise.resolve();
	await Promise.resolve();
	vi.restoreAllMocks();
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

it('posts one snapshot of the state at export time', async () => {
	shareExportData.set(true);
	const layer = addEffect('palette-fit');
	const fetch = vi.fn(async () => new Response(null, { status: 204 }));
	vi.stubGlobal('fetch', fetch);
	let release: ((value: ReadonlyMap<number, never>) => void) | undefined;
	vi.mocked(measurePaletteFits).mockReturnValueOnce(
		new Promise<ReadonlyMap<number, never>>((resolve) => {
			release = resolve;
		})
	);
	// The export's hash must match the image's; rebuild the image under the fit step.
	processedImage.set(exportableImage());
	exportPng();
	await vi.waitFor(() => expect(downloadPngBlob).toHaveBeenCalledOnce());

	// A look edit mid-measurement must not reach the posted event.
	const step = effectLayers.get().find((item) => item.id === layer.id)!.step;
	if (step.effect !== 'palette-fit') throw new Error('Expected a palette-fit layer.');
	updateEffect(layer.id, { ...step, look: 'vivid' });
	release!(new Map<number, never>());
	await vi.waitFor(() => expect(fetch).toHaveBeenCalledOnce());
	const [, init] = fetch.mock.calls[0]! as unknown as [string, RequestInit];
	const event = JSON.parse(init.body as string);
	expect(event.settings.effects[0].look).toBe('fitted');
	expect(event.fits[0].look).toBe('fitted');
});

function pendingEncode() {
	let resolve!: (blob: Blob) => void;
	const pending = new Promise<Blob>((done) => {
		resolve = done;
	});
	vi.mocked(encodeIndexedPngOffThread).mockReturnValueOnce(pending);
	return () => resolve(new Blob(['PNG'], { type: 'image/png' }));
}

describe('export lifecycle', () => {
	it('waits for encoding and preserves the source filename', async () => {
		sourceMeta.set({
			name: 'A nice image.jpg',
			width: 2,
			height: 2,
			type: 'image/jpeg',
			updatedAt: 1
		});
		processedImage.set(exportableImage());
		const complete = pendingEncode();
		exportPng();
		expect(downloadPngBlob).not.toHaveBeenCalled();
		complete();
		await vi.waitFor(() =>
			expect(downloadPngBlob).toHaveBeenCalledWith(
				expect.any(Blob),
				'A-nice-image-ditherette-2x2.png'
			)
		);
	});

	it.each(['result', 'source', 'settings', 'processing', 'cancel', 'pagehide', 'unmount'] as const)(
		'cancels on %s and rejects a late completion without telemetry',
		async (change) => {
			shareExportData.set(true);
			const fetch = vi.fn();
			vi.stubGlobal('fetch', fetch);
			const complete = pendingEncode();
			const unsubscribe = exportable.listen(() => {});
			exportPng();
			const signal = vi.mocked(encodeIndexedPngOffThread).mock.calls[0][1]!;
			if (change === 'result') processedImage.set({ ...exportableImage() });
			if (change === 'source') sourceImageData.set(image());
			if (change === 'settings')
				outputSettings.set({ ...outputSettings.get(), width: outputSettings.get().width + 1 });
			if (change === 'processing') processingProgress.set({ stage: 'quantize', progress: 1 });
			if (change === 'cancel') cancelPngExport();
			if (change === 'pagehide') window.dispatchEvent(new Event('pagehide'));
			if (change === 'unmount') unsubscribe();
			expect(signal.aborted).toBe(true);
			complete();
			await new Promise((done) => setTimeout(done, 0));
			expect(downloadPngBlob).not.toHaveBeenCalled();
			expect(fetch).not.toHaveBeenCalled();
			unsubscribe();
		}
	);

	it('a second click supersedes the first export', async () => {
		const first = pendingEncode();
		exportPng();
		const firstSignal = vi.mocked(encodeIndexedPngOffThread).mock.calls[0][1]!;
		const second = pendingEncode();
		exportPng();
		expect(firstSignal.aborted).toBe(true);
		second();
		await vi.waitFor(() => expect(downloadPngBlob).toHaveBeenCalledOnce());
		first();
		await new Promise((done) => setTimeout(done, 0));
		expect(downloadPngBlob).toHaveBeenCalledOnce();
	});

	it('worker failure publishes no file or telemetry and allows retry', async () => {
		shareExportData.set(true);
		const fetch = vi.fn(async () => new Response(null, { status: 204 }));
		vi.stubGlobal('fetch', fetch);
		const error = new Error('Worker unavailable');
		const log = vi.spyOn(console, 'error').mockImplementation(() => {});
		vi.mocked(encodeIndexedPngOffThread).mockRejectedValueOnce(error);
		exportPng();
		await vi.waitFor(() => expect(log).toHaveBeenCalledWith('PNG export failed.', error));
		expect(downloadPngBlob).not.toHaveBeenCalled();
		expect(fetch).not.toHaveBeenCalled();
		exportPng();
		await vi.waitFor(() => expect(downloadPngBlob).toHaveBeenCalledOnce());
		await vi.waitFor(() => expect(fetch).toHaveBeenCalledOnce());
	});
});
