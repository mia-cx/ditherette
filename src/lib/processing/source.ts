import {
	clearInMemoryImageState,
	outputSettings,
	processedImage,
	processingError,
	sourceImageData,
	sourceMeta,
	sourceObjectUrl,
	updateOutputSettings
} from '$lib/stores/app';
import {
	clearPersistedImages,
	clearPersistedProcessedImage,
	loadProcessedImage,
	loadSourceImage,
	saveSourceImage,
	sourceMetaFromRecord
} from './db';
import { decodeBlob } from './image-decode';
import { cancelProcessing, currentSettingsHash, scheduleProcessing } from './client';
import { fitOutputSizeToBounds } from './types';
import { validateSourceBlob } from './image-metadata';
import type { SourceImageRecord } from './types';

export const MIN_SCALE_FACTOR = 0.05;
let sourceGeneration = 0;

class SourceSuperseded extends Error {
	constructor() {
		super('Source image was superseded by a newer upload.');
		this.name = 'SourceSuperseded';
	}
}

export function isSourceSuperseded(error: unknown) {
	return error instanceof SourceSuperseded;
}

function errorMessage(error: unknown, fallback: string) {
	return error instanceof Error ? error.message : fallback;
}

export async function setSourceFile(file: File) {
	const generation = ++sourceGeneration;
	cancelProcessing();
	await validateSourceBlob(file);
	if (generation !== sourceGeneration) throw new SourceSuperseded();
	const decoded = await decodeBlob(file, { validate: false });
	if (generation !== sourceGeneration) throw new SourceSuperseded();
	const record: SourceImageRecord = {
		blob: file,
		name: file.name,
		width: decoded.width,
		height: decoded.height,
		type: file.type,
		updatedAt: Date.now()
	};
	await saveSourceImage(record);
	if (generation !== sourceGeneration) throw new SourceSuperseded();
	await clearPersistedProcessedImage();
	if (generation !== sourceGeneration) throw new SourceSuperseded();
	setSourceRecord(record, decoded.imageData);
	const settings = outputSettings.get();
	const scaleFactor = Math.min(1, Math.max(MIN_SCALE_FACTOR, settings.scaleFactor ?? 1));
	const size = fitOutputSizeToBounds(
		Math.max(1, Math.round(decoded.width * scaleFactor)),
		Math.max(1, Math.round(decoded.height * scaleFactor))
	);
	updateOutputSettings({
		...settings,
		width: size.width,
		height: size.height,
		lockAspect: true,
		scaleFactor,
		crop: undefined
	});
	processedImage.set(undefined);
	scheduleProcessing(0);
}

function setSourceRecord(record: SourceImageRecord, imageData: ImageData) {
	const oldUrl = sourceObjectUrl.get();
	if (oldUrl) URL.revokeObjectURL(oldUrl);
	sourceMeta.set(sourceMetaFromRecord(record));
	sourceObjectUrl.set(URL.createObjectURL(record.blob));
	sourceImageData.set(imageData);
}

export async function restorePersistedImages() {
	const generation = ++sourceGeneration;
	let source: SourceImageRecord | undefined;
	try {
		source = await loadSourceImage();
	} catch (error) {
		const restoreError = new Error(
			`Could not restore saved source image: ${errorMessage(error, 'record validation failed')}`,
			{ cause: error }
		);
		cancelProcessing();
		clearInMemoryImageState();
		try {
			await clearPersistedImages();
		} catch {
			// Best effort: do not mask the original restore failure.
		}
		throw restoreError;
	}
	if (!source) return;

	let decoded: Awaited<ReturnType<typeof decodeBlob>>;
	try {
		decoded = await decodeBlob(source.blob);
	} catch (error) {
		const restoreError = new Error(
			`Could not restore saved source image: ${errorMessage(error, 'decode failed')}`,
			{ cause: error }
		);
		cancelProcessing();
		clearInMemoryImageState();
		try {
			await clearPersistedImages();
		} catch {
			// Best effort: do not mask the original decode failure.
		}
		throw restoreError;
	}

	if (generation !== sourceGeneration) throw new SourceSuperseded();
	setSourceRecord(source, decoded.imageData);

	try {
		const processed = await loadProcessedImage();
		if (generation !== sourceGeneration) throw new SourceSuperseded();
		if (processed?.settingsHash === currentSettingsHash()) {
			processedImage.set(processed);
			return;
		} else if (processed) await clearPersistedProcessedImage();
	} catch (error) {
		processedImage.set(undefined);
		try {
			await clearPersistedProcessedImage();
		} catch {
			// Best effort: recovery should still schedule fresh processing.
		}
		processingError.set(
			`Could not restore saved output; it will be regenerated. ${errorMessage(error, '')}`.trim()
		);
	}

	scheduleProcessing(0);
}

export async function clearAllImageData() {
	sourceGeneration++;
	cancelProcessing();
	clearInMemoryImageState();
	await clearPersistedImages();
}
