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
	if (processed?.settingsHash === currentSettingsHash()) processedImage.set(processed);
	else if (processed) await clearPersistedProcessedImage();
	scheduleProcessing(0);
}

export async function clearAllImageData() {
	cancelProcessing();
	clearInMemoryImageState();
	await clearPersistedImages();
}
