import { afterEach, beforeEach, expect, it, vi } from 'vitest';
import {
	clearAllImageData,
	isSourceSuperseded,
	restorePersistedImages,
	setSourceFile
} from './source';
import { decodeBlob } from './image-decode';
import {
	loadProcessedImage,
	loadSourceImage,
	clearPersistedProcessedImage,
	clearPersistedImages
} from './db';
import { scheduleProcessing } from './client';
import { processedImage, processingError, sourceImageData, sourceMeta } from '$lib/stores/app';
import type { DecodedImage } from './image-decode';
import type { ProcessedImage, SourceImageRecord } from './types';

vi.mock('./db', () => ({
	loadSourceImage: vi.fn(),
	loadProcessedImage: vi.fn(),
	clearPersistedImages: vi.fn(async () => undefined),
	clearPersistedImagesIfSourceMatches: vi.fn(async () => undefined),
	clearPersistedProcessedImage: vi.fn(async () => undefined),
	saveSourceImageAndClearProcessed: vi.fn(async () => undefined),
	sourceMetaFromRecord: (record: SourceImageRecord) => ({
		name: record.name,
		width: record.width,
		height: record.height,
		type: record.type,
		updatedAt: record.updatedAt
	})
}));
vi.mock('./client', () => ({
	cancelProcessing: vi.fn(),
	currentSettingsHash: () => 'current',
	scheduleProcessing: vi.fn()
}));
vi.mock('./image-decode', () => ({ decodeBlob: vi.fn() }));
vi.mock('./image-metadata', () => ({ validateSourceBlob: vi.fn(async () => undefined) }));

function decoded(red: number): DecodedImage {
	return {
		width: 1,
		height: 1,
		imageData: new ImageData(new Uint8ClampedArray([red, 0, 0, 255]), 1, 1)
	};
}

function source(): SourceImageRecord {
	return {
		blob: new Blob(['old']),
		name: 'old.png',
		width: 1,
		height: 1,
		type: 'image/png',
		updatedAt: 1
	};
}

beforeEach(async () => {
	vi.stubGlobal(
		'ImageData',
		class {
			constructor(
				public data: Uint8ClampedArray,
				public width: number,
				public height: number
			) {}
		}
	);
	await clearAllImageData();
	vi.clearAllMocks();
	vi.mocked(loadSourceImage).mockResolvedValue(source());
	vi.mocked(decodeBlob).mockResolvedValue(decoded(1));
});
afterEach(() => vi.unstubAllGlobals());

it('does not republish or clear saved data when a restore finishes after a new upload', async () => {
	const saved = Promise.withResolvers<ProcessedImage | undefined>();
	vi.mocked(loadProcessedImage).mockReturnValue(saved.promise);
	const restoring = restorePersistedImages().catch((error: unknown) => error);
	await vi.waitFor(() => expect(loadProcessedImage).toHaveBeenCalledOnce());
	const replacement = decoded(200);
	vi.mocked(decodeBlob).mockResolvedValue(replacement);
	await setSourceFile(new File(['new'], 'new.png', { type: 'image/png' }));
	const freshMeta = sourceMeta.get();
	vi.mocked(scheduleProcessing).mockClear();
	saved.resolve(undefined);
	expect(isSourceSuperseded(await restoring)).toBe(true);
	expect(sourceMeta.get()).toBe(freshMeta);
	expect(sourceImageData.get()).toBe(replacement.imageData);
	expect(clearPersistedProcessedImage).not.toHaveBeenCalled();
	expect(scheduleProcessing).not.toHaveBeenCalled();
	expect(processingError.get()).toBeUndefined();
});

it('regenerates output from the source when no processed payload was admitted', async () => {
	vi.mocked(loadProcessedImage).mockResolvedValue(undefined);
	await restorePersistedImages();
	expect(sourceMeta.get()?.name).toBe('old.png');
	expect(sourceImageData.get()?.data[0]).toBe(1);
	expect(processedImage.get()).toBeUndefined();
	expect(scheduleProcessing).toHaveBeenCalledExactlyOnceWith(0, expect.anything());
	expect(clearPersistedImages).not.toHaveBeenCalled();
});

it('hands the upload validation, decode and save to the processing it schedules', async () => {
	const startedAt = performance.now();
	await setSourceFile(new File(['new'], 'new.png', { type: 'image/png' }));
	expect(scheduleProcessing).toHaveBeenLastCalledWith(0, {
		startedAt: expect.closeTo(startedAt, -1),
		timings: [
			{ name: 'main source validation', ms: expect.any(Number) },
			{ name: 'main source decode', ms: expect.any(Number) },
			{ name: 'main source save', ms: expect.any(Number) },
			{ name: 'main source publish', ms: expect.any(Number) }
		]
	});
});

it.each(['source read', 'decode', 'processed read'])(
	'ignores a late %s failure after an upload without clearing its state or persistence',
	async (phase) => {
		const failure = Promise.withResolvers<never>();
		if (phase === 'source read') vi.mocked(loadSourceImage).mockReturnValueOnce(failure.promise);
		else if (phase === 'decode') vi.mocked(decodeBlob).mockReturnValueOnce(failure.promise);
		else vi.mocked(loadProcessedImage).mockReturnValueOnce(failure.promise);
		const restoring = restorePersistedImages().catch((error: unknown) => error);
		if (phase === 'decode') await vi.waitFor(() => expect(decodeBlob).toHaveBeenCalledOnce());
		if (phase === 'processed read')
			await vi.waitFor(() => expect(loadProcessedImage).toHaveBeenCalledOnce());
		const replacement = decoded(200);
		vi.mocked(decodeBlob).mockResolvedValue(replacement);
		await setSourceFile(new File(['new'], 'new.png', { type: 'image/png' }));
		failure.reject(new Error('late failure'));
		expect(isSourceSuperseded(await restoring)).toBe(true);
		expect(sourceImageData.get()).toBe(replacement.imageData);
		expect(sourceMeta.get()?.name).toBe('new.png');
		expect(clearPersistedImages).not.toHaveBeenCalled();
		expect(clearPersistedProcessedImage).not.toHaveBeenCalled();
		expect(processingError.get()).toBeUndefined();
	}
);
