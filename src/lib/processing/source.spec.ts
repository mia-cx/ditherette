import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
	colorSpace,
	ditherSettings,
	outputSettings,
	processedImage,
	processingError,
	sourceImageData,
	sourceMeta,
	sourceObjectUrl
} from '$lib/stores/app';
import { effectLayers } from '$lib/stores/effects';
import { cancelProcessing, currentSettingsHash } from './client';
import {
	clearPersistedImages,
	clearPersistedProcessedImage,
	loadProcessedImage,
	loadSourceImage,
	saveSourceImageAndClearProcessed,
	sourceMetaFromRecord
} from './db';
import { settingsHash, type JsonValue } from './hash';
import { decodeBlob } from './image-decode';
import { restorePersistedImages } from './source';
import type { ProcessedImage, SourceImageRecord, WorkerRequest } from './types';

vi.mock('./db', async (importOriginal) => ({
	...(await importOriginal<typeof import('./db')>()),
	loadSourceImage: vi.fn(),
	loadProcessedImage: vi.fn(),
	clearPersistedImages: vi.fn(async () => undefined),
	clearPersistedProcessedImage: vi.fn(async () => undefined),
	saveSourceImageAndClearProcessed: vi.fn(async () => undefined),
	saveProcessedImage: vi.fn(async () => undefined)
}));

vi.mock('./image-decode', () => ({ decodeBlob: vi.fn() }));

// Keep restore, stores, and client scheduling real; mock browser transport and persistence.
class ControlledWorker {
	static instances: ControlledWorker[] = [];
	onmessage?: (event: MessageEvent<unknown>) => void;
	messages: WorkerRequest[] = [];
	terminate = vi.fn();
	constructor() {
		ControlledWorker.instances.push(this);
	}
	postMessage(message: WorkerRequest) {
		this.messages.push(message);
	}
}

beforeEach(() => {
	vi.useFakeTimers();
	vi.clearAllMocks();
	vi.stubGlobal('Worker', ControlledWorker);
	ControlledWorker.instances = [];
	processedImage.set(undefined);
	processingError.set(undefined);
	sourceImageData.set(undefined);
	sourceMeta.set(undefined);
	sourceObjectUrl.set(undefined);
	effectLayers.set([]);
});

afterEach(() => {
	cancelProcessing();
	const url = sourceObjectUrl.get();
	if (url) URL.revokeObjectURL(url);
	vi.useRealTimers();
	vi.unstubAllGlobals();
});

describe('restorePersistedImages', () => {
	it('rejects an old unversioned output and reprocesses without losing source or settings', async () => {
		const savedSource: SourceImageRecord = {
			blob: new Blob(['saved source'], { type: 'image/png' }),
			name: 'source.png',
			width: 2,
			height: 2,
			type: 'image/png',
			updatedAt: 1
		};
		const decoded: ImageData = {
			width: 2,
			height: 2,
			data: new Uint8ClampedArray(16),
			colorSpace: 'srgb'
		};
		outputSettings.set({
			...outputSettings.get(),
			width: 1,
			height: 1,
			resize: 'nearest',
			crop: { x: 0, y: 0, width: 1, height: 1 }
		});
		ditherSettings.set({ ...ditherSettings.get(), algorithm: 'floyd-steinberg', seed: 123 });
		colorSpace.set('srgb');
		const savedOutputSettings = outputSettings.get();
		const savedDitherSettings = ditherSettings.get();
		sourceMeta.set(sourceMetaFromRecord(savedSource));
		const currentHash = currentSettingsHash();
		const oldIdentity = JSON.parse(currentHash) as Record<string, JsonValue>;
		delete oldIdentity.version;
		const savedOutput: ProcessedImage = {
			width: 1,
			height: 1,
			indices: new Uint8Array([0]),
			palette: [],
			transparentIndex: -1,
			warnings: [],
			settingsHash: settingsHash(oldIdentity),
			updatedAt: 1
		};
		sourceMeta.set(undefined);
		vi.mocked(loadSourceImage).mockResolvedValue(savedSource);
		vi.mocked(loadProcessedImage).mockResolvedValue(savedOutput);
		vi.mocked(decodeBlob).mockResolvedValue({ width: 2, height: 2, imageData: decoded });

		await restorePersistedImages();

		expect(processedImage.get()).toBeUndefined();
		expect(clearPersistedProcessedImage).toHaveBeenCalledOnce();
		expect(clearPersistedImages).not.toHaveBeenCalled();
		expect(saveSourceImageAndClearProcessed).not.toHaveBeenCalled();
		expect(decodeBlob).toHaveBeenCalledWith(savedSource.blob);
		expect(sourceMeta.get()).toEqual(sourceMetaFromRecord(savedSource));
		expect(sourceObjectUrl.get()).toBeDefined();
		expect(sourceImageData.get()).toBe(decoded);
		expect(outputSettings.get()).toBe(savedOutputSettings);
		expect(ditherSettings.get()).toBe(savedDitherSettings);
		expect(colorSpace.get()).toBe('srgb');
		expect(currentSettingsHash()).toBe(currentHash);
		expect(currentHash).not.toBe(savedOutput.settingsHash);
		expect(processingError.get()).toBeUndefined();

		await vi.advanceTimersByTimeAsync(0);
		expect(ControlledWorker.instances).toHaveLength(1);
		const worker = ControlledWorker.instances[0];
		const load = worker.messages[0];
		expect(load).toMatchObject({ type: 'load-source', source: decoded });
		if (load.type !== 'load-source') throw new Error('Expected source load.');
		worker.onmessage?.({
			data: { id: load.id, type: 'source-loaded', sourceId: load.sourceId }
		} as MessageEvent<unknown>);
		expect(worker.messages.at(-1)).toMatchObject({
			type: 'process',
			settingsHash: currentHash,
			settings: {
				output: savedOutputSettings,
				dither: savedDitherSettings,
				colorSpace: 'srgb'
			}
		});
	});
});
