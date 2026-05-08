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
	decodeBlob,
	loadProcessedImage,
	loadSourceImage,
	saveSourceImage,
	sourceMetaFromRecord
} from './db';
import { cancelProcessing, currentSettingsHash, scheduleProcessing } from './client';
import { fitOutputSizeToBounds } from './types';
import { validateSourceBlob } from './image-metadata';
import type { SourceImageRecord } from './types';

const MIN_SCALE_FACTOR = 0.05;

function errorMessage(error: unknown, fallback: string) {
	return error instanceof Error ? error.message : fallback;
}

export async function setSourceFile(file: File) {
	await validateSourceBlob(file);
	cancelProcessing();
	const decoded = await decodeBlob(file);
	const record: SourceImageRecord = {
		blob: file,
		name: file.name,
		width: decoded.width,
		height: decoded.height,
		type: file.type,
		updatedAt: Date.now()
	};
	await saveSourceImage(record);
	await clearPersistedProcessedImage();
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

export function setSourceRecord(record: SourceImageRecord, imageData: ImageData) {
	const oldUrl = sourceObjectUrl.get();
	if (oldUrl) URL.revokeObjectURL(oldUrl);
	sourceMeta.set(sourceMetaFromRecord(record));
	sourceObjectUrl.set(URL.createObjectURL(record.blob));
	sourceImageData.set(imageData);
}

export async function restorePersistedImages() {
	const source = await loadSourceImage();
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

	setSourceRecord(source, decoded.imageData);

	try {
		const processed = await loadProcessedImage();
		if (processed?.settingsHash === currentSettingsHash()) processedImage.set(processed);
		else if (processed) await clearPersistedProcessedImage();
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
	cancelProcessing();
	clearInMemoryImageState();
	await clearPersistedImages();
}
