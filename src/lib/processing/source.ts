import {
	clearInMemoryImageState,
	processedImage,
	sourceImageData,
	sourceMeta,
	sourceObjectUrl
} from '$lib/stores/app';
import {
	clearPersistedImages,
	decodeBlob,
	loadProcessedImage,
	loadSourceImage,
	saveSourceImage,
	sourceMetaFromRecord
} from './db';
import { scheduleProcessing } from './client';
import type { SourceImageRecord } from './types';

export async function setSourceFile(file: File) {
	if (!file.type.startsWith('image/')) throw new Error('Choose an image file.');
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
	setSourceRecord(record, decoded.imageData);
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
	const decoded = await decodeBlob(source.blob);
	setSourceRecord(source, decoded.imageData);
	const processed = await loadProcessedImage();
	if (processed) processedImage.set(processed);
	scheduleProcessing(0);
}

export async function clearAllImageData() {
	clearInMemoryImageState();
	await clearPersistedImages();
}
