import {
	clearInMemoryImageState,
	outputSettings,
	processedImage,
	processingError,
	sourceImageData,
	sourceMeta,
	sourceObjectUrl,
	sourceWarning,
	updateOutputSettings
} from '$lib/stores/app';
import {
	clearPersistedImages,
	clearPersistedImagesIfSourceMatches,
	clearPersistedProcessedImage,
	loadProcessedImage,
	loadSourceImage,
	saveSourceImageAndClearProcessed,
	sourceMetaFromRecord
} from './db';
import { decodeBlob, type DecodedImage } from './image-decode';
import {
	cancelProcessing,
	currentSettingsHash,
	scheduleProcessing,
	type ProcessingLead
} from './client';
import { clampOutputScale, fitOutputSizeToBounds } from './types';
import { validateSourceBlob } from './image-metadata';
import type { SourceImageRecord } from './types';

let sourceGeneration = 0;
let lastSourceTimestamp = 0;

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

function nextSourceTimestamp() {
	lastSourceTimestamp = Math.max(Date.now(), lastSourceTimestamp + 1);
	return lastSourceTimestamp;
}

async function throwIfSourceSuperseded(generation: number, record?: SourceImageRecord) {
	if (generation === sourceGeneration) return;
	if (record) await clearPersistedImagesIfSourceMatches(record);
	throw new SourceSuperseded();
}

/** Times consecutive source steps as the lead of the processing they schedule. */
function processingLead() {
	const lead: ProcessingLead = { startedAt: performance.now(), timings: [] };
	let stepStartedAt = lead.startedAt;
	const step = (name: string) => {
		const now = performance.now();
		lead.timings.push({ name, ms: now - stepStartedAt });
		stepStartedAt = now;
	};
	return { lead, step };
}

export async function setSourceFile(file: File) {
	const generation = ++sourceGeneration;
	const { lead, step } = processingLead();
	cancelProcessing();
	await validateSourceBlob(file);
	step('main source validation');
	if (generation !== sourceGeneration) throw new SourceSuperseded();
	const decoded = await decodeBlob(file, { validate: false });
	step('main source decode');
	if (generation !== sourceGeneration) throw new SourceSuperseded();
	const record: SourceImageRecord = {
		blob: file,
		name: file.name,
		width: decoded.width,
		height: decoded.height,
		type: file.type,
		updatedAt: nextSourceTimestamp()
	};
	await saveSourceImageAndClearProcessed(record);
	step('main source save');
	await throwIfSourceSuperseded(generation, record);
	setSourceMetadata(record);
	const settings = outputSettings.get();
	const scaleFactor = clampOutputScale(settings.scaleFactor ?? 1, decoded.width, decoded.height);
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
	publishSourceImageData(decoded);
	step('main source publish');
	scheduleProcessing(0, lead);
}

function setSourceMetadata(record: SourceImageRecord) {
	const oldUrl = sourceObjectUrl.get();
	if (oldUrl) URL.revokeObjectURL(oldUrl);
	sourceMeta.set(sourceMetaFromRecord(record));
	sourceObjectUrl.set(URL.createObjectURL(record.blob));
}

function publishSourceImageData({ imageData, warning }: DecodedImage) {
	sourceImageData.set(imageData);
	sourceWarning.set(warning);
}

export async function restorePersistedImages() {
	const generation = ++sourceGeneration;
	const { lead, step } = processingLead();
	cancelProcessing();
	let source: SourceImageRecord | undefined;
	try {
		source = await loadSourceImage();
		step('main source read');
	} catch (error) {
		if (generation !== sourceGeneration) throw new SourceSuperseded();
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
		if (generation !== sourceGeneration) throw new SourceSuperseded();
		throw restoreError;
	}
	if (generation !== sourceGeneration) throw new SourceSuperseded();
	if (!source) return;

	let decoded: DecodedImage;
	try {
		decoded = await decodeBlob(source.blob);
		step('main source decode');
	} catch (error) {
		if (generation !== sourceGeneration) throw new SourceSuperseded();
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
		if (generation !== sourceGeneration) throw new SourceSuperseded();
		throw restoreError;
	}

	if (generation !== sourceGeneration) throw new SourceSuperseded();
	setSourceMetadata(source);

	try {
		const processed = await loadProcessedImage();
		step('main output read');
		if (generation !== sourceGeneration) throw new SourceSuperseded();
		if (processed?.settingsHash === currentSettingsHash()) {
			processedImage.set(processed);
			publishSourceImageData(decoded);
			return;
		} else if (processed) await clearPersistedProcessedImage();
	} catch (error) {
		// Supersession also covers a late DB failure. Never recover by clearing a newer upload.
		if (generation !== sourceGeneration) throw new SourceSuperseded();
		processedImage.set(undefined);
		try {
			await clearPersistedProcessedImage();
		} catch {
			// Best effort: recovery should still schedule fresh processing.
		}
		if (generation !== sourceGeneration) throw new SourceSuperseded();
		processingError.set(
			`Could not restore saved output; it will be regenerated. ${errorMessage(error, '')}`.trim()
		);
	}

	if (generation !== sourceGeneration) throw new SourceSuperseded();
	publishSourceImageData(decoded);
	step('main source publish');
	scheduleProcessing(0, lead);
}

export async function clearAllImageData() {
	sourceGeneration++;
	cancelProcessing();
	clearInMemoryImageState();
	await clearPersistedImages();
}
