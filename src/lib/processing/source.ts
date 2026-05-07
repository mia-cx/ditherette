import {
	clearInMemoryImageState,
	outputSettings,
	processedImage,
	sourceImageData,
	sourceMeta,
	sourceObjectUrl,
	updateOutputSettings
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
import { ACCEPTED_IMAGE_TYPES, MAX_SOURCE_BYTES, fitOutputSizeToBounds } from './types';
import type { SourceImageRecord } from './types';

function validateSourceFile(file: File) {
	if (!ACCEPTED_IMAGE_TYPES.has(file.type)) {
		throw new Error('Choose a PNG, JPEG, WebP, or GIF image.');
	}
	if (file.size > MAX_SOURCE_BYTES) {
		throw new Error(
			`Image file is too large. Maximum file size is ${Math.round(MAX_SOURCE_BYTES / 1024 / 1024)} MB.`
		);
	}
}

export async function setSourceFile(file: File) {
	validateSourceFile(file);
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
	const settings = outputSettings.get();
	if (settings.autoSizeOnUpload !== false) {
		const size = fitOutputSizeToBounds(decoded.width, decoded.height);
		updateOutputSettings({
			...settings,
			width: size.width,
			height: size.height,
			lockAspect: true,
			crop: undefined
		});
	} else {
		updateOutputSettings({ crop: undefined });
	}
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
